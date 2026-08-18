#![allow(clippy::redundant_closure)]
//! SDK-based implementation of the AuthApp trait.
//!
//! Calls tachyon-api REST endpoints via the auto-generated
//! `tachyon-sdk` crate where available, and falls back to raw
//! reqwest for endpoints not yet covered by the SDK.

use std::collections::BTreeMap;
use std::fmt::Debug;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tachyon_sdk::apis::configuration::Configuration;
use tachyon_sdk::auth::{
    self, AuthApp, Identifier, Operator, Policy, PolicyId, PublicApiKey,
    PublicApiKeyId, PublicApiKeyValue, ServiceAccount, ServiceAccountId,
    TenantHierarchy, TenantId, User, UserId,
};

use tachyon_sdk::auth::UserPolicy;
use tachyon_sdk::auth::UserQuery;

const SDK_GET_RETRY_POLICY: SdkGetRetryPolicy = SdkGetRetryPolicy {
    max_attempts: 3,
    per_attempt_timeout: Duration::from_millis(500),
    total_budget: Duration::from_millis(1_500),
    base_delay: Duration::from_millis(50),
    max_jitter: Duration::from_millis(25),
};

#[derive(Clone, Copy)]
struct SdkGetRetryPolicy {
    max_attempts: usize,
    per_attempt_timeout: Duration,
    total_budget: Duration,
    base_delay: Duration,
    max_jitter: Duration,
}

#[derive(Debug, Clone, Copy)]
enum SdkRequestError {
    Transport {
        retryable: bool,
        timeout: bool,
        connect: bool,
        attempts: usize,
    },
    HttpStatus {
        status: reqwest::StatusCode,
        attempts: usize,
    },
    Decode {
        attempts: usize,
    },
}

#[derive(Debug)]
struct SdkRequestFailure {
    error: SdkRequestError,
    correlation_id: Option<String>,
}

impl SdkRequestError {
    fn transport(error: &reqwest::Error) -> Self {
        let timeout = error.is_timeout();
        let connect = error.is_connect();
        Self::Transport {
            retryable: timeout || connect,
            timeout,
            connect,
            attempts: 1,
        }
    }

    fn http_status(status: reqwest::StatusCode) -> Self {
        Self::HttpStatus {
            status,
            attempts: 1,
        }
    }

    fn decode() -> Self {
        Self::Decode { attempts: 1 }
    }

    fn with_attempts(self, attempts: usize) -> Self {
        match self {
            Self::Transport {
                retryable,
                timeout,
                connect,
                ..
            } => Self::Transport {
                retryable,
                timeout,
                connect,
                attempts,
            },
            Self::HttpStatus { status, .. } => {
                Self::HttpStatus { status, attempts }
            }
            Self::Decode { .. } => Self::Decode { attempts },
        }
    }

    fn error_kind(self) -> &'static str {
        match self {
            Self::Transport { .. } => "transport",
            Self::HttpStatus { .. } => "http_status",
            Self::Decode { .. } => "decode",
        }
    }

    fn retryable(self) -> bool {
        matches!(
            self,
            Self::Transport {
                retryable: true,
                ..
            }
        )
    }

    fn timeout(self) -> bool {
        matches!(self, Self::Transport { timeout: true, .. })
    }

    fn connect(self) -> bool {
        matches!(self, Self::Transport { connect: true, .. })
    }

    fn attempts(self) -> usize {
        match self {
            Self::Transport { attempts, .. } => attempts,
            Self::HttpStatus { attempts, .. }
            | Self::Decode { attempts } => attempts,
        }
    }

    fn status(self) -> Option<reqwest::StatusCode> {
        match self {
            Self::HttpStatus { status, .. } => Some(status),
            Self::Transport { .. } | Self::Decode { .. } => None,
        }
    }

    fn into_public_error(self) -> errors::Error {
        match self {
            Self::Transport { .. } => errors::Error::service_unavailable(
                "Upstream dependency unavailable",
            ),
            Self::HttpStatus {
                status: reqwest::StatusCode::UNAUTHORIZED,
                ..
            } => errors::Error::unauthorized(
                "Upstream authentication rejected",
            ),
            Self::HttpStatus {
                status: reqwest::StatusCode::FORBIDDEN,
                ..
            } => {
                errors::Error::forbidden("Upstream authorization rejected")
            }
            Self::HttpStatus {
                status: reqwest::StatusCode::NOT_FOUND,
                ..
            } => errors::Error::not_found("Upstream resource not found"),
            Self::HttpStatus { status, .. }
                if status.is_server_error()
                    || status == reqwest::StatusCode::REQUEST_TIMEOUT
                    || status == reqwest::StatusCode::TOO_MANY_REQUESTS =>
            {
                errors::Error::service_unavailable(
                    "Upstream dependency unavailable",
                )
            }
            Self::HttpStatus { status, .. } if status.is_client_error() => {
                errors::Error::bad_request("Upstream request rejected")
            }
            Self::HttpStatus { .. } | Self::Decode { .. } => {
                errors::Error::internal_server_error(
                    "Upstream protocol error",
                )
            }
        }
    }
}

impl From<SdkRequestError> for errors::Error {
    fn from(error: SdkRequestError) -> Self {
        error.into_public_error()
    }
}

/// AuthApp implementation that delegates to tachyon-api
/// REST endpoints via the tachyon-sdk.
///
/// For user-scoped calls (check_policy, get_user, etc.),
/// the caller's original JWT should be forwarded so that
/// tachyon-api evaluates the correct user's policies.
/// Use `with_caller_token()` to create a request-scoped
/// instance that carries the user's token.
pub struct SdkAuthApp {
    base_url: String,
    /// Default operator ID sent as `x-operator-id` header.
    default_operator_id: String,
    /// Bearer token for authenticating with tachyon-api.
    /// For request-scoped instances, this is the caller's
    /// original JWT. For the base instance, this is a
    /// fallback token (e.g. dummy-token for dev).
    auth_token: String,
}

tokio::task_local! {
    /// Bearer token of the request currently being handled.
    ///
    /// tachyon-api authenticates every operator lookup, but the
    /// organization-resolution path runs deep inside use cases that
    /// have no request context of their own. Carrying the credential
    /// in task-local storage lets those calls authenticate as the
    /// requester without threading a parameter through every use case.
    ///
    /// Set by [`caller_token_scope`] at the HTTP boundary.
    static REQUEST_CALLER_TOKEN: Option<String>;
}

/// Run `future` with `token` recorded as the request's caller credential.
pub async fn caller_token_scope<F>(
    token: Option<String>,
    future: F,
) -> F::Output
where
    F: std::future::Future,
{
    REQUEST_CALLER_TOKEN.scope(token, future).await
}

/// Record the request's `Authorization: Bearer` value for the duration
/// of the request so downstream SDK calls can authenticate as the caller.
pub async fn caller_token_middleware(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let token = request
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .map(str::to_string);

    caller_token_scope(token, next.run(request)).await
}

/// The caller credential of the request currently being handled, if any.
fn request_caller_token() -> Option<String> {
    REQUEST_CALLER_TOKEN
        .try_with(|token| token.clone())
        .ok()
        .flatten()
}

impl Debug for SdkAuthApp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SdkAuthApp")
            .field("base_url", &self.base_url)
            .finish()
    }
}

impl SdkAuthApp {
    pub fn new(
        base_url: impl Into<String>,
        default_operator_id: &TenantId,
        auth_token: impl Into<String>,
    ) -> Self {
        Self {
            base_url: base_url.into(),
            default_operator_id: default_operator_id.as_str().to_string(),
            auth_token: auth_token.into(),
        }
    }

    /// Create a request-scoped instance that uses the
    /// caller's original Bearer token for tachyon-api calls.
    /// This ensures user-scoped operations (check_policy,
    /// get_user, etc.) evaluate the correct user's policies.
    pub fn with_caller_token(&self, token: &str) -> Self {
        Self {
            base_url: self.base_url.clone(),
            default_operator_id: self.default_operator_id.clone(),
            auth_token: token.to_string(),
        }
    }

    // ---- SDK Configuration builders ----

    /// Build an SDK Configuration for public endpoints that
    /// do not require authentication (e.g. verify, oauth-config).
    fn sdk_config_public(&self) -> Configuration {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "x-operator-id",
            self.default_operator_id.parse().unwrap(),
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .unwrap_or_default();

        Configuration {
            base_path: self.base_url.clone(),
            client,
            ..Default::default()
        }
    }

    /// Build an SDK Configuration for public endpoints
    /// scoped to a specific tenant (no Authorization header).
    fn sdk_config_public_for_tenant(
        &self,
        tenant_id: &TenantId,
    ) -> Configuration {
        let mut headers = reqwest::header::HeaderMap::new();
        headers
            .insert("x-operator-id", tenant_id.as_str().parse().unwrap());

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .unwrap_or_default();

        Configuration {
            base_path: self.base_url.clone(),
            client,
            ..Default::default()
        }
    }

    /// Build an SDK Configuration authenticated as the current
    /// request's caller when one is known, falling back to the
    /// process-level credential otherwise.
    fn sdk_config_as_caller(&self) -> Configuration {
        match request_caller_token() {
            Some(token) => self.sdk_config_with_token(&token),
            None => self.sdk_config(),
        }
    }

    /// Build an SDK Configuration using an explicit bearer token.
    fn sdk_config_with_token(&self, token: &str) -> Configuration {
        let mut headers = reqwest::header::HeaderMap::new();
        if let Ok(value) = format!("Bearer {token}").parse() {
            headers.insert("Authorization", value);
        }
        headers.insert(
            "x-operator-id",
            self.default_operator_id.parse().unwrap(),
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .unwrap_or_default();

        Configuration {
            base_path: self.base_url.clone(),
            client,
            ..Default::default()
        }
    }

    /// Build an SDK Configuration with the default operator
    /// header. Used for methods that don't take
    /// executor/multi-tenancy context.
    fn sdk_config(&self) -> Configuration {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "Authorization",
            format!("Bearer {}", self.auth_token).parse().unwrap(),
        );
        headers.insert(
            "x-operator-id",
            self.default_operator_id.parse().unwrap(),
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .unwrap_or_default();

        Configuration {
            base_path: self.base_url.clone(),
            client,
            ..Default::default()
        }
    }

    /// Build an SDK Configuration with auth headers derived
    /// from executor and multi-tenancy context.
    fn sdk_config_with_context(
        &self,
        executor: &dyn tachyon_sdk::auth::ExecutorAction,
        multi_tenancy: &dyn tachyon_sdk::auth::MultiTenancyAction,
    ) -> Configuration {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "Authorization",
            format!("Bearer {}", self.auth_token).parse().unwrap(),
        );

        let resolved_op = multi_tenancy.get_operator_id().ok();

        if let Some(ref op_id) = resolved_op {
            if let Ok(val) = op_id.to_string().parse() {
                headers.insert("x-operator-id", val);
            }
        }

        if let Some(ref platform_id) = multi_tenancy.platform_id() {
            let same_as_operator =
                resolved_op.as_ref() == Some(platform_id);
            if !same_as_operator {
                if let Ok(val) = platform_id.to_string().parse() {
                    headers.insert("x-platform-id", val);
                }
            }
        }

        if let Ok(user_id) = executor.get_user_id() {
            if let Ok(val) = user_id.to_string().parse() {
                headers.insert("x-user-id", val);
            }
        }

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .unwrap_or_default();

        Configuration {
            base_path: self.base_url.clone(),
            client,
            ..Default::default()
        }
    }

