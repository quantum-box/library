//! Authenticated broker calls. GitHub App/refresh secrets stay in Tachyon.
use super::*;

pub(super) const TOKEN_PATH: &str = "/v1/integrations/github/oauth/token";

pub fn github_oauth_broker_enabled() -> bool {
    std::env::var("LIBRARY_GITHUB_OAUTH_BROKER_ENABLED")
        .is_ok_and(|v| v == "true")
}

#[derive(Deserialize)]
pub struct BrokerAuthorization {
    pub authorization_url: String,
    pub state: String,
}

#[derive(Deserialize)]
pub struct BrokerConnection {
    pub connected: bool,
    pub username: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize)]
struct BrokerToken {
    provider_user_id: String,
    access_token: String,
    expires_at: chrono::DateTime<chrono::Utc>,
}

impl SdkAuthApp {
    pub(super) fn github_broker_tenant(
        &self,
        tenant: &TenantId,
        bearer: &str,
    ) -> errors::Result<Configuration> {
        let mut headers = reqwest::header::HeaderMap::new();
        for (key, value) in [
            ("authorization", format!("Bearer {bearer}")),
            ("x-operator-id", tenant.to_string()),
            ("x-platform-id", crate::domain::LIBRARY_TENANT.to_string()),
        ] {
            headers.insert(
                key,
                value.parse().map_err(|_| {
                    errors::Error::invalid("Invalid broker request context")
                })?,
            );
        }
        Ok(self.configuration(headers))
    }

    pub(super) fn github_broker_context(
        &self,
        executor: &dyn auth::ExecutorAction,
        tenancy: &dyn auth::MultiTenancyAction,
    ) -> errors::Result<Configuration> {
        // Preserve the caller; never use SystemUser to bypass consumer policies.
        let bearer = if executor.is_user() || executor.is_service_account()
        {
            request_caller_token()
                .unwrap_or_else(|| self.auth_token.clone())
        } else {
            self.auth_token.clone()
        };
        self.github_broker_tenant(&tenancy.get_operator_id()?, &bearer)
    }

    pub async fn start_github_oauth(
        &self,
        executor: &dyn auth::ExecutorAction,
        tenancy: &dyn auth::MultiTenancyAction,
        return_url: &str,
        code_challenge: &str,
    ) -> errors::Result<BrokerAuthorization> {
        require_enabled()?;
        let config = self.github_broker_context(executor, tenancy)?;
        Self::rest_post(
            &config,
            "/v1/integrations/github/oauth/start",
            &serde_json::json!({
                "return_url": return_url, "code_challenge": code_challenge,
            }),
        )
        .await
    }

    pub async fn complete_github_oauth(
        &self,
        executor: &dyn auth::ExecutorAction,
        tenancy: &dyn auth::MultiTenancyAction,
        session: &str,
        code_verifier: &str,
    ) -> errors::Result<BrokerConnection> {
        require_enabled()?;
        let config = self.github_broker_context(executor, tenancy)?;
        Self::rest_post(
            &config,
            "/v1/integrations/github/oauth/complete",
            &serde_json::json!({
                "session": session, "code_verifier": code_verifier,
            }),
        )
        .await
    }

    pub(super) async fn github_broker_token(
        config: &Configuration,
    ) -> errors::Result<Option<auth::OAuthTokenDetail>> {
        // The broker may rotate a near-expiry token. Allow its bounded provider
        // request to finish and do not retry an uncertain rotation from here.
        let response = config
            .client
            .get(format!("{}{}", config.base_path, TOKEN_PATH))
            .timeout(Duration::from_secs(25))
            .send()
            .await
            .map_err(|_| {
                errors::Error::service_unavailable(
                    "GitHub OAuth broker is unavailable",
                )
            })?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let token: BrokerToken = handle_rest_response(response)
            .await
            .map_err(errors::Error::from)?;
        Ok(Some(auth::OAuthTokenDetail {
            provider: "github".into(),
            provider_user_id: token.provider_user_id,
            access_token: token.access_token,
            refresh_token: None,
            expires_at: token.expires_at,
        }))
    }
}

