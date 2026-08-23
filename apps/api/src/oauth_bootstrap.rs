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
        self.config
            .get_or_try_init(|| async {
                self.sdk.fetch_oauth_config(&self.tenant).await.inspect_err(
                    |error| {
                        tracing::warn!(
                            %error,
                            "failed to fetch OAuth config",
                        );
                    },
                )
            })
            .await
            .ok()
    }

    /// The GitHub client secret, used to sign OAuth CSRF state.
    pub async fn github_client_secret(&self) -> Option<String> {
        let credentials = self.get().await?.github_credentials.as_ref()?;
        Some(credentials.client_secret.clone())
    }
}

#[async_trait::async_trait]
impl github_provider::OAuthConfigSource for OAuthBootstrap {
    async fn github_oauth_config(
        &self,
    ) -> Option<github_provider::OAuthConfig> {
        let credentials = self.get().await?.github_credentials.as_ref()?;
        Some(github_provider::OAuthConfig {
            client_id: credentials.client_id.clone(),
            client_secret: credentials.client_secret.clone(),
            redirect_uri: github_redirect_uri(&credentials.redirect_uri),
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
        let config = self.get().await?;
        let credentials = match provider {
            OAuthProvider::Github => config.github_credentials.as_ref()?,
            OAuthProvider::Linear => config.linear_credentials.as_ref()?,
            _ => return None,
        };

        // The redirect URI may be overridden per deployment, the same
        // way the startup path allowed.
        let redirect_uri = match provider {
            OAuthProvider::Github => {
                github_redirect_uri(&credentials.redirect_uri)
            }
            _ => credentials.redirect_uri.clone(),
        };

        Some(OAuthClientCredentials {
            client_id: credentials.client_id.clone(),
            client_secret: credentials.client_secret.clone(),
            redirect_uri,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    const TEST_TENANT_ID: &str = "tn_01j702qf86pc2j35s0kv0gv3gy";

    /// Serves `/v1/iac/oauth-providers`, failing the first
    /// `failures` requests, and reports how many it received.
    async fn bootstrap_against_tachyon(
        failures: usize,
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

        assert!(
            bootstrap.credentials(OAuthProvider::Github).await.is_none(),
            "the first resolution fails"
        );
        assert!(
            bootstrap.credentials(OAuthProvider::Github).await.is_some(),
            "the next one tries again rather than serving the failure"
        );
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