    /// Build an SDK Configuration for a specific tenant.
    /// Used by SdkOAuthTokenRepository and get_user_by_id_full.
    fn sdk_config_for_tenant(&self, tenant_id: &TenantId) -> Configuration {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "Authorization",
            format!("Bearer {}", self.auth_token).parse().unwrap(),
        );
        headers.insert(
            "x-operator-id",
            tenant_id.to_string().parse().unwrap(),
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .unwrap_or_default();

        Configuration {
            base_path: self.base_url.clone(),
            client,
            ..Default::default()
        }
    }

    // ---- Raw REST helpers ----

    /// Make a GET request and deserialize the response.
    async fn rest_get<T: serde::de::DeserializeOwned>(
        config: &Configuration,
        path: &str,
    ) -> errors::Result<T> {
        Self::rest_get_typed(config, path).await.map_err(Into::into)
    }

    /// Make a GET request with query params.
    async fn rest_get_query<T: serde::de::DeserializeOwned>(
        config: &Configuration,
        path: &str,
        query: &[(&str, &str)],
    ) -> errors::Result<T> {
        Self::rest_get_query_typed(config, path, query)
            .await
            .map_err(Into::into)
    }

    async fn rest_get_typed<T: serde::de::DeserializeOwned>(
        config: &Configuration,
        path: &str,
    ) -> Result<T, SdkRequestError> {
        Self::rest_get_query_with_policy(
            config,
            path,
            &[],
            SDK_GET_RETRY_POLICY,
        )
        .await
    }

