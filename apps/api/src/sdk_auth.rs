//! SDK-based implementation of the AuthApp trait.
//!
//! Calls tachyon-api REST endpoints via the auto-generated
//! `tachyon-sdk` crate where available, and falls back to raw
//! reqwest for endpoints not yet covered by the SDK.

use std::fmt::Debug;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tachyon_sdk::auth::{
    self, AuthApp, Identifier, Operator, Policy,
    PolicyId, PublicApiKey, PublicApiKeyId, PublicApiKeyValue,
    ServiceAccount, ServiceAccountId, TenantHierarchy, TenantId, User,
    UserId,
};
use tachyon_sdk::apis::configuration::Configuration;

use tachyon_sdk::auth::UserPolicy;
use tachyon_sdk::auth::UserQuery;

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
        headers.insert(
            "x-operator-id",
            tenant_id.as_str().parse().unwrap(),
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
                resolved_op.as_ref().map_or(false, |op| op == platform_id);
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
        let resp = config
            .client
            .get(format!("{}{}", config.base_path, path))
            .send()
            .await
            .map_err(|e| sdk_err(&e))?;
        handle_rest_response(resp).await
    }

    /// Make a GET request with query params.
    async fn rest_get_query<T: serde::de::DeserializeOwned>(
        config: &Configuration,
        path: &str,
        query: &[(&str, &str)],
    ) -> errors::Result<T> {
        let resp = config
            .client
            .get(format!("{}{}", config.base_path, path))
            .query(query)
            .send()
            .await
            .map_err(|e| sdk_err(&e))?;
        handle_rest_response(resp).await
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
            .map_err(|e| sdk_err(&e))?;
        handle_rest_response(resp).await
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
            .map_err(|e| sdk_err(&e))?;
        handle_rest_response(resp).await
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
            .map_err(|e| sdk_err(&e))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(map_http_error(status, &body));
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
        let resp = config
            .client
            .get(format!("{}/v1/iac/oauth-providers", self.base_url))
            .query(&[("tenant_id", tenant_id.as_str())])
            .send()
            .await
            .map_err(|e| sdk_err(&e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(sdk_err(format!(
                "fetch_oauth_config failed: {status} {body}"
            )));
        }

        let body: OAuthProvidersResp =
            resp.json().await.map_err(|e| sdk_err(&e))?;

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
        let config = self.sdk_config();
        let resp: SdkOperatorListResp = Self::rest_get_query(
            &config,
            "/v1/auth/operators/by-user",
            &[
                ("platform_id", platform_id.as_str()),
                ("user_id", user_id),
            ],
        )
        .await?;

        Ok(resp
            .operators
            .into_iter()
            .map(operator_resp_from_rest)
            .collect())
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
    /// Uses the public SDK configuration (no Authorization
    /// header) because tachyon-api's handler ignores the
    /// caller's executor and always uses SystemUser
    /// internally. This allows anonymous library visitors to
    /// resolve organizations without a valid token.
    pub async fn get_operator_by_alias(
        &self,
        platform_id: &TenantId,
        alias: &str,
    ) -> errors::Result<OperatorResp> {
        let config = self.sdk_config_public();
        let resp: SdkOperatorResp = Self::rest_get_query(
            &config,
            "/v1/auth/operators/by-alias",
            &[("platform_id", platform_id.as_str()), ("alias", alias)],
        )
        .await
        .map_err(|e| {
            if is_not_found(&e) {
                errors::Error::not_found("Operator".to_string())
            } else {
                e
            }
        })?;

        Ok(operator_resp_from_rest(resp))
    }

    /// Verify a bearer token via SDK.
    /// Uses public endpoint (no auth required).
    pub async fn verify_token(&self, token: &str) -> errors::Result<User> {
        let config = self.sdk_config_public();
        let req = tachyon_sdk::models::VerifyRequest {
            token: token.to_string(),
        };

        let resp = tachyon_sdk::apis::auth_verify_api::verify(&config, req)
            .await
            .map_err(sdk_api_err)?;

        user_from_sdk_model(&resp.user)
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
                .await
                .map_err(|_| {
                    errors::Error::unauthorized(
                        "API key verification failed".to_string(),
                    )
                })?;

        let id: ServiceAccountId =
            resp.service_account_id.parse().map_err(|e| {
                sdk_err(format!("Invalid service account id: {e}"))
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
        let config = self.sdk_config();
        let req = tachyon_sdk::models::SignInWithPlatformRequest {
            platform_id: platform_id.to_string(),
            access_token: access_token.to_string(),
            allow_sign_up: Some(allow_sign_up),
            email: Some(email.map(|s| s.to_string())),
            name: Some(name.map(|s| s.to_string())),
        };

        let resp =
            tachyon_sdk::apis::auth_verify_api::sign_in_with_platform(
                &config, req,
            )
            .await
            .map_err(sdk_api_err)?;

        user_from_sdk_model(&resp.user)
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
                        .map_err(|e| sdk_err(&e))?
                        .with_timezone(&chrono::Utc);

                let policy_id: PolicyId =
                    m.policy_id.parse().map_err(|e| {
                        sdk_err(format!("Invalid policy_id: {e}"))
                    })?;

                let user_id: UserId = m.user_id.parse().map_err(|e| {
                    sdk_err(format!("Invalid user_id: {e}"))
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
struct SdkOperatorListResp {
    operators: Vec<SdkOperatorResp>,
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

fn sdk_err(msg: impl std::fmt::Display) -> errors::Error {
    errors::Error::internal_server_error(format!("SDK auth error: {msg}"))
}

/// Convert a tachyon-sdk API error into an errors::Error.
fn sdk_api_err<T: std::fmt::Debug>(
    err: tachyon_sdk::apis::Error<T>,
) -> errors::Error {
    match err {
        tachyon_sdk::apis::Error::ResponseError(resp) => {
            let msg = if let Some(entity) = &resp.entity {
                format!("API error ({}): {:?}", resp.status, entity)
            } else {
                format!("API error ({}): {}", resp.status, resp.content)
            };

            map_http_error(resp.status, &msg)
        }
        tachyon_sdk::apis::Error::Reqwest(e) => {
            sdk_err(format!("HTTP request failed: {e}"))
        }
        tachyon_sdk::apis::Error::Serde(e) => {
            sdk_err(format!("Response parse error: {e}"))
        }
        tachyon_sdk::apis::Error::Io(e) => {
            sdk_err(format!("IO error: {e}"))
        }
    }
}

/// Map HTTP status to errors::Error.
fn map_http_error(status: reqwest::StatusCode, msg: &str) -> errors::Error {
    match status.as_u16() {
        401 => errors::Error::unauthorized(msg.to_string()),
        403 => errors::Error::forbidden(msg.to_string()),
        404 => errors::Error::not_found(msg.to_string()),
        _ => errors::Error::internal_server_error(msg.to_string()),
    }
}

/// Handle a REST response: check status and deserialize.
async fn handle_rest_response<T: serde::de::DeserializeOwned>(
    resp: reqwest::Response,
) -> errors::Result<T> {
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(map_http_error(status, &body));
    }
    resp.json::<T>().await.map_err(|e| sdk_err(&e))
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
    let operator_name: Identifier = resp
        .operator_name
        .parse()
        .map_err(|e| sdk_err(format!("Invalid operator_name: {e}")))?;
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
    let operator_name: Identifier = resp
        .operator_name
        .parse()
        .map_err(|e| sdk_err(format!("Invalid operator_name: {e}")))?;
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
        .map_err(|e| sdk_err(format!("Invalid user id: {e}")))?;
    let username = id.to_string();
    let email: Option<String> = user
        .email
        .as_ref()
        .and_then(|e| e.as_ref())
        .cloned();
    let name: Option<String> = user
        .name
        .as_ref()
        .and_then(|n| n.as_ref())
        .cloned();
    let role: tachyon_sdk::auth::DefaultRole =
        user.role.parse().unwrap_or(tachyon_sdk::auth::DefaultRole::General);

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
        .map_err(|e| sdk_err(format!("Invalid user id: {e}")))?;
    let username = id.to_string();
    let email: Option<String> = resp
        .email
        .as_ref()
        .and_then(|e| e.as_ref())
        .cloned();
    let name: Option<String> = resp
        .name
        .as_ref()
        .and_then(|n| n.as_ref())
        .cloned();
    let role: tachyon_sdk::auth::DefaultRole =
        resp.role.parse().unwrap_or(tachyon_sdk::auth::DefaultRole::General);
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

/// Construct a User from REST user response.
fn user_from_rest_user_response(
    resp: &RestUserResponse,
) -> errors::Result<User> {
    let id: UserId = resp
        .id
        .parse()
        .map_err(|e| sdk_err(format!("Invalid user id: {e}")))?;
    let username = id.to_string();
    let email: Option<String> = resp.email.clone();
    let name: Option<String> = resp.name.clone();
    let role: tachyon_sdk::auth::DefaultRole =
        resp.role.parse().unwrap_or(tachyon_sdk::auth::DefaultRole::General);
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
        .map_err(|e| sdk_err(&e))?
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
        sdk_err(format!("Invalid api key id: {e}"))
    })?;
    let sa_id: ServiceAccountId = resp.service_account_id.clone().into();
    let value: PublicApiKeyValue = resp.value.parse().map_err(|e| {
        sdk_err(format!("Invalid api key value: {e}"))
    })?;
    let created_at = chrono::DateTime::parse_from_rfc3339(&resp.created_at)
        .map_err(|e| sdk_err(&e))?
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

        let resp =
            tachyon_sdk::apis::auth_policies_api::evaluate_policies_batch(
                &config, req,
            )
            .await
            .map_err(|e| {
                tracing::debug!(
                    action = %input.action,
                    error = %e,
                    "check_policy failed"
                );
                errors::Error::forbidden(format!(
                    "Policy check failed for action: {}",
                    input.action
                ))
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
        Err(sdk_err("get_tenant_hierarchy not supported via SDK"))
    }

    async fn get_user_id_by_user_provider_id<'a>(
        &self,
        _input: &auth::GetUserIdByUserProviderIdInput<'a>,
    ) -> errors::Result<Option<String>> {
        Err(sdk_err("get_user_id_by_user_provider_id not supported"))
    }

    async fn delete_operator<'a>(
        &self,
        _input: &auth::DeleteOperatorInput<'a>,
    ) -> errors::Result<()> {
        Err(sdk_err("delete_operator not supported via SDK"))
    }

    async fn get_operator_by_identifier<'a>(
        &self,
        _input: &auth::GetOperatorByIdentifierInput<'a>,
    ) -> errors::Result<Option<Operator>> {
        Err(sdk_err("get_operator_by_identifier not supported via SDK"))
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
                        .map_err(|e| sdk_err(&e))?
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
        Err(sdk_err("update_service_account not supported via SDK"))
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
                &input.tenant_id.to_string(),
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
            &input.service_account_id.to_string(),
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
            &input.service_account_id.to_string(),
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
            &input.service_account_id.to_string(),
            &input.operator_id.to_string(),
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
            &input.user_id.to_string(),
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
            &input.tenant_id.to_string(),
        )
        .await
        .map_err(sdk_api_err)?;

        resp.users
            .iter()
            .map(|u| user_from_sdk_user_response(u))
            .collect()
    }

    async fn get_policy_by_id<'a>(
        &self,
        input: &auth::GetPolicyByIdInput<'a>,
    ) -> errors::Result<Option<Policy>> {
        let config = self
            .sdk_config_with_context(input.executor, input.multi_tenancy);

        match tachyon_sdk::apis::auth_policies_api::get_policy(
            &config,
            &input.policy_id.to_string(),
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
            "find_policy_by_name not implemented in SdkAuthApp"
                .to_string(),
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
        Err(sdk_err("create_mapping: use AuthApp trait"))
    }

    async fn delete_mapping(
        &self,
        _user_id: &tachyon_sdk::auth::UserId,
        _policy_id: &tachyon_sdk::auth::PolicyId,
        _tenant_id: &TenantId,
    ) -> errors::Result<()> {
        Err(sdk_err("delete_mapping: use AuthApp trait"))
    }

    async fn find_policies_by_user(
        &self,
        _user_id: &tachyon_sdk::auth::UserId,
        _tenant_id: &TenantId,
    ) -> errors::Result<Vec<tachyon_sdk::auth::PolicyId>> {
        Err(sdk_err("find_policies_by_user: use AuthApp"))
    }

    async fn find_users_by_policy(
        &self,
        _policy_id: &tachyon_sdk::auth::PolicyId,
        _tenant_id: &TenantId,
    ) -> errors::Result<Vec<tachyon_sdk::auth::UserId>> {
        Err(sdk_err("find_users_by_policy: use AuthApp"))
    }

    async fn exists_mapping(
        &self,
        _user_id: &tachyon_sdk::auth::UserId,
        _policy_id: &tachyon_sdk::auth::PolicyId,
        _tenant_id: &TenantId,
    ) -> errors::Result<bool> {
        Err(sdk_err("exists_mapping: use AuthApp"))
    }

    async fn create_mapping_with_scope(
        &self,
        _user_id: &tachyon_sdk::auth::UserId,
        _policy_id: &tachyon_sdk::auth::PolicyId,
        _tenant_id: &TenantId,
        _resource_scope: &str,
    ) -> errors::Result<()> {
        Err(sdk_err("create_mapping_with_scope: use AuthApp"))
    }

    async fn delete_mapping_with_scope(
        &self,
        _user_id: &tachyon_sdk::auth::UserId,
        _policy_id: &tachyon_sdk::auth::PolicyId,
        _tenant_id: &TenantId,
        _resource_scope: &str,
    ) -> errors::Result<()> {
        Err(sdk_err("delete_mapping_with_scope: use AuthApp"))
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

        let sdk_tenant_id =
            TenantId::new(&token.tenant_id.to_string())?;
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
        let sdk_tenant_id =
            TenantId::new(&tenant_id.to_string())?;
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
        let sdk_tenant_id =
            TenantId::new(&tenant_id.to_string())?;
        let config = self.sdk.sdk_config_for_tenant(&sdk_tenant_id);
        let path = format!("/v1/auth/oauth-tokens/{}", provider);
        SdkAuthApp::rest_delete(&config, &path)
            .await
    }
}
