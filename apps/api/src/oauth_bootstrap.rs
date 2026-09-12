//! OAuth client credentials, fetched from tachyon-api when first
//! needed.
//!
//! These live in Tachyon's IaC configuration, so reading them costs a
//! round trip to tachyon-api. Fetching them during startup put that
//! round trip in front of every cold start: production traces put it at
//! roughly 1.4s of a 1.9s init, for a value most requests never read —
//! only the OAuth connect flows do.

use std::sync::Arc;

use inbound_sync_domain::{
    OAuthClientCredentials, OAuthCredentialsSource, OAuthProvider,
};
use tachyon_sdk::auth::TenantId;
use tokio::sync::OnceCell;

use crate::sdk_auth::{OAuthBootstrapConfig, SdkAuthApp};

/// The tenant's OAuth configuration, fetched at most once per process.
pub struct OAuthBootstrap {
    sdk: Arc<SdkAuthApp>,
    tenant: TenantId,
    config: OnceCell<OAuthBootstrapConfig>,
}

impl std::fmt::Debug for OAuthBootstrap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OAuthBootstrap")
            .field("resolved", &self.config.initialized())
            .finish()
    }
}

impl OAuthBootstrap {
    pub fn new(sdk: Arc<SdkAuthApp>, tenant: TenantId) -> Self {
        Self {
            sdk,
            tenant,
            config: OnceCell::new(),
        }
    }

    /// The tenant's OAuth configuration, fetching it the first time.
    ///
    /// A failed fetch is not remembered: the startup version of this
    /// logged a warning and left the process without credentials for
    /// the rest of its life, so a single unlucky moment disabled the
    /// OAuth flows until the container was replaced. Here the next
    /// caller tries again.
    pub async fn get(&self) -> Option<&OAuthBootstrapConfig> {
        self.resolve().await.ok()
    }

    async fn resolve(&self) -> errors::Result<&OAuthBootstrapConfig> {
        self.config
            .get_or_try_init(|| async {
                let config = self
                    .sdk
                    .fetch_oauth_config(&self.tenant)
                    .await
                    .map_err(|error| {
                        errors::Error::service_unavailable(format!(
                            "OAuth configuration could not be loaded \
                             for Library platform {}: {error}",
                            self.tenant
                        ))
                    })?;
                if config.github_credentials.is_none()
                    && config.linear_credentials.is_none()
                {
                    // IaC may temporarily omit providers while resolving secrets.
                    // Do not retain an empty result for the Lambda's lifetime.
                    return Err(errors::Error::service_unavailable(format!(
                        "No OAuth providers available for Library platform {}",
                        self.tenant
                    )));
                }
                Ok(config)
            })
            .await
            .inspect_err(|error| {
                tracing::warn!(%error, "failed to resolve OAuth config")
            })
    }

    /// The GitHub client secret, used to sign OAuth CSRF state.
    pub async fn github_client_secret(&self) -> errors::Result<String> {
        Ok(self.github_credentials().await?.client_secret)
    }

    async fn github_credentials(
        &self,
    ) -> errors::Result<OAuthClientCredentials> {
        // A Preview can use its own App without replacing the platform's
        // shared IaC registration. All GitHub callers, including state signing
        // and token refresh, must resolve the same credential pair.
        if let Some(credentials) = deployment_github_credentials(
            std::env::var("LIBRARY_GITHUB_OAUTH_CLIENT_ID").ok(),
            std::env::var("LIBRARY_GITHUB_OAUTH_CLIENT_SECRET").ok(),
            std::env::var("GITHUB_REDIRECT_URI").ok(),
        )? {
            return Ok(credentials);
        }
        let config = self.resolve().await?;
        let credentials =
            config.github_credentials.as_ref().ok_or_else(|| {
                errors::Error::service_unavailable(format!(
                "GitHub OAuth provider unavailable for Library platform {}",
                self.tenant
            ))
            })?;
        Ok(OAuthClientCredentials {
            client_id: credentials.client_id.clone(),
            client_secret: credentials.client_secret.clone(),
            redirect_uri: github_redirect_uri(&credentials.redirect_uri),
        })
    }
}