    async fn rest_get_query_typed<T: serde::de::DeserializeOwned>(
        config: &Configuration,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<T, SdkRequestError> {
        Self::rest_get_query_with_policy(
            config,
            path,
            query,
            SDK_GET_RETRY_POLICY,
        )
        .await
    }

    async fn rest_get_query_with_policy<T: serde::de::DeserializeOwned>(
        config: &Configuration,
        path: &str,
        query: &[(&str, &str)],
        policy: SdkGetRetryPolicy,
    ) -> Result<T, SdkRequestError> {
        let started_at = Instant::now();
        let mut last_error = SdkRequestError::Transport {
            retryable: true,
            timeout: true,
            connect: false,
            attempts: 0,
        };

        for attempt in 0..policy.max_attempts.max(1) {
            let Some(remaining_budget) =
                policy.total_budget.checked_sub(started_at.elapsed())
            else {
                return Err(last_error);
            };
            if remaining_budget.is_zero() {
                return Err(last_error);
            }

            let response = config
                .client
                .get(format!("{}{}", config.base_path, path))
                .query(query)
                .timeout(policy.per_attempt_timeout.min(remaining_budget))
                .send()
                .await;

            match response {
                Ok(response) => {
                    return handle_rest_response(response)
                        .await
                        .map_err(|error| error.with_attempts(attempt + 1))
                }
                Err(error) => {
                    last_error = SdkRequestError::transport(&error)
                        .with_attempts(attempt + 1);
                    if !last_error.retryable()
                        || attempt + 1 >= policy.max_attempts.max(1)
                    {
                        return Err(last_error);
                    }
                }
            }

            let exponent =
                u32::try_from(attempt).unwrap_or(u32::MAX).min(8);
            let backoff = policy
                .base_delay
                .saturating_mul(2_u32.saturating_pow(exponent));
            let jitter_millis =
                u64::try_from(policy.max_jitter.as_millis())
                    .unwrap_or(u64::MAX);
            let jitter = if jitter_millis == 0 {
                Duration::ZERO
            } else {
                Duration::from_millis(
                    rand::random::<u64>() % (jitter_millis + 1),
                )
            };
            let delay = backoff.saturating_add(jitter);
            let Some(remaining_budget) =
                policy.total_budget.checked_sub(started_at.elapsed())
            else {
                return Err(last_error);
            };
            if delay >= remaining_budget {
                return Err(last_error);
            }
            tokio::time::sleep(delay).await;
        }

        Err(last_error)
    }

    /// Make a POST request with a JSON body.
    async fn rest_post<B: Serialize, T: serde::de::DeserializeOwned>(
        config: &Configuration,
        path: &str,
        body: &B,
    ) -> errors::Result<T> {
        let resp = config
            .client
            .post(format!("{}{}", config.base_path, path))
            .json(body)
            .send()
            .await
            .map_err(|error| SdkRequestError::transport(&error))?;
        handle_rest_response(resp).await.map_err(Into::into)
    }

    /// Make a POST request while retaining safe downstream diagnostics.
    async fn rest_post_observed<
        B: Serialize,
        T: serde::de::DeserializeOwned,
    >(
        config: &Configuration,
        path: &str,
        body: &B,
    ) -> Result<T, SdkRequestFailure> {
        let resp = config
            .client
            .post(format!("{}{}", config.base_path, path))
            .json(body)
            .send()
            .await
            .map_err(|error| SdkRequestFailure {
                error: SdkRequestError::transport(&error),
                correlation_id: None,
            })?;
        let correlation_id = downstream_correlation_id(resp.headers());
        let status = resp.status();
        if !status.is_success() {
            return Err(SdkRequestFailure {
                error: SdkRequestError::http_status(status),
                correlation_id,
            });
        }
        resp.json::<T>().await.map_err(|_| SdkRequestFailure {
            error: SdkRequestError::decode(),
            correlation_id,
        })
    }

    /// Make a PUT request with a JSON body.
    async fn rest_put<B: Serialize, T: serde::de::DeserializeOwned>(
        config: &Configuration,
        path: &str,
        body: &B,
    ) -> errors::Result<T> {
        let resp = config
            .client
            .put(format!("{}{}", config.base_path, path))
            .json(body)
            .send()
            .await
            .map_err(|error| SdkRequestError::transport(&error))?;
        handle_rest_response(resp).await.map_err(Into::into)
    }

    /// Make a DELETE request.
    async fn rest_delete(
        config: &Configuration,
        path: &str,
    ) -> errors::Result<()> {
        let resp = config
            .client
            .delete(format!("{}{}", config.base_path, path))
            .send()
            .await
            .map_err(|error| SdkRequestError::transport(&error))?;
        if !resp.status().is_success() {
            return Err(SdkRequestError::http_status(resp.status()).into());
        }
        Ok(())
    }

    // ---- OAuth bootstrap (raw reqwest - IaC not in SDK) ----

    /// Fetch OAuth provider configurations from tachyon-api.
    /// Uses public endpoint (no auth required).
    pub async fn fetch_oauth_config(
        &self,
        tenant_id: &TenantId,
    ) -> errors::Result<OAuthBootstrapConfig> {
        let config = self.sdk_config_public();
        let body: OAuthProvidersResp = Self::rest_get_query(
            &config,
            "/v1/iac/oauth-providers",
            &[("tenant_id", tenant_id.as_str())],
        )
        .await?;

        let mut bootstrap = OAuthBootstrapConfig::default();

        for p in body.providers {
            match p.provider.as_str() {
                "github" => {
                    bootstrap.github_credentials = Some(OAuthCredentials {
                        client_id: p.client_id,
                        client_secret: p.client_secret,
                        redirect_uri: p.redirect_uri,
                    });
                }
                "linear" => {
                    bootstrap.linear_credentials = Some(OAuthCredentials {
                        client_id: p.client_id,
                        client_secret: p.client_secret,
                        redirect_uri: p.redirect_uri,
                    });
                    bootstrap.linear_webhook_secret = p.webhook_secret;
                }
                _ => {}
            }
        }

        Ok(bootstrap)
    }

    // ---- Library-specific REST methods (not on AuthApp) ----

    /// Find operators accessible to a user under a platform.
    pub async fn find_operators_by_user(
        &self,
        platform_id: &TenantId,
        user_id: &str,
    ) -> errors::Result<Vec<OperatorResp>> {
        let config = self.sdk_config_public();
        let resp: SdkOperatorsByUserResp = Self::rest_get_query(
            &config,
            "/v1/auth/operators/by-user",
            &[("platform_id", platform_id.as_str()), ("user_id", user_id)],
        )
        .await?;

        let operators = match resp {
            SdkOperatorsByUserResp::Wrapped { operators } => operators,
            SdkOperatorsByUserResp::Bare(operators) => operators,
        };

        Ok(operators.into_iter().map(operator_resp_from_rest).collect())
    }

    /// Get a single operator by ID.
    pub async fn get_operator(
        &self,
        operator_id: &str,
    ) -> errors::Result<Option<OperatorResp>> {
        let config = self.sdk_config();
        let path = format!("/v1/auth/operators/{}", operator_id);
        match Self::rest_get::<SdkOperatorResp>(&config, &path).await {
            Ok(resp) => Ok(Some(operator_resp_from_rest(resp))),
            Err(e) if is_not_found(&e) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Create an operator via REST.
    pub async fn create_operator_rest(
        &self,
        executor: &dyn tachyon_sdk::auth::ExecutorAction,
        multi_tenancy: &dyn tachyon_sdk::auth::MultiTenancyAction,
        req: &CreateOperatorReq,
    ) -> errors::Result<CreateOperatorResp> {
        let config = self.sdk_config_with_context(executor, multi_tenancy);
        let body = serde_json::json!({
            "platformId": req.platform_id,
            "operatorAlias": req.operator_alias,
            "operatorName": req.operator_name,
            "newOperatorOwnerMethod": req.new_operator_owner_method,
            "newOperatorOwnerId": req.new_operator_owner_id,
            "newOperatorOwnerPassword": req.new_operator_owner_password,
        });

        let resp: RestCreateOperatorResp =
            Self::rest_post(&config, "/v1/auth/operators", &body).await?;

        Ok(CreateOperatorResp {
            operator: operator_resp_from_rest(resp.operator),
            owner_id: resp.owner_id,
        })
    }

    /// Invite a user to a tenant via REST.
    pub async fn invite_user_rest(
        &self,
        executor: &dyn tachyon_sdk::auth::ExecutorAction,
        multi_tenancy: &dyn tachyon_sdk::auth::MultiTenancyAction,
        req: &InviteUserReq,
    ) -> errors::Result<User> {
        let config = self.sdk_config_with_context(executor, multi_tenancy);
        let body = serde_json::json!({
            "tenantId": req.tenant_id,
            "platformId": req.platform_id,
            "inviteeId": req.invitee_id,
            "inviteeEmail": req.invitee_email,
            "notifyUser": req.notify_user,
        });

        let resp: RestUserResponse =
            Self::rest_post(&config, "/v1/auth/users/invite", &body)
                .await?;

        user_from_rest_user_response(&resp)
    }

    /// Update a user's role in a specific tenant via REST.
    pub async fn update_user_role(
        &self,
        executor: &dyn tachyon_sdk::auth::ExecutorAction,
        multi_tenancy: &dyn tachyon_sdk::auth::MultiTenancyAction,
        user_id: &str,
        tenant_id: &TenantId,
        role: &str,
    ) -> errors::Result<User> {
        let config = self.sdk_config_with_context(executor, multi_tenancy);
        let body = serde_json::json!({
            "tenantId": tenant_id.as_str(),
            "role": role,
        });

        let path = format!("/v1/auth/users/{}/role", user_id);
        let resp: RestUserResponse =
            Self::rest_put(&config, &path, &body).await?;

        user_from_rest_user_response(&resp)
    }

    /// Get an operator by alias within a platform.
    ///
    /// tachyon-api requires authentication on this endpoint, so the
    /// request's caller credential is forwarded when one is available
    /// (see [`caller_token_scope`]). Anonymous requests fall back to
    /// the process-level credential.
    pub async fn get_operator_by_alias(
        &self,
        platform_id: &TenantId,
        alias: &str,
    ) -> errors::Result<OperatorResp> {
        let config = self.sdk_config_as_caller();
        let resp: SdkOperatorResp = Self::rest_get_query_typed(
            &config,
            "/v1/auth/operators/by-alias",
            &[("platform_id", platform_id.as_str()), ("alias", alias)],
        )
        .await
        .map_err(|error| {
            observe_sdk_request_failure(
                "get_operator_by_alias",
                error,
                None,
            );
            if matches!(
                error,
                SdkRequestError::HttpStatus {
                    status: reqwest::StatusCode::NOT_FOUND,
                    ..
                }
            ) {
                errors::Error::not_found("Operator".to_string())
            } else {
                error.into_public_error()
            }
        })?;

        Ok(operator_resp_from_rest(resp))
    }

    /// Verify a bearer token via SDK.
    /// Uses public endpoint (no auth required).
    pub async fn verify_token(&self, token: &str) -> errors::Result<User> {
        match self.bootstrap_token(token).await {
            Ok(user) => return Ok(user),
            Err(err) => {
                tracing::debug!(
                    error = %err,
                    "bootstrap token verification failed; falling back to legacy verify"
                );
            }
        }

        let config = self.sdk_config_public();
        let req = tachyon_sdk::models::VerifyRequest {
            token: token.to_string(),
        };

        let resp = tachyon_sdk::apis::auth_verify_api::verify(&config, req)
            .await
            .map_err(sdk_api_err)?;

        user_from_sdk_model(&resp.user)
    }

    /// Verify a bearer token with Tachyon's bootstrap endpoint.
    ///
    /// `/v1/me` accepts Tachyon-issued session JWTs as well as
    /// external IdP tokens and returns tenant memberships, which
    /// Library needs before applying repo visibility checks.
    pub async fn bootstrap_token(
        &self,
        token: &str,
    ) -> errors::Result<User> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "Authorization",
            format!("Bearer {token}")
                .parse()
                .map_err(sdk_internal_err)?,
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .map_err(sdk_internal_err)?;
        let config = Configuration {
            base_path: self.base_url.clone(),
            client,
            ..Default::default()
        };

        let resp: BootstrapResponse =
            Self::rest_get(&config, "/v1/me").await?;

        user_from_bootstrap_response(&resp)
    }

    /// Call `/v1/me` using the current `auth_token` and return
    /// the full user with all tenant memberships.
    ///
    /// Unlike `get_user_by_id_full`, this uses no
    /// `x-operator-id` header so tachyon returns memberships
    /// across all platforms, not just the Library platform.
    /// Use this when you need the caller's complete tenant
    /// list (e.g. onboarding wizard seed candidates).
    pub async fn get_caller_user(&self) -> errors::Result<User> {
        self.bootstrap_token(&self.auth_token).await
    }

    /// Verify a public API key via REST.
    pub async fn verify_api_key(
        &self,
        tenant_id: &TenantId,
        api_key: &str,
    ) -> errors::Result<ServiceAccount> {
        let config = self.sdk_config();
        let body = serde_json::json!({
            "tenantId": tenant_id.as_str(),
            "apiKey": api_key,
        });

        let resp: RestVerifyApiKeyResp =
            Self::rest_post(&config, "/v1/auth/api-keys/verify", &body)
                .await?;

        let id: ServiceAccountId =
            resp.service_account_id.parse().map_err(|e| {
                sdk_internal_err(format!("Invalid service account id: {e}"))
            })?;
        let sa_tenant_id = TenantId::new(&resp.tenant_id)?;

        Ok(ServiceAccount {
            id,
            tenant_id: sa_tenant_id,
            name: resp.name.clone(),
            created_at: chrono::Utc::now(),
        })
    }

    /// Sign in with platform via SDK.
    pub async fn sign_in_with_platform(
        &self,
        platform_id: &str,
        access_token: &str,
        allow_sign_up: Option<bool>,
        email: Option<&str>,
        name: Option<&str>,
    ) -> errors::Result<User> {
        let req = tachyon_sdk::models::SignInWithPlatformRequest {
            platform_id: platform_id.to_string(),
            access_token: access_token.to_string(),
            allow_sign_up: Some(allow_sign_up),
            email: Some(email.map(|s| s.to_string())),
            name: Some(name.map(|s| s.to_string())),
        };

        let resp = self.post_sign_in_with_platform(req).await?;

        user_from_sdk_model(&resp.user)
    }

    async fn post_sign_in_with_platform(
        &self,
        req: tachyon_sdk::models::SignInWithPlatformRequest,
    ) -> errors::Result<tachyon_sdk::models::SignInWithPlatformResponse>
    {
        let url =
            format!("{}/auth/v1beta/sign-in-with-platform", self.base_url);
        let resp = reqwest::Client::new()
            .post(url)
            .header(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", self.auth_token),
            )
            .header("x-operator-id", self.default_operator_id.as_str())
            .json(&req)
            .send()
            .await
            .map_err(|error| SdkRequestError::transport(&error))?;

        handle_rest_response(resp).await.map_err(Into::into)
    }

    /// Search user by username via REST.
    pub async fn find_user_by_username(
        &self,
        username: &str,
    ) -> errors::Result<Option<User>> {
        let config = self.sdk_config();
        let path = format!(
            "/v1/auth/users/search/by-username?username={}",
            username
        );
        match Self::rest_get::<RestUserResponse>(&config, &path).await {
            Ok(resp) => user_from_rest_user_response(&resp).map(Some),
            Err(e) if is_not_found(&e) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Find user-policy mappings by resource scope via REST.
    ///
    /// Uses a public (unauthenticated) config because this is a
    /// read-only query that does not require a Bearer token.
    /// The schema-level `auth_token` (`SERVICE_AUTH_TOKEN`) may
    /// be a placeholder like `dummy-token` which production
    /// tachyon-api rejects as an invalid JWT.
    pub async fn find_user_policy_mappings_by_resource_scope(
        &self,
        tenant_id: &TenantId,
        resource_trn: &str,
    ) -> errors::Result<Vec<UserPolicy>> {
        let config = self.sdk_config_public_for_tenant(tenant_id);
        let resp: RestUserPolicyMappingsResp = Self::rest_get_query(
            &config,
            "/v1/auth/user-policy-mappings",
            &[
                ("tenantId", tenant_id.as_str()),
                ("resourceScope", resource_trn),
            ],
        )
        .await?;

        resp.mappings
            .into_iter()
            .map(|m| {
                let assigned_at =
                    chrono::DateTime::parse_from_rfc3339(&m.assigned_at)
                        .map_err(sdk_internal_err)?
                        .with_timezone(&chrono::Utc);

                let policy_id: PolicyId =
                    m.policy_id.parse().map_err(|e| {
                        sdk_internal_err(format!("Invalid policy_id: {e}"))
                    })?;

                let user_id: UserId = m.user_id.parse().map_err(|e| {
                    sdk_internal_err(format!("Invalid user_id: {e}"))
                })?;
                Ok(UserPolicy {
                    user_id,
                    tenant_id: TenantId::new(&m.tenant_id)?,
                    policy_id,
                    resource_scope: m.resource_scope.flatten(),
                    assigned_at,
                })
            })
            .collect()
    }

    /// Get user by ID with tenant list via SDK.
    pub async fn get_user_by_id_full(
        &self,
        operator_id: &TenantId,
        user_id: &str,
    ) -> errors::Result<Option<User>> {
        let config = self.sdk_config_for_tenant(operator_id);
        match tachyon_sdk::apis::auth_users_api::get_user(&config, user_id)
            .await
        {
            Ok(resp) => user_from_sdk_user_response(&resp).map(Some),
            Err(tachyon_sdk::apis::Error::ResponseError(resp))
                if resp.status == reqwest::StatusCode::NOT_FOUND =>
            {
                Ok(None)
            }
            Err(e) => Err(sdk_api_err(e)),
        }
    }
}

// ---- REST response types for endpoints not in SDK ----

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SdkOperatorResp {
    id: String,
    name: String,
    operator_name: String,
    platform_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum SdkOperatorsByUserResp {
    Wrapped { operators: Vec<SdkOperatorResp> },
    Bare(Vec<SdkOperatorResp>),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RestCreateOperatorResp {
    operator: SdkOperatorResp,
    owner_id: String,
}

#[derive(Debug, Deserialize)]
struct RestUserResponse {
    id: String,
    email: Option<String>,
    name: Option<String>,
    role: String,
    #[serde(default)]
    tenants: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct BootstrapResponse {
    user: BootstrapUser,
    tenants: Vec<BootstrapTenant>,
}

#[derive(Debug, Deserialize)]
struct BootstrapUser {
    id: String,
    email: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BootstrapTenant {
    id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RestVerifyApiKeyResp {
    service_account_id: String,
    tenant_id: String,
    name: String,
}

#[derive(Debug, Deserialize)]
struct RestUserPolicyMappingsResp {
    mappings: Vec<RestUserPolicyMapping>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RestUserPolicyMapping {
    user_id: String,
    tenant_id: String,
    policy_id: String,
    resource_scope: Option<Option<String>>,
    assigned_at: String,
}

#[derive(Debug, Deserialize)]
struct RestOAuthTokenListResp {
    tokens: Vec<RestOAuthToken>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RestOAuthToken {
    provider: String,
    access_token: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RestOAuthTokenDetail {
    provider: String,
    provider_user_id: String,
    access_token: String,
    refresh_token: Option<Option<String>>,
    expires_at: String,
}

// ---- Public DTOs (used by callers outside this module) ----

/// Response DTO for operator
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperatorResp {
    pub id: String,
    pub name: String,
    pub operator_name: String,
    pub platform_id: String,
}

/// Request DTO for creating an operator
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateOperatorReq {
    pub platform_id: String,
    pub operator_alias: Option<String>,
    pub operator_name: String,
    pub new_operator_owner_method: String,
    pub new_operator_owner_id: String,
    pub new_operator_owner_password: Option<String>,
}

/// Response DTO for creating an operator
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateOperatorResp {
    pub operator: OperatorResp,
    pub owner_id: String,
}

/// Request DTO for inviting a user
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InviteUserReq {
    pub platform_id: Option<String>,
    pub tenant_id: String,
    pub invitee_id: Option<String>,
    pub invitee_email: Option<String>,
    pub notify_user: Option<bool>,
}

// ---- OAuth bootstrap types (IaC-specific) ----

/// Response from `GET /v1/iac/oauth-providers`
#[derive(Debug, Deserialize)]
struct OAuthProvidersResp {
    providers: Vec<OAuthProviderItemResp>,
}

#[derive(Debug, Deserialize)]
struct OAuthProviderItemResp {
    provider: String,
    client_id: String,
    client_secret: String,
    redirect_uri: String,
    webhook_secret: Option<String>,
}

/// OAuth credentials for a single provider
#[derive(Debug, Clone)]
pub struct OAuthCredentials {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
}

/// Bootstrap configuration fetched via REST.
#[derive(Debug, Clone, Default)]
pub struct OAuthBootstrapConfig {
    pub github_credentials: Option<OAuthCredentials>,
    pub linear_credentials: Option<OAuthCredentials>,
    pub linear_webhook_secret: Option<String>,
}

// ---- Helpers ----

fn sdk_internal_err(msg: impl std::fmt::Display) -> errors::Error {
    errors::Error::internal_server_error(format!(
        "SDK integration error: {msg}"
    ))
}

fn observe_sdk_request_failure(
    operation: &'static str,
    error: SdkRequestError,
    correlation_id: Option<&str>,
) {
    debug_assert!(error.attempts() > 0);
    tracing::warn!(
        operation,
        error_kind = error.error_kind(),
        http_status = error.status().map(|status| status.as_u16()),
        upstream_request_id = correlation_id.unwrap_or(""),
        attempts = error.attempts(),
        retryable = error.retryable(),
        timeout = error.timeout(),
        connect = error.connect(),
        "upstream SDK request failed"
    );

    let mut data: BTreeMap<String, sentry::protocol::Value> = [
        ("operation".to_string(), operation.into()),
        ("error_kind".to_string(), error.error_kind().into()),
        ("attempts".to_string(), error.attempts().into()),
        ("retryable".to_string(), error.retryable().into()),
        ("timeout".to_string(), error.timeout().into()),
        ("connect".to_string(), error.connect().into()),
    ]
    .into_iter()
    .collect();
    if let Some(status) = error.status() {
        data.insert("http_status".to_string(), status.as_u16().into());
    }
    if let Some(correlation_id) = correlation_id {
        data.insert(
            "upstream_request_id".to_string(),
            correlation_id.into(),
        );
    }

    sentry::add_breadcrumb(sentry::Breadcrumb {
        category: Some("sdk_auth".to_string()),
        message: Some("upstream SDK request failed".to_string()),
        level: sentry::Level::Warning,
        data,
        ..Default::default()
    });
}

fn downstream_correlation_id(
    headers: &reqwest::header::HeaderMap,
) -> Option<String> {
    headers
        .get(telemetry::http::REQUEST_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .filter(|value| {
            !value.is_empty()
                && value.len() <= 128
                && value.bytes().all(|byte| {
                    byte.is_ascii_alphanumeric()
                        || matches!(byte, b'-' | b'_' | b':' | b'.')
                })
        })
        .map(ToOwned::to_owned)
}

/// Convert a tachyon-sdk API error into an errors::Error.
fn sdk_api_err<T: std::fmt::Debug>(
    err: tachyon_sdk::apis::Error<T>,
) -> errors::Error {
    match err {
        tachyon_sdk::apis::Error::ResponseError(resp) => {
            SdkRequestError::http_status(resp.status).into_public_error()
        }
        tachyon_sdk::apis::Error::Reqwest(e) => {
            SdkRequestError::transport(&e).into_public_error()
        }
        tachyon_sdk::apis::Error::Serde(e) => {
            let _ = e;
            SdkRequestError::decode().into_public_error()
        }
        tachyon_sdk::apis::Error::Io(e) => {
            let _ = e;
            errors::Error::internal_server_error("Upstream protocol error")
        }
    }
}

/// Handle a REST response: check status and deserialize.
async fn handle_rest_response<T: serde::de::DeserializeOwned>(
    resp: reqwest::Response,
) -> Result<T, SdkRequestError> {
    let status = resp.status();
    if !status.is_success() {
        return Err(SdkRequestError::http_status(status));
    }
    resp.json::<T>()
        .await
        .map_err(|_| SdkRequestError::decode())
}

/// Check if an error is a 404 not-found error.
fn is_not_found(e: &errors::Error) -> bool {
    matches!(e, errors::Error::NotFound { .. })
}

/// Convert REST OperatorResp → local OperatorResp
fn operator_resp_from_rest(resp: SdkOperatorResp) -> OperatorResp {
    OperatorResp {
        id: resp.id,
        name: resp.name,
        operator_name: resp.operator_name,
        platform_id: resp.platform_id,
    }
}

/// Convert REST operator data → domain Operator
fn operator_from_rest(resp: &SdkOperatorResp) -> errors::Result<Operator> {
    let id = TenantId::new(&resp.id)?;
    let platform_id = TenantId::new(&resp.platform_id)?;
    let operator_name: Identifier =
        resp.operator_name.parse().map_err(|e| {
            sdk_internal_err(format!("Invalid operator_name: {e}"))
        })?;
    let now = chrono::Utc::now();
    Ok(Operator {
        id,
        name: resp.name.clone(),
        operator_name,
        platform_id,
        created_at: now,
        updated_at: now,
    })
}

/// Construct an Operator domain object from OperatorResp
pub fn operator_from_resp(resp: &OperatorResp) -> errors::Result<Operator> {
    let id = TenantId::new(&resp.id)?;
    let platform_id = TenantId::new(&resp.platform_id)?;
    let operator_name: Identifier =
        resp.operator_name.parse().map_err(|e| {
            sdk_internal_err(format!("Invalid operator_name: {e}"))
        })?;
    let now = chrono::Utc::now();
    Ok(Operator {
        id,
        name: resp.name.clone(),
        operator_name,
        platform_id,
        created_at: now,
        updated_at: now,
    })
}

/// Construct a User from SDK's `models::User` (no tenants).
fn user_from_sdk_model(
    user: &tachyon_sdk::models::User,
) -> errors::Result<User> {
    let id: UserId = user
        .id
        .parse()
        .map_err(|e| sdk_internal_err(format!("Invalid user id: {e}")))?;
    let username = id.to_string();
    let email: Option<String> =
        user.email.as_ref().and_then(|e| e.as_ref()).cloned();
    let name: Option<String> =
        user.name.as_ref().and_then(|n| n.as_ref()).cloned();
    let role: tachyon_sdk::auth::DefaultRole = user
        .role
        .parse()
        .unwrap_or(tachyon_sdk::auth::DefaultRole::General);

    Ok(User {
        id,
        username,
        email,
        name,
        email_verified: None,
        image: None,
        role,
        tenants: vec![],
        metadata: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    })
}

/// Construct a User from SDK's `models::UserResponse`
/// (with tenants).
fn user_from_sdk_user_response(
    resp: &tachyon_sdk::models::UserResponse,
) -> errors::Result<User> {
    let id: UserId = resp
        .id
        .parse()
        .map_err(|e| sdk_internal_err(format!("Invalid user id: {e}")))?;
    let username = id.to_string();
    let email: Option<String> =
        resp.email.as_ref().and_then(|e| e.as_ref()).cloned();
    let name: Option<String> =
        resp.name.as_ref().and_then(|n| n.as_ref()).cloned();
    let role: tachyon_sdk::auth::DefaultRole = resp
        .role
        .parse()
        .unwrap_or(tachyon_sdk::auth::DefaultRole::General);
    let tenants: Vec<TenantId> = resp
        .tenants
        .iter()
        .map(|t| TenantId::new(t))
        .collect::<errors::Result<Vec<_>>>()?;

    Ok(User {
        id,
        username,
        email,
        name,
        email_verified: None,
        image: None,
        role,
        tenants,
        metadata: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    })
}

/// Construct a User from Tachyon's bootstrap response.
fn user_from_bootstrap_response(
    resp: &BootstrapResponse,
) -> errors::Result<User> {
    let id: UserId =
        resp.user.id.parse().map_err(|e| {
            sdk_internal_err(format!("Invalid user id: {e}"))
        })?;
    let tenants: Vec<TenantId> = resp
        .tenants
        .iter()
        .map(|t| TenantId::new(&t.id))
        .collect::<errors::Result<Vec<_>>>()?;

    Ok(User {
        id: id.clone(),
        username: id.to_string(),
        email: resp.user.email.clone(),
        name: None,
        email_verified: None,
        image: None,
        role: tachyon_sdk::auth::DefaultRole::General,
        tenants,
        metadata: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    })
}

/// Construct a User from REST user response.
fn user_from_rest_user_response(
    resp: &RestUserResponse,
) -> errors::Result<User> {
    let id: UserId = resp
        .id
        .parse()
        .map_err(|e| sdk_internal_err(format!("Invalid user id: {e}")))?;
    let username = id.to_string();
    let email: Option<String> = resp.email.clone();
    let name: Option<String> = resp.name.clone();
    let role: tachyon_sdk::auth::DefaultRole = resp
        .role
        .parse()
        .unwrap_or(tachyon_sdk::auth::DefaultRole::General);
    let tenants: Vec<TenantId> = resp
        .tenants
        .iter()
        .map(|t| TenantId::new(t))
        .collect::<errors::Result<Vec<_>>>()?;

    Ok(User {
        id,
        username,
        email,
        name,
        email_verified: None,
        image: None,
        role,
        tenants,
        metadata: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    })
}

fn service_account_from_sdk(
    resp: &tachyon_sdk::models::ServiceAccountResponse,
) -> errors::Result<ServiceAccount> {
    let id: ServiceAccountId = resp.id.clone().into();
    let tenant_id = TenantId::new(&resp.tenant_id)?;
    let created_at = chrono::DateTime::parse_from_rfc3339(&resp.created_at)
        .map_err(sdk_internal_err)?
        .with_timezone(&chrono::Utc);
    Ok(ServiceAccount {
        id,
        tenant_id,
        name: resp.name.clone(),
        created_at,
    })
}

fn api_key_from_sdk(
    resp: &tachyon_sdk::models::ApiKeyResponse,
    tenant_id: &TenantId,
) -> errors::Result<PublicApiKey> {
    let id: PublicApiKeyId = resp.id.parse().map_err(|e| {
        sdk_internal_err(format!("Invalid api key id: {e}"))
    })?;
    let sa_id: ServiceAccountId = resp.service_account_id.clone().into();
    let value: PublicApiKeyValue = resp.value.parse().map_err(|e| {
        sdk_internal_err(format!("Invalid api key value: {e}"))
    })?;
    let created_at = chrono::DateTime::parse_from_rfc3339(&resp.created_at)
        .map_err(sdk_internal_err)?
        .with_timezone(&chrono::Utc);
    Ok(PublicApiKey {
        id,
        tenant_id: tenant_id.clone(),
        service_account_id: sa_id,
        name: resp.name.clone(),
        value,
        created_at,
    })
}

// ---- AuthApp trait implementation ----

#[async_trait::async_trait]
impl AuthApp for SdkAuthApp {
    async fn check_policy<'a>(
        &self,
        input: &auth::CheckPolicyInput<'a>,
    ) -> errors::Result<()> {
        let config = self
            .sdk_config_with_context(input.executor, input.multi_tenancy);
        let req = tachyon_sdk::models::EvaluatePoliciesBatchRequest {
            actions: vec![input.action.to_string()],
        };

        let resp: tachyon_sdk::models::EvaluatePoliciesBatchResponse =
            Self::rest_post_observed(
                &config,
                "/v1/auth/policies/check",
                &req,
            )
            .await
            .map_err(|failure| {
                observe_sdk_request_failure(
                    "check_policy",
                    failure.error,
                    failure.correlation_id.as_deref(),
                );
                failure.error.into_public_error()
            })?;

        if let Some(result) = resp.results.first() {
            if !result.allowed {
                return Err(errors::Error::forbidden(format!(
                    "action: {}",
                    input.action
                )));
            }
        }

        Ok(())
    }

    async fn evaluate_policies_batch<'a>(
        &self,
        input: &auth::EvaluatePoliciesBatchInput<'a>,
    ) -> errors::Result<Vec<auth::EvaluatePoliciesBatchOutcome>> {
        let config = self
            .sdk_config_with_context(input.executor, input.multi_tenancy);
        let req = tachyon_sdk::models::EvaluatePoliciesBatchRequest {
            actions: input.actions.iter().map(|a| a.to_string()).collect(),
        };

        let resp =
            tachyon_sdk::apis::auth_policies_api::evaluate_policies_batch(
                &config, req,
            )
            .await
            .map_err(sdk_api_err)?;

        Ok(resp
            .results
            .into_iter()
            .map(|o| auth::EvaluatePoliciesBatchOutcome {
                action: o.action,
                allowed: o.allowed,
                error: o.error.flatten(),
            })
            .collect())
    }

    async fn get_tenant_hierarchy<'a>(
        &self,
        _tenant_id: &'a TenantId,
    ) -> errors::Result<TenantHierarchy> {
        Err(sdk_internal_err(
            "get_tenant_hierarchy not supported via SDK",
        ))
    }

    async fn get_user_id_by_user_provider_id<'a>(
        &self,
        _input: &auth::GetUserIdByUserProviderIdInput<'a>,
    ) -> errors::Result<Option<String>> {
        Err(sdk_internal_err(
            "get_user_id_by_user_provider_id not supported",
        ))
    }

    async fn delete_operator<'a>(
        &self,
        _input: &auth::DeleteOperatorInput<'a>,
    ) -> errors::Result<()> {
        Err(sdk_internal_err("delete_operator not supported via SDK"))
    }

    async fn get_operator_by_identifier<'a>(
        &self,
        _input: &auth::GetOperatorByIdentifierInput<'a>,
    ) -> errors::Result<Option<Operator>> {
        Err(sdk_internal_err(
            "get_operator_by_identifier not supported via SDK",
        ))
    }

    async fn get_operator_by_id<'a>(
        &self,
        input: &auth::GetOperatorByIdInput<'a>,
    ) -> errors::Result<Option<Operator>> {
        let config = self
            .sdk_config_with_context(input.executor, input.multi_tenancy);
        let path = format!("/v1/auth/operators/{}", input.operator_id);
        match Self::rest_get::<SdkOperatorResp>(&config, &path).await {
            Ok(resp) => operator_from_rest(&resp).map(Some),
            Err(e) if is_not_found(&e) => Ok(None),
            Err(e) => Err(e),
        }
    }

    async fn create_operator<'a>(
        &self,
        input: &auth::CreateOperatorInput<'a>,
    ) -> errors::Result<Operator> {
        let config = self
            .sdk_config_with_context(input.executor, input.multi_tenancy);
        let body = serde_json::json!({
            "platformId": input.platform_id.to_string(),
            "operatorAlias": input.operator_alias.to_string(),
            "operatorName": input.operator_name.to_string(),
            "newOperatorOwnerMethod": match input.new_operator_owner_method {
                auth::NewOperatorOwnerMethod::Inherit => "Inherit",
                auth::NewOperatorOwnerMethod::Create => "Create",
            },
            "newOperatorOwnerId": input.new_operator_owner_id.to_string(),
            "newOperatorOwnerPassword": input.new_operator_owner_password.map(|s| s.to_string()),
        });

        let resp: RestCreateOperatorResp =
            Self::rest_post(&config, "/v1/auth/operators", &body).await?;

        operator_from_rest(&resp.operator)
    }

    async fn oauth_tokens<'a>(
        &self,
        input: &auth::OAuthTokenInput<'a>,
    ) -> errors::Result<Vec<auth::OAuthToken>> {
        let config = self
            .sdk_config_with_context(input.executor, input.multi_tenancy);
        let resp: RestOAuthTokenListResp =
            Self::rest_get(&config, "/v1/auth/oauth-tokens").await?;

        Ok(resp
            .tokens
            .into_iter()
            .map(|t| auth::OAuthToken {
                provider: t.provider,
                access_token: t.access_token,
            })
            .collect())
    }

    async fn get_oauth_token_by_provider<'a>(
        &self,
        input: &auth::GetOAuthTokenByProviderInput<'a>,
    ) -> errors::Result<Option<auth::OAuthTokenDetail>> {
        let config = self
            .sdk_config_with_context(input.executor, input.multi_tenancy);
        let path = format!("/v1/auth/oauth-tokens/{}", input.provider);
        match Self::rest_get::<RestOAuthTokenDetail>(&config, &path).await {
            Ok(resp) => {
                let expires_at =
                    chrono::DateTime::parse_from_rfc3339(&resp.expires_at)
                        .map_err(sdk_internal_err)?
                        .with_timezone(&chrono::Utc);

                Ok(Some(auth::OAuthTokenDetail {
                    provider: resp.provider,
                    provider_user_id: resp.provider_user_id,
                    access_token: resp.access_token,
                    refresh_token: resp.refresh_token.flatten(),
                    expires_at,
                }))
            }
            Err(e) if is_not_found(&e) => Ok(None),
            Err(e) => Err(e),
        }
    }

    async fn save_oauth_token<'a>(
        &self,
        input: &auth::SaveOAuthTokenInput<'a>,
    ) -> errors::Result<()> {
        let config = self
            .sdk_config_with_context(input.executor, input.multi_tenancy);
        let body = serde_json::json!({
            "provider": input.provider,
            "accessToken": input.access_token,
            "refreshToken": input.refresh_token,
            "expiresIn": input.expires_in,
            "scope": null,
            "providerUserId": input.provider_user_id,
        });

        let _: serde_json::Value =
            Self::rest_post(&config, "/v1/auth/oauth-tokens", &body)
                .await?;

        Ok(())
    }

    async fn delete_oauth_token<'a>(
        &self,
        input: &auth::DeleteOAuthTokenInput<'a>,
    ) -> errors::Result<()> {
        let config = self
            .sdk_config_with_context(input.executor, input.multi_tenancy);
        let path = format!("/v1/auth/oauth-tokens/{}", input.provider);
        Self::rest_delete(&config, &path).await
    }

    async fn create_service_account<'a>(
        &self,
        input: &auth::CreateServiceAccountInput<'a>,
    ) -> errors::Result<ServiceAccount> {
        let config = self
            .sdk_config_with_context(input.executor, input.multi_tenancy);
        let req = tachyon_sdk::models::CreateServiceAccountRequest {
            name: input.name.to_string(),
            tenant_id: input.tenant_id.to_string(),
        };

        let resp =
            tachyon_sdk::apis::auth_service_accounts_api::create_service_account(
                &config, req,
            )
            .await
            .map_err(sdk_api_err)?;

        service_account_from_sdk(&resp)
    }