fn require_enabled() -> errors::Result<()> {
    if github_oauth_broker_enabled() {
        Ok(())
    } else {
        Err(errors::Error::service_unavailable(
            "GitHub OAuth broker is not enabled for this deployment",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct User;
    impl auth::ExecutorAction for User {
        fn get_id(&self) -> &str {
            "us_01hs2yepy5hw4rz8pdq2wywnwt"
        }
        fn has_tenant_id(&self, _: &TenantId) -> bool {
            true
        }
        fn is_system_user(&self) -> bool {
            false
        }
        fn is_user(&self) -> bool {
            true
        }
        fn is_service_account(&self) -> bool {
            false
        }
        fn is_none(&self) -> bool {
            false
        }
    }

    #[tokio::test]
    async fn broker_preserves_caller_and_tenant_and_never_returns_refresh_secrets(
    ) {
        let operator: TenantId =
            "tn_01hy91qw3362djx6z9jerr34v4".parse().unwrap();
        let requests = Arc::new(std::sync::Mutex::new(Vec::new()));
        let captured = requests.clone();
        let router = axum::Router::new().route(TOKEN_PATH, axum::routing::get(
            move |headers: axum::http::HeaderMap| {
                let captured = captured.clone();
                async move {
                    captured.lock().unwrap().push(headers);
                    axum::Json(serde_json::json!({
                        "provider_user_id": "octocat", "access_token": "test-access",
                        "refresh_token": "must-never-be-forwarded", "expires_at": "2030-01-01T00:00:00Z"
                    }))
                }
            }
        ));
        let listener =
            tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        let sdk = SdkAuthApp::new(
            format!("http://{addr}"),
            &operator,
            "process-secret",
        );
        let tenancy = auth::MultiTenancy::new(
            Some(operator.clone()),
            Some(operator.clone()),
        );
        let token = caller_token_scope(Some("caller-jwt".into()), async {
            let config =
                sdk.github_broker_context(&User, &tenancy).unwrap();
            SdkAuthApp::github_broker_token(&config)
                .await
                .unwrap()
                .unwrap()
        })
        .await;
        server.abort();
        let seen = requests.lock().unwrap();
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0]["authorization"], "Bearer caller-jwt");
        assert_eq!(seen[0]["x-operator-id"], operator.as_str());
        assert_eq!(
            seen[0]["x-platform-id"],
            crate::domain::LIBRARY_TENANT.as_str()
        );
        assert_eq!(token.access_token, "test-access");
        assert!(token.refresh_token.is_none());
    }

    #[tokio::test]
    async fn broker_distinguishes_disconnection_and_does_not_retry_uncertain_rotation(
    ) {
        use std::sync::atomic::{AtomicUsize, Ordering};
        for status in [
            axum::http::StatusCode::NOT_FOUND,
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
        ] {
            let requests = Arc::new(AtomicUsize::new(0));
            let captured = requests.clone();
            let router = axum::Router::new().route(
                TOKEN_PATH,
                axum::routing::get(move || {
                    let captured = captured.clone();
                    async move {
                        captured.fetch_add(1, Ordering::SeqCst);
                        (status, "sensitive-provider-body")
                    }
                }),
            );
            let listener =
                tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            let server = tokio::spawn(async move {
                axum::serve(listener, router).await.unwrap();
            });
            let config = Configuration {
                base_path: format!("http://{addr}"),
                client: reqwest::Client::new(),
                ..Default::default()
            };
            let result = SdkAuthApp::github_broker_token(&config).await;
            server.abort();
            assert_eq!(requests.load(Ordering::SeqCst), 1);
            if status == axum::http::StatusCode::NOT_FOUND {
                assert!(result.unwrap().is_none());
            } else {
                assert!(!result
                    .err()
                    .unwrap()
                    .to_string()
                    .contains("sensitive-provider-body"));
            }
        }
    }
}