fn deployment_github_credentials(
    client_id: Option<String>,
    client_secret: Option<String>,
    redirect_uri: Option<String>,
) -> errors::Result<Option<OAuthClientCredentials>> {
    // A redirect override alone retains the existing IaC behavior.
    if client_id.is_none() && client_secret.is_none() {
        return Ok(None);
    }
    match (client_id, client_secret, redirect_uri) {
        (Some(client_id), Some(client_secret), Some(redirect_uri))
            if !client_id.trim().is_empty()
                && !client_secret.trim().is_empty()
                && !redirect_uri.trim().is_empty() =>
        {
            Ok(Some(OAuthClientCredentials {
                client_id,
                client_secret,
                redirect_uri,
            }))
        }
        _ => Err(errors::Error::service_unavailable(
            "GitHub deployment OAuth override requires \
             LIBRARY_GITHUB_OAUTH_CLIENT_ID, \
             LIBRARY_GITHUB_OAUTH_CLIENT_SECRET and GITHUB_REDIRECT_URI",
        )),
    }
}

#[async_trait::async_trait]
impl github_provider::OAuthConfigSource for OAuthBootstrap {
    async fn github_oauth_config(
        &self,
    ) -> Option<github_provider::OAuthConfig> {
        let credentials = self.github_credentials().await.ok()?;
        Some(github_provider::OAuthConfig {
            client_id: credentials.client_id,
            client_secret: credentials.client_secret,
            redirect_uri: credentials.redirect_uri,
        })
    }
}

/// The redirect URI, allowing the per-deployment override the startup
/// path also honoured.
fn github_redirect_uri(configured: &str) -> String {
    std::env::var("GITHUB_REDIRECT_URI")
        .unwrap_or_else(|_| configured.to_string())
}