    async fn update_service_account<'a>(
        &self,
        _input: &auth::UpdateServiceAccountInput<'a>,
    ) -> errors::Result<ServiceAccount> {
        Err(sdk_internal_err(
            "update_service_account not supported via SDK",
        ))
    }

    async fn get_service_account_by_name<'a>(
        &self,
        input: &auth::GetServiceAccountByNameInput<'a>,
    ) -> errors::Result<Option<ServiceAccount>> {
        let config = self
            .sdk_config_with_context(input.executor, input.multi_tenancy);

        let resp =
            tachyon_sdk::apis::auth_service_accounts_api::list_service_accounts(
                &config,
                input.tenant_id.as_ref(),
            )
            .await
            .map_err(sdk_api_err)?;

        for sa in resp.service_accounts {
            if sa.name == input.name {
                return service_account_from_sdk(&sa).map(Some);
            }
        }
        Ok(None)
    }

    async fn delete_service_account<'a>(
        &self,
        input: &auth::DeleteServiceAccountInput<'a>,
    ) -> errors::Result<()> {
        let config = self
            .sdk_config_with_context(input.executor, input.multi_tenancy);

        tachyon_sdk::apis::auth_service_accounts_api::delete_service_account(
            &config,
            input.service_account_id.as_ref(),
        )
        .await
        .map_err(sdk_api_err)?;

        Ok(())
    }

    async fn create_public_api_key<'a>(
        &self,
        input: &auth::CreatePublicApiKeyInput<'a>,
    ) -> errors::Result<PublicApiKey> {
        let config = self
            .sdk_config_with_context(input.executor, input.multi_tenancy);
        let req = tachyon_sdk::models::CreateApiKeyRequest {
            name: input.name.to_string(),
            operator_id: input.operator_id.to_string(),
        };

        let resp = tachyon_sdk::apis::auth_api_keys_api::create_api_key(
            &config,
            input.service_account_id.as_ref(),
            req,
        )
        .await
        .map_err(sdk_api_err)?;

        api_key_from_sdk(&resp, input.operator_id)
    }

    async fn find_all_public_api_key<'a>(
        &self,
        input: &auth::FindAllPublicApiKeyInput<'a>,
    ) -> errors::Result<Vec<PublicApiKey>> {
        let config = self
            .sdk_config_with_context(input.executor, input.multi_tenancy);

        let resp = tachyon_sdk::apis::auth_api_keys_api::list_api_keys(
            &config,
            input.service_account_id.as_ref(),
            input.operator_id.as_ref(),
        )
        .await
        .map_err(sdk_api_err)?;

        resp.api_keys
            .into_iter()
            .map(|k| api_key_from_sdk(&k, input.operator_id))
            .collect()
    }

    async fn attach_user_policy<'a>(
        &self,
        input: &auth::AttachUserPolicyInput<'a>,
    ) -> errors::Result<()> {
        let config = self
            .sdk_config_with_context(input.executor, input.multi_tenancy);
        let body = serde_json::json!({
            "userId": input.user_id.to_string(),
            "policyId": input.policy_id.to_string(),
            "tenantId": input.tenant_id.to_string(),
        });

        let _: serde_json::Value = Self::rest_post(
            &config,
            "/v1/auth/user-policies/attach",
            &body,
        )
        .await?;

        Ok(())
    }

    async fn detach_user_policy<'a>(
        &self,
        input: &auth::DetachUserPolicyInput<'a>,
    ) -> errors::Result<()> {
        let config = self
            .sdk_config_with_context(input.executor, input.multi_tenancy);
        let body = serde_json::json!({
            "userId": input.user_id.to_string(),
            "policyId": input.policy_id.to_string(),
            "tenantId": input.tenant_id.to_string(),
        });

        let _: serde_json::Value = Self::rest_post(
            &config,
            "/v1/auth/user-policies/detach",
            &body,
        )
        .await?;

        Ok(())
    }

    async fn check_policy_for_resource<'a>(
        &self,
        input: &auth::CheckPolicyForResourceInput<'a>,
    ) -> errors::Result<()> {
        let config = self
            .sdk_config_with_context(input.executor, input.multi_tenancy);
        let body = serde_json::json!({
            "action": input.action.to_string(),
            "resourceTrn": input.resource_trn.to_string(),
        });

        #[derive(Deserialize)]
        struct Resp {
            allowed: bool,
        }

        let resp: Resp = Self::rest_post(
            &config,
            "/v1/auth/policies/check-for-resource",
            &body,
        )
        .await
        .map_err(|e| {
            tracing::debug!(
                action = %input.action,
                resource = %input.resource_trn,
                error = %e,
                "check_policy_for_resource failed"
            );
            e
        })?;

        if !resp.allowed {
            return Err(errors::Error::forbidden(format!(
                "action: {} on {}",
                input.action, input.resource_trn
            )));
        }

        Ok(())
    }

    async fn attach_user_policy_with_scope<'a>(
        &self,
        input: &auth::AttachUserPolicyWithScopeInput<'a>,
    ) -> errors::Result<()> {
        let config = self
            .sdk_config_with_context(input.executor, input.multi_tenancy);
        let body = serde_json::json!({
            "userId": input.user_id.to_string(),
            "policyId": input.policy_id.to_string(),
            "tenantId": input.tenant_id.to_string(),
            "resourceScope": input.resource_scope.to_string(),
        });

        let _: serde_json::Value = Self::rest_post(
            &config,
            "/v1/auth/user-policies/attach-with-scope",
            &body,
        )
        .await?;

        Ok(())
    }

    async fn detach_user_policy_with_scope<'a>(
        &self,
        input: &auth::DetachUserPolicyWithScopeInput<'a>,
    ) -> errors::Result<()> {
        let config = self
            .sdk_config_with_context(input.executor, input.multi_tenancy);
        let body = serde_json::json!({
            "userId": input.user_id.to_string(),
            "policyId": input.policy_id.to_string(),
            "tenantId": input.tenant_id.to_string(),
            "resourceScope": input.resource_scope.to_string(),
        });

        let _: serde_json::Value = Self::rest_post(
            &config,
            "/v1/auth/user-policies/detach-with-scope",
            &body,
        )
        .await?;

        Ok(())
    }

    async fn add_user_to_tenant<'a>(
        &self,
        input: &auth::AddUserToTenantInput<'a>,
    ) -> errors::Result<()> {
        let config = self
            .sdk_config_with_context(input.executor, input.multi_tenancy);
        let body = serde_json::json!({
            "tenantId": input.tenant_id.to_string(),
        });

        let path = format!("/v1/auth/users/{}/tenants", input.user_id);
        let _: serde_json::Value =
            Self::rest_post(&config, &path, &body).await?;

        Ok(())
    }

    async fn get_user_by_id<'a>(
        &self,
        input: &auth::GetUserByIdInput<'a>,
    ) -> errors::Result<Option<User>> {
        let config = self
            .sdk_config_with_context(input.executor, input.multi_tenancy);

        match tachyon_sdk::apis::auth_users_api::get_user(
            &config,
            input.user_id.as_ref(),
        )
        .await
        {
            Ok(resp) => user_from_sdk_user_response(&resp).map(Some),
            Err(tachyon_sdk::apis::Error::ResponseError(resp))
                if resp.status == reqwest::StatusCode::NOT_FOUND =>
            {
                Ok(None)
            }
            Err(e) => Err(sdk_api_err(e)),
        }
    }

    async fn find_users_by_tenant<'a>(
        &self,
        input: &auth::FindUsersByTenantInput<'a>,
    ) -> errors::Result<Vec<User>> {
        let config = self
            .sdk_config_with_context(input.executor, input.multi_tenancy);

        let resp = tachyon_sdk::apis::auth_users_api::list_users(
            &config,
            input.tenant_id.as_ref(),
        )
        .await
        .map_err(sdk_api_err)?;

        resp.users.iter().map(user_from_sdk_user_response).collect()
    }

    async fn get_policy_by_id<'a>(
        &self,
        input: &auth::GetPolicyByIdInput<'a>,
    ) -> errors::Result<Option<Policy>> {
        let config = self
            .sdk_config_with_context(input.executor, input.multi_tenancy);

        match tachyon_sdk::apis::auth_policies_api::get_policy(
            &config,
            input.policy_id.as_ref(),
        )
        .await
        {
            Ok(resp) => {
                let tenant_id = resp
                    .tenant_id
                    .flatten()
                    .map(|t| TenantId::new(&t))
                    .transpose()?;

                let created_at =
                    chrono::DateTime::parse_from_rfc3339(&resp.created_at)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now());
                let updated_at =
                    chrono::DateTime::parse_from_rfc3339(&resp.updated_at)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now());

                Ok(Some(Policy {
                    id: PolicyId::from(resp.id),
                    name: resp.name,
                    description: resp.description.flatten(),
                    is_system: resp.is_system,
                    tenant_id,
                    created_at,
                    updated_at,
                }))
            }
            Err(tachyon_sdk::apis::Error::ResponseError(resp))
                if resp.status == reqwest::StatusCode::NOT_FOUND =>
            {
                Ok(None)
            }
            Err(e) => Err(sdk_api_err(e)),
        }
    }

    async fn register_policy<'a>(
        &self,
        _input: &auth::RegisterPolicyInput<'a>,
    ) -> errors::Result<Policy> {
        Err(errors::Error::internal_server_error(
            "register_policy not implemented in SdkAuthApp".to_string(),
        ))
    }

    async fn find_policy_by_name<'a>(
        &self,
        _input: &auth::FindPolicyByNameInput<'a>,
    ) -> errors::Result<Option<Policy>> {
        Err(errors::Error::internal_server_error(
            "find_policy_by_name not implemented in SdkAuthApp".to_string(),
        ))
    }

    async fn attach_sa_policy<'a>(
        &self,
        _input: &auth::AttachSaPolicyInput<'a>,
    ) -> errors::Result<()> {
        Err(errors::Error::internal_server_error(
            "attach_sa_policy not implemented in SdkAuthApp".to_string(),
        ))
    }

    async fn create_oauth2_client<'a>(
        &self,
        _input: &auth::CreateOAuth2ClientInput<'a>,
    ) -> errors::Result<auth::OAuth2ClientCreated> {
        Err(errors::Error::internal_server_error(
            "create_oauth2_client not implemented in SdkAuthApp"
                .to_string(),
        ))
    }

    async fn find_oauth2_client_by_name<'a>(
        &self,
        _input: &auth::FindOAuth2ClientByNameInput<'a>,
    ) -> errors::Result<Option<String>> {
        Err(errors::Error::internal_server_error(
            "find_oauth2_client_by_name not implemented in SdkAuthApp"
                .to_string(),
        ))
    }
}

