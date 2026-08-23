//! GitHub OAuth provider.
//!
//! Provides OAuth authentication for GitHub, enabling access to
//! repositories and other GitHub resources.
//!
//! # Example
//!
//! ```ignore
//! use github_provider::{GitHub, OAuthConfig};
//!
//! let config = OAuthConfig {
//!     client_id: "your_client_id".to_string(),
//!     client_secret: "your_client_secret".to_string(),
//!     redirect_uri: "http://localhost:3000/oauth/github/callback".to_string(),
//! };
//!
//! let github = GitHub::new(Some(config));
//!
//! // Get authorization URL
//! let url = github.authorization_url(&["repo", "read:user"], "state123")?;
//!
//! // Exchange code for token (after user authorizes)
//! let token = github.exchange_token("code_from_callback").await?;
//! ```

mod oauth;

pub use oauth::*;

/// OAuth configuration for GitHub provider.
#[derive(Debug, Clone)]
pub struct OAuthConfig {
    /// GitHub OAuth client ID.
    pub client_id: String,
    /// GitHub OAuth client secret.
    pub client_secret: String,
    /// Redirect URI for OAuth callback.
    pub redirect_uri: String,
}

/// Supplies the GitHub OAuth configuration when a flow needs it.
///
/// The configuration lives in Tachyon's IaC configuration, which costs
/// a round trip to fetch. Reading it through this trait lets that
/// happen on the OAuth endpoints that need it, instead of in front of
/// every cold start.
#[async_trait::async_trait]
pub trait OAuthConfigSource: std::fmt::Debug + Send + Sync {
    async fn github_oauth_config(&self) -> Option<OAuthConfig>;
}

/// GitHub provider client.
#[derive(Debug, Clone, Default)]
pub struct GitHub {
    oauth: Option<std::sync::Arc<dyn OAuthConfigSource>>,
}

impl GitHub {
    /// Create a client that resolves its OAuth configuration from
    /// `source`.
    pub fn new(
        source: Option<std::sync::Arc<dyn OAuthConfigSource>>,
    ) -> Self {
        Self { oauth: source }
    }

    /// The OAuth configuration, or `None` when GitHub OAuth is not
    /// configured for this deployment.
    pub(crate) async fn oauth_config(&self) -> Option<OAuthConfig> {
        self.oauth.as_ref()?.github_oauth_config().await
    }
}