#[async_trait::async_trait]
impl OAuthCredentialsSource for OAuthBootstrap {
    async fn credentials(
        &self,
        provider: OAuthProvider,
    ) -> Option<OAuthClientCredentials> {
        if provider == OAuthProvider::Github {
            return self.github_credentials().await.ok();
        }
        let config = self.get().await?;
        let credentials = match provider {
            OAuthProvider::Linear => config.linear_credentials.as_ref()?,
            _ => return None,
        };

        Some(OAuthClientCredentials {
            client_id: credentials.client_id.clone(),
            client_secret: credentials.client_secret.clone(),
            redirect_uri: credentials.redirect_uri.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    const TEST_TENANT_ID: &str = "tn_01j702qf86pc2j35s0kv0gv3gy";

    #[test]
    fn deployment_override_preserves_the_complete_credential_pair() {
        let credentials = deployment_github_credentials(
            Some("preview-client".into()),
            Some("preview-secret".into()),
            Some("https://example.test/callback".into()),
        )
        .unwrap()
        .unwrap();
        assert_eq!(credentials.client_id, "preview-client");
        assert_eq!(credentials.client_secret, "preview-secret");
        assert_eq!(
            credentials.redirect_uri,
            "https://example.test/callback"
        );
        assert!(deployment_github_credentials(
            None,
            None,
            Some("https://example.test/callback".into()),
        )
        .unwrap()
        .is_none());
    }

    #[test]
    fn incomplete_deployment_override_cannot_fall_back_to_another_app() {
        for (client_id, secret, redirect) in [
            (
                Some("preview-client"),
                None,
                Some("https://example.test/cb"),
            ),
            (
                None,
                Some("preview-secret"),
                Some("https://example.test/cb"),
            ),
            (Some("preview-client"), Some("preview-secret"), None),
            (
                Some(""),
                Some("preview-secret"),
                Some("https://example.test/cb"),
            ),
            (
                Some("preview-client"),
                Some("  "),
                Some("https://example.test/cb"),
            ),
            (Some("preview-client"), Some("preview-secret"), Some("")),
        ] {
            let error = deployment_github_credentials(
                client_id.map(str::to_owned),
                secret.map(str::to_owned),
                redirect.map(str::to_owned),
            )
            .unwrap_err()
            .to_string();
            assert!(
                error.contains("GitHub deployment OAuth override requires")
            );
            assert!(!error.contains("preview-secret"));
            assert!(!error.contains("preview-client"));
        }
    }

    /// Serves `/v1/iac/oauth-providers`, failing the first
    /// `failures` requests, and reports how many it received.
    async fn bootstrap_against_tachyon(
        failures: usize,
    ) -> (Arc<OAuthBootstrap>, Arc<AtomicUsize>) {
        bootstrap_with_empty_responses(failures, 0).await
    }

    async fn bootstrap_with_empty_responses(
        failures: usize,
        empty_responses: usize,
    ) -> (Arc<OAuthBootstrap>, Arc<AtomicUsize>) {
        let hits = Arc::new(AtomicUsize::new(0));
        let counter = hits.clone();

        let app = axum::Router::new().route(
            "/v1/iac/oauth-providers",
            axum::routing::get(move || {
                let seen = counter.fetch_add(1, Ordering::SeqCst);
                async move {
                    if seen < failures {
                        return Err(
                            axum::http::StatusCode::SERVICE_UNAVAILABLE,
                        );
                    }
                    if seen < failures + empty_responses {
                        return Ok(axum::Json(
                            serde_json::json!({"providers": []}),
                        ));
                    }
                    Ok(axum::Json(serde_json::json!({
                        "providers": [{
                            "provider": "github",
                            "client_id": "client-id",
                            "client_secret": "client-secret",
                            "redirect_uri": "https://example.test/cb",
                            "webhook_secret": null,
                        }],
                    })))
                }
            }),
        );

        let listener =
            tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        let tenant: TenantId = TEST_TENANT_ID.parse().unwrap();
        let sdk = Arc::new(SdkAuthApp::new(
            format!("http://{addr}"),
            &tenant,
            "process-level-token",
        ));

        (Arc::new(OAuthBootstrap::new(sdk, tenant)), hits)
    }

    #[tokio::test]
    async fn resolves_the_configured_credentials() {
        let (bootstrap, _) = bootstrap_against_tachyon(0).await;

        let credentials = bootstrap
            .credentials(OAuthProvider::Github)
            .await
            .expect("GitHub is configured");

        assert_eq!(credentials.client_id, "client-id");
        assert_eq!(credentials.client_secret, "client-secret");
    }

    /// The whole point of holding the configuration: the round trip to
    /// tachyon-api happens once, not on every OAuth call.
    #[tokio::test]
    async fn fetches_the_configuration_once() {
        let (bootstrap, hits) = bootstrap_against_tachyon(0).await;

        for _ in 0..3 {
            bootstrap.credentials(OAuthProvider::Github).await.unwrap();
        }
        bootstrap.github_client_secret().await.unwrap();

        assert_eq!(hits.load(Ordering::SeqCst), 1);
    }

    /// The startup version logged a warning and left the process
    /// without credentials for the rest of its life. A failure here is
    /// only this caller's failure.
    #[tokio::test]
    async fn retries_after_a_failed_fetch() {
        // A status response is not retried — only transport failures
        // are — so one failing request fails one `get`.
        let (bootstrap, _) = bootstrap_against_tachyon(1).await;

        let error = bootstrap
            .github_client_secret()
            .await
            .unwrap_err()
            .to_string();
        assert!(error.contains("could not be loaded"));
        assert!(error.contains(TEST_TENANT_ID));
        assert!(!error.contains("client-secret"));
        assert!(
            bootstrap.credentials(OAuthProvider::Github).await.is_some(),
            "the next one tries again rather than serving the failure"
        );
    }

    #[tokio::test]
    async fn retries_after_temporarily_empty_provider_configuration() {
        let (bootstrap, hits) = bootstrap_with_empty_responses(0, 1).await;
        let error = bootstrap
            .github_client_secret()
            .await
            .unwrap_err()
            .to_string();
        assert!(error.contains("No OAuth providers available"));
        assert!(!bootstrap.config.initialized());
        assert!(bootstrap.github_client_secret().await.is_ok());
        assert!(bootstrap.github_client_secret().await.is_ok());
        assert_eq!(hits.load(Ordering::SeqCst), 2);
    }

    /// A provider tachyon does not report is simply not configured.
    #[tokio::test]
    async fn reports_an_unconfigured_provider_as_absent() {
        let (bootstrap, _) = bootstrap_against_tachyon(0).await;

        assert!(bootstrap
            .credentials(OAuthProvider::Linear)
            .await
            .is_none());
    }
}