// ---- REST-backed UserPolicyMappingRepository ----

/// REST-backed implementation of UserPolicyMappingRepository
/// that delegates to tachyon-api endpoints via SDK.
#[derive(Debug)]
pub struct SdkUserPolicyMappingRepository {
    sdk: Arc<SdkAuthApp>,
}

impl SdkUserPolicyMappingRepository {
    pub fn new(sdk: Arc<SdkAuthApp>) -> Self {
        Self { sdk }
    }
}

#[async_trait::async_trait]
impl tachyon_sdk::auth::UserPolicyMappingRepository
    for SdkUserPolicyMappingRepository
{
    async fn create_mapping(
        &self,
        _user_id: &tachyon_sdk::auth::UserId,
        _policy_id: &tachyon_sdk::auth::PolicyId,
        _tenant_id: &TenantId,
    ) -> errors::Result<()> {
        Err(sdk_internal_err("create_mapping: use AuthApp trait"))
    }

    async fn delete_mapping(
        &self,
        _user_id: &tachyon_sdk::auth::UserId,
        _policy_id: &tachyon_sdk::auth::PolicyId,
        _tenant_id: &TenantId,
    ) -> errors::Result<()> {
        Err(sdk_internal_err("delete_mapping: use AuthApp trait"))
    }

    async fn find_policies_by_user(
        &self,
        _user_id: &tachyon_sdk::auth::UserId,
        _tenant_id: &TenantId,
    ) -> errors::Result<Vec<tachyon_sdk::auth::PolicyId>> {
        Err(sdk_internal_err("find_policies_by_user: use AuthApp"))
    }

    async fn find_users_by_policy(
        &self,
        _policy_id: &tachyon_sdk::auth::PolicyId,
        _tenant_id: &TenantId,
    ) -> errors::Result<Vec<tachyon_sdk::auth::UserId>> {
        Err(sdk_internal_err("find_users_by_policy: use AuthApp"))
    }

    async fn exists_mapping(
        &self,
        _user_id: &tachyon_sdk::auth::UserId,
        _policy_id: &tachyon_sdk::auth::PolicyId,
        _tenant_id: &TenantId,
    ) -> errors::Result<bool> {
        Err(sdk_internal_err("exists_mapping: use AuthApp"))
    }

    async fn create_mapping_with_scope(
        &self,
        _user_id: &tachyon_sdk::auth::UserId,
        _policy_id: &tachyon_sdk::auth::PolicyId,
        _tenant_id: &TenantId,
        _resource_scope: &str,
    ) -> errors::Result<()> {
        Err(sdk_internal_err("create_mapping_with_scope: use AuthApp"))
    }

    async fn delete_mapping_with_scope(
        &self,
        _user_id: &tachyon_sdk::auth::UserId,
        _policy_id: &tachyon_sdk::auth::PolicyId,
        _tenant_id: &TenantId,
        _resource_scope: &str,
    ) -> errors::Result<()> {
        Err(sdk_internal_err("delete_mapping_with_scope: use AuthApp"))
    }

    async fn find_by_resource_scope(
        &self,
        tenant_id: &TenantId,
        resource_trn: &str,
    ) -> errors::Result<Vec<UserPolicy>> {
        self.sdk
            .find_user_policy_mappings_by_resource_scope(
                tenant_id,
                resource_trn,
            )
            .await
    }
}

// ---- REST-backed UserQuery ----

/// REST-backed implementation of UserQuery
#[derive(Debug)]
pub struct SdkUserQuery {
    sdk: Arc<SdkAuthApp>,
}

impl SdkUserQuery {
    pub fn new(sdk: Arc<SdkAuthApp>) -> Self {
        Self { sdk }
    }
}

#[async_trait::async_trait]
impl UserQuery for SdkUserQuery {
    async fn find_by_id(
        &self,
        id: &UserId,
    ) -> errors::Result<Option<User>> {
        // Delegate to get_by_user_id with a dummy tenant
        // (SDK user lookup doesn't require tenant context)
        self.sdk
            .get_user_by_id_full(
                &TenantId::new("tn_00000000000000000000000000")?,
                id.as_str(),
            )
            .await
    }

    async fn find_by_tenant(
        &self,
        _tenant_id: &TenantId,
    ) -> errors::Result<Vec<User>> {
        // Not used by library-api
        Ok(vec![])
    }

    async fn get_by_user_id(
        &self,
        tenant_id: &TenantId,
        id: &str,
    ) -> errors::Result<Option<User>> {
        self.sdk.get_user_by_id_full(tenant_id, id).await
    }

    async fn get_by_email(
        &self,
        _tenant_id: &TenantId,
        _email: &str,
    ) -> errors::Result<Option<User>> {
        // Not used by library-api
        Ok(None)
    }

    async fn get_by_username(
        &self,
        username: &value_object::Username,
    ) -> errors::Result<Option<User>> {
        self.sdk.find_user_by_username(username.value()).await
    }

    async fn search_by_username_prefix(
        &self,
        _prefix: &str,
        _limit: u32,
    ) -> errors::Result<Vec<User>> {
        // Not used by library-api
        Ok(vec![])
    }
}

// ---- SDK-backed OAuthTokenRepository ----

/// SDK-backed implementation of StoredOAuthTokenRepository.
#[derive(Debug)]
pub struct SdkOAuthTokenRepository {
    sdk: Arc<SdkAuthApp>,
}

impl SdkOAuthTokenRepository {
    pub fn new(sdk: Arc<SdkAuthApp>) -> Self {
        Self { sdk }
    }
}

#[async_trait::async_trait]
impl inbound_sync_domain::OAuthTokenRepository for SdkOAuthTokenRepository {
    async fn save(
        &self,
        token: &inbound_sync_domain::StoredOAuthToken,
    ) -> errors::Result<()> {
        let expires_in = token
            .expires_at
            .map(|exp| (exp - chrono::Utc::now()).num_seconds().max(0))
            .unwrap_or(3600);

        let scope = if token.scopes.is_empty() {
            None
        } else {
            Some(token.scopes.join(" "))
        };

        let sdk_tenant_id = TenantId::new(token.tenant_id.as_ref())?;
        let config = self.sdk.sdk_config_for_tenant(&sdk_tenant_id);
        let body = serde_json::json!({
            "provider": token.provider.to_string(),
            "accessToken": token.access_token,
            "refreshToken": token.refresh_token,
            "expiresIn": expires_in,
            "scope": scope,
            "providerUserId": token.external_account_id
                .as_deref()
                .unwrap_or("unknown"),
        });

        let _: serde_json::Value =
            SdkAuthApp::rest_post(&config, "/v1/auth/oauth-tokens", &body)
                .await?;

        Ok(())
    }

    async fn find_by_tenant_and_provider(
        &self,
        tenant_id: &value_object::TenantId,
        provider: inbound_sync_domain::OAuthProvider,
    ) -> errors::Result<Option<inbound_sync_domain::StoredOAuthToken>> {
        let sdk_tenant_id = TenantId::new(tenant_id.as_ref())?;
        let config = self.sdk.sdk_config_for_tenant(&sdk_tenant_id);
        let path = format!("/v1/auth/oauth-tokens/{}", provider);
        match SdkAuthApp::rest_get::<RestOAuthTokenDetail>(&config, &path)
            .await
        {
            Ok(detail) => {
                let expires_at = chrono::DateTime::parse_from_rfc3339(
                    &detail.expires_at,
                )
                .ok()
                .map(|dt| dt.with_timezone(&chrono::Utc));

                Ok(Some(inbound_sync_domain::StoredOAuthToken {
                    id: String::new(),
                    tenant_id: tenant_id.clone(),
                    provider,
                    access_token: detail.access_token,
                    refresh_token: detail.refresh_token.flatten(),
                    token_type: "Bearer".to_string(),
                    expires_at,
                    scopes: vec![],
                    external_account_id: Some(detail.provider_user_id),
                    external_account_name: None,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                }))
            }
            Err(e) if is_not_found(&e) => Ok(None),
            Err(e) => Err(e),
        }
    }

    async fn delete(
        &self,
        tenant_id: &value_object::TenantId,
        provider: inbound_sync_domain::OAuthProvider,
    ) -> errors::Result<()> {
        let sdk_tenant_id = TenantId::new(tenant_id.as_ref())?;
        let config = self.sdk.sdk_config_for_tenant(&sdk_tenant_id);
        let path = format!("/v1/auth/oauth-tokens/{}", provider);
        SdkAuthApp::rest_delete(&config, &path).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_graphql::ErrorExtensions;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tracing_subscriber::prelude::*;

    const TEST_TENANT_ID: &str = "tn_01j702qf86pc2j35s0kv0gv3gy";

    fn public_error_class(error: &errors::Error) -> &'static str {
        match error {
            errors::Error::Unauthorized { .. } => "unauthorized",
            errors::Error::Forbidden { .. } => "forbidden",
            errors::Error::NotFound { .. } => "not_found",
            errors::Error::BadRequest { .. } => "bad_request",
            errors::Error::ServiceUnavailable { .. } => {
                "service_unavailable"
            }
            errors::Error::InternalServerError { .. } => "internal",
            errors::Error::Conflict { .. }
            | errors::Error::PaymentRequired { .. } => "other",
        }
    }

    fn test_config(base_url: String) -> Configuration {
        Configuration {
            base_path: base_url,
            client: reqwest::Client::new(),
            ..Default::default()
        }
    }

    fn fast_retry_policy() -> SdkGetRetryPolicy {
        SdkGetRetryPolicy {
            max_attempts: 3,
            per_attempt_timeout: Duration::from_millis(20),
            total_budget: Duration::from_millis(150),
            base_delay: Duration::from_millis(2),
            max_jitter: Duration::ZERO,
        }
    }

    async fn single_response_case(
        status: &str,
        body: &str,
    ) -> SdkRequestError {
        let listener =
            tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let response = format!(
            "HTTP/1.1 {status}\r\ncontent-type: application/json\r\nconnection: close\r\ncontent-length: {}\r\n\r\n{body}",
            body.len()
        );
        let server = tokio::spawn(async move {
            let (mut socket, _) = tokio::time::timeout(
                Duration::from_secs(1),
                listener.accept(),
            )
            .await
            .unwrap()
            .unwrap();
            let mut request = vec![0_u8; 4096];
            let _ = socket.read(&mut request).await;
            socket.write_all(response.as_bytes()).await.unwrap();
        });

        let config = test_config(format!("http://{addr}"));
        let error =
            SdkAuthApp::rest_get_query_with_policy::<SdkOperatorResp>(
                &config,
                "/v1/auth/operators/by-alias",
                &[("platform_id", TEST_TENANT_ID), ("alias", "safe-alias")],
                fast_retry_policy(),
            )
            .await
            .unwrap_err();
        server.await.unwrap();

        error
    }

    async fn policy_check_response_case(
        status: &str,
        body: &str,
        request_id: Option<&str>,
    ) -> errors::Result<()> {
        let listener =
            tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let request_id_header = request_id
            .map(|request_id| format!("x-request-id: {request_id}\r\n"))
            .unwrap_or_default();
        let response = format!(
            "HTTP/1.1 {status}\r\ncontent-type: application/json\r\n{request_id_header}connection: close\r\ncontent-length: {}\r\n\r\n{body}",
            body.len()
        );
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = vec![0_u8; 4096];
            let n = socket.read(&mut request).await.unwrap();
            let request = String::from_utf8_lossy(&request[..n]);
            assert!(request.contains("POST /v1/auth/policies/check "));
            assert!(request.contains("authorization: Bearer caller-token"));
            assert!(request
                .contains("x-operator-id: tn_01j702qf86pc2j35s0kv0gv3gz"));
            assert!(request.contains("library:CreateRepo"));
            socket.write_all(response.as_bytes()).await.unwrap();
        });

        let default_tenant: TenantId = TEST_TENANT_ID.parse().unwrap();
        let operator: TenantId =
            "tn_01j702qf86pc2j35s0kv0gv3gz".parse().unwrap();
        let sdk = SdkAuthApp::new(
            format!("http://{addr}"),
            &default_tenant,
            "caller-token",
        );
        let executor = tachyon_sdk::auth::Executor::SystemUser;
        let multi_tenancy =
            tachyon_sdk::auth::MultiTenancy::new(None, Some(operator));
        let result = sdk
            .check_policy(&tachyon_sdk::auth::CheckPolicyInput {
                executor: &executor,
                multi_tenancy: &multi_tenancy,
                action: "library:CreateRepo",
            })
            .await;
        server.await.unwrap();
        result
    }

    #[test]
    fn sdk_request_error_mapping_separates_auth_and_dependency_failures() {
        let cases = [
            (
                "401",
                SdkRequestError::http_status(
                    reqwest::StatusCode::UNAUTHORIZED,
                ),
                "http_status",
                "unauthorized",
                true,
            ),
            (
                "403",
                SdkRequestError::http_status(
                    reqwest::StatusCode::FORBIDDEN,
                ),
                "http_status",
                "forbidden",
                true,
            ),
            (
                "404",
                SdkRequestError::http_status(
                    reqwest::StatusCode::NOT_FOUND,
                ),
                "http_status",
                "not_found",
                false,
            ),
            (
                "5xx",
                SdkRequestError::http_status(
                    reqwest::StatusCode::BAD_GATEWAY,
                ),
                "http_status",
                "service_unavailable",
                false,
            ),
            (
                "decode",
                SdkRequestError::decode(),
                "decode",
                "internal",
                false,
            ),
            (
                "timeout",
                SdkRequestError::Transport {
                    retryable: true,
                    timeout: true,
                    connect: false,
                    attempts: 1,
                },
                "transport",
                "service_unavailable",
                false,
            ),
            (
                "connect",
                SdkRequestError::Transport {
                    retryable: true,
                    timeout: false,
                    connect: true,
                    attempts: 1,
                },
                "transport",
                "service_unavailable",
                false,
            ),
        ];

        for (name, sdk_error, error_kind, public_class, is_auth) in cases {
            assert_eq!(sdk_error.error_kind(), error_kind, "case={name}");
            let public_error = sdk_error.into_public_error();
            assert_eq!(
                public_error_class(&public_error),
                public_class,
                "case={name}"
            );
            assert_eq!(
                matches!(
                    public_error,
                    errors::Error::Unauthorized { .. }
                        | errors::Error::Forbidden { .. }
                ),
                is_auth,
                "case={name}"
            );
            assert!(
                !public_error.to_string().contains("SDK auth error"),
                "case={name}"
            );
        }
    }

    #[tokio::test]
    async fn status_and_decode_failures_are_not_retried() {
        let cases = [
            ("401 Unauthorized", "{}", "unauthorized"),
            ("403 Forbidden", "{}", "forbidden"),
            ("404 Not Found", "{}", "not_found"),
            ("502 Bad Gateway", "{}", "service_unavailable"),
            ("200 OK", "not-json", "internal"),
        ];

        for (status, body, expected_class) in cases {
            let sdk_error = single_response_case(status, body).await;
            assert_eq!(sdk_error.attempts(), 1, "status={status}");
            assert_eq!(
                public_error_class(&sdk_error.into_public_error()),
                expected_class,
                "status={status}"
            );
        }
    }

    #[tokio::test]
    async fn check_policy_distinguishes_deny_from_upstream_failures() {
        let deny = policy_check_response_case(
            "200 OK",
            r#"{"results":[{"action":"library:CreateRepo","allowed":false}]}"#,
            Some("auth-deny-request"),
        )
        .await
        .unwrap_err();
        assert!(matches!(deny, errors::Error::Forbidden { .. }));
        assert!(deny.to_string().contains("library:CreateRepo"));

        let authentication_failure = policy_check_response_case(
            "401 Unauthorized",
            "{}",
            Some("auth-401-request"),
        )
        .await
        .unwrap_err();
        assert!(matches!(
            authentication_failure,
            errors::Error::Unauthorized { .. }
        ));
        assert!(!authentication_failure
            .to_string()
            .contains("library:CreateRepo"));

        let listener =
            tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let closed_addr = listener.local_addr().unwrap();
        drop(listener);
        let tenant: TenantId = TEST_TENANT_ID.parse().unwrap();
        let sdk = SdkAuthApp::new(
            format!("http://{closed_addr}"),
            &tenant,
            "caller-token",
        );
        let transport_failure = sdk
            .check_policy(&tachyon_sdk::auth::CheckPolicyInput {
                executor: &tachyon_sdk::auth::Executor::SystemUser,
                multi_tenancy: &tachyon_sdk::auth::MultiTenancy::default(),
                action: "library:CreateRepo",
            })
            .await
            .unwrap_err();
        assert!(matches!(
            transport_failure,
            errors::Error::ServiceUnavailable { .. }
        ));
    }

    #[test]
    fn check_policy_breadcrumb_keeps_status_and_correlation() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let events = sentry::test::with_captured_events(|| {
            let error = runtime.block_on(policy_check_response_case(
                "401 Unauthorized",
                "{}",
                Some("auth-correlation-401"),
            ));
            assert!(matches!(
                error,
                Err(errors::Error::Unauthorized { .. })
            ));
            sentry::capture_message(
                "check policy observability test",
                sentry::Level::Error,
            );
        });

        let breadcrumb = events[0]
            .breadcrumbs
            .iter()
            .find(|breadcrumb| {
                breadcrumb.category.as_deref() == Some("sdk_auth")
                    && breadcrumb
                        .data
                        .get("operation")
                        .and_then(|value| value.as_str())
                        == Some("check_policy")
            })
            .expect("check_policy breadcrumb must be captured");
        assert_eq!(
            breadcrumb
                .data
                .get("http_status")
                .and_then(|value| value.as_u64()),
            Some(401)
        );
        assert_eq!(
            breadcrumb
                .data
                .get("upstream_request_id")
                .and_then(|value| value.as_str()),
            Some("auth-correlation-401")
        );
    }

    #[tokio::test]
    async fn timeout_and_connect_send_failures_are_bounded_and_retryable() {
        let listener =
            tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let timeout_addr = listener.local_addr().unwrap();
        let timeout_attempts = Arc::new(AtomicUsize::new(0));
        let server_attempts = timeout_attempts.clone();
        let server = tokio::spawn(async move {
            let mut sockets = Vec::new();
            for _ in 0..3 {
                let (socket, _) = tokio::time::timeout(
                    Duration::from_secs(1),
                    listener.accept(),
                )
                .await
                .unwrap()
                .unwrap();
                server_attempts.fetch_add(1, Ordering::SeqCst);
                sockets.push(socket);
            }
            tokio::time::sleep(Duration::from_millis(40)).await;
        });

        let timeout_error =
            SdkAuthApp::rest_get_query_with_policy::<SdkOperatorResp>(
                &test_config(format!("http://{timeout_addr}")),
                "/v1/auth/operators/by-alias",
                &[],
                fast_retry_policy(),
            )
            .await
            .unwrap_err();
        server.await.unwrap();
        assert_eq!(timeout_attempts.load(Ordering::SeqCst), 3);
        assert!(timeout_error.retryable());
        assert!(timeout_error.timeout());
        assert!(!timeout_error.connect());
        assert_eq!(timeout_error.attempts(), 3);

        let closed_listener =
            tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let closed_addr = closed_listener.local_addr().unwrap();
        drop(closed_listener);
        let connect_error =
            SdkAuthApp::rest_get_query_with_policy::<SdkOperatorResp>(
                &test_config(format!("http://{closed_addr}")),
                "/v1/auth/operators/by-alias",
                &[],
                fast_retry_policy(),
            )
            .await
            .unwrap_err();
        assert!(connect_error.retryable());
        assert!(connect_error.connect());
        assert!(!connect_error.timeout());
        assert_eq!(connect_error.attempts(), 3);
    }

    #[test]
    fn transport_failure_is_redacted_and_captured_once() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let mut rendered_error = String::new();
        let mut redacted_inputs = Vec::new();

        let events = sentry::test::with_captured_events(|| {
            let subscriber = tracing_subscriber::registry().with(
                sentry_tracing::layer().event_filter(|metadata| {
                    if metadata.level() == &tracing::Level::ERROR {
                        sentry_tracing::EventFilter::Event
                    } else {
                        sentry_tracing::EventFilter::Ignore
                    }
                }),
            );
            tracing::subscriber::with_default(subscriber, || {
                runtime.block_on(async {
                    let listener =
                        tokio::net::TcpListener::bind("127.0.0.1:0")
                            .await
                            .unwrap();
                    let closed_addr = listener.local_addr().unwrap();
                    drop(listener);

                    let base_url = format!("http://{closed_addr}");
                    let alias = "query-identifier-marker";
                    let credential = "credential-marker";
                    let tenant_id: TenantId =
                        TEST_TENANT_ID.parse().unwrap();
                    let sdk = SdkAuthApp::new(
                        base_url.clone(),
                        &tenant_id,
                        credential,
                    );
                    let error = sdk
                        .get_operator_by_alias(&tenant_id, alias)
                        .await
                        .unwrap_err();

                    assert!(matches!(
                        error,
                        errors::Error::ServiceUnavailable { .. }
                    ));
                    assert!(!matches!(
                        error,
                        errors::Error::Unauthorized { .. }
                            | errors::Error::Forbidden { .. }
                    ));

                    rendered_error = format!("{error:?}\n{error}");
                    redacted_inputs = vec![
                        base_url,
                        closed_addr.to_string(),
                        alias.to_string(),
                        tenant_id.as_str().to_string(),
                        credential.to_string(),
                    ];

                    crate::handler::graphql::log_graphql_operation_error(
                        "library_query",
                        &error,
                    );
                    let _ = error.extend();
                });
            });
        });

        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0].message.as_deref(),
            Some("ServiceUnavailable: Upstream dependency unavailable")
        );

        let sdk_breadcrumb = events[0]
            .breadcrumbs
            .iter()
            .find(|breadcrumb| {
                breadcrumb.category.as_deref() == Some("sdk_auth")
            })
            .expect(
                "sdk_auth breadcrumb must be attached to the error event",
            );
        assert_eq!(
            sdk_breadcrumb
                .data
                .get("operation")
                .and_then(|value| value.as_str()),
            Some("get_operator_by_alias")
        );
        assert_eq!(
            sdk_breadcrumb
                .data
                .get("error_kind")
                .and_then(|value| value.as_str()),
            Some("transport")
        );
        assert_eq!(
            sdk_breadcrumb
                .data
                .get("retryable")
                .and_then(|value| value.as_bool()),
            Some(true)
        );
        assert_eq!(
            sdk_breadcrumb
                .data
                .get("connect")
                .and_then(|value| value.as_bool()),
            Some(true)
        );

        let sentry_payload = format!("{:#?}", events[0]);
        for sensitive in redacted_inputs {
            assert!(!rendered_error.contains(&sensitive));
            assert!(!sentry_payload.contains(&sensitive));
        }
    }

    #[tokio::test]
    async fn sign_in_with_platform_sends_operator_header_directly(
    ) -> anyhow::Result<()> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;

        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0_u8; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let request = String::from_utf8_lossy(&buf[..n]).to_string();

            assert!(
                request.contains(
                    "x-operator-id: tn_01j702qf86pc2j35s0kv0gv3gy"
                ),
                "request was missing x-operator-id header: {request}"
            );

            let body = serde_json::json!({
                "user": {
                    "id": "us_01hs2yepy5hw4rz8pdq2wywnwt",
                    "role": "GENERAL",
                    "tenants": ["tn_01j702qf86pc2j35s0kv0gv3gy"]
                }
            })
            .to_string();
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{}",
                body.len(),
                body
            );
            socket.write_all(response.as_bytes()).await.unwrap();
        });

        let tenant_id: TenantId =
            "tn_01j702qf86pc2j35s0kv0gv3gy".parse()?;
        let sdk = SdkAuthApp::new(
            format!("http://{addr}"),
            &tenant_id,
            "pk_test_service_token",
        );

        let user = sdk
            .sign_in_with_platform(
                tenant_id.as_str(),
                "access-token",
                Some(true),
                None,
                None,
            )
            .await?;

        assert_eq!(user.id().as_str(), "us_01hs2yepy5hw4rz8pdq2wywnwt");
        server.await?;

        Ok(())
    }

    #[tokio::test]
    async fn find_operators_by_user_uses_public_auth_endpoint(
    ) -> anyhow::Result<()> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;

        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0_u8; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let request = String::from_utf8_lossy(&buf[..n]).to_string();

            assert!(
                request.contains("GET /v1/auth/operators/by-user?"),
                "request did not call operators by-user endpoint: {request}"
            );
            assert!(
                request.contains(
                    "x-operator-id: tn_01j702qf86pc2j35s0kv0gv3gy"
                ),
                "request was missing x-operator-id header: {request}"
            );
            assert!(
                !request.to_ascii_lowercase().contains("authorization:"),
                "public operator lookup must not send Authorization: {request}"
            );

            let body = serde_json::json!({
                "operators": [{
                    "id": "tn_01j702qf86pc2j35s0kv0gv3gy",
                    "name": "Library",
                    "operatorName": "library",
                    "platformId": "tn_01j702qf86pc2j35s0kv0gv3gy"
                }]
            })
            .to_string();
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{}",
                body.len(),
                body
            );
            socket.write_all(response.as_bytes()).await.unwrap();
        });

        let tenant_id: TenantId =
            "tn_01j702qf86pc2j35s0kv0gv3gy".parse()?;
        let sdk = SdkAuthApp::new(
            format!("http://{addr}"),
            &tenant_id,
            "pk_test_service_token",
        );

        let operators = sdk
            .find_operators_by_user(
                &tenant_id,
                "us_01hs2yepy5hw4rz8pdq2wywnwt",
            )
            .await?;

        assert_eq!(operators.len(), 1);
        assert_eq!(operators[0].id, tenant_id.as_str());
        server.await?;

        Ok(())
    }
}

#[cfg(test)]
mod caller_token_scope_tests {
    use super::*;
    use tachyon_sdk::auth::test_helper::TEST_TENANT_ID;

    #[tokio::test]
    async fn forwards_the_request_caller_token() {
        let observed =
            caller_token_scope(Some("caller-jwt".to_string()), async {
                request_caller_token()
            })
            .await;

        assert_eq!(observed.as_deref(), Some("caller-jwt"));
    }

    #[tokio::test]
    async fn reports_no_token_for_anonymous_requests() {
        let observed =
            caller_token_scope(None, async { request_caller_token() })
                .await;

        assert_eq!(observed, None);
    }

    #[tokio::test]
    async fn reports_no_token_outside_a_request() {
        assert_eq!(request_caller_token(), None);
    }

    #[tokio::test]
    async fn operator_lookup_sends_the_caller_token() {
        let requests = Arc::new(std::sync::Mutex::new(Vec::new()));
        let captured = requests.clone();

        let app = axum::Router::new().route(
            "/v1/auth/operators/by-alias",
            axum::routing::get(move |headers: axum::http::HeaderMap| {
                let captured = captured.clone();
                async move {
                    let auth = headers
                        .get(axum::http::header::AUTHORIZATION)
                        .and_then(|value| value.to_str().ok())
                        .map(str::to_string);
                    captured.lock().unwrap().push(auth);
                    axum::http::StatusCode::NOT_FOUND
                }
            }),
        );

        let listener =
            tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        let tenant_id: TenantId = TEST_TENANT_ID.parse().unwrap();
        let sdk = SdkAuthApp::new(
            format!("http://{addr}"),
            &tenant_id,
            "process-level-token",
        );

        let _ = caller_token_scope(Some("caller-jwt".to_string()), async {
            sdk.get_operator_by_alias(&tenant_id, "some-org").await
        })
        .await;

        let seen = requests.lock().unwrap().clone();
        assert_eq!(seen, vec![Some("Bearer caller-jwt".to_string())]);
    }
}
