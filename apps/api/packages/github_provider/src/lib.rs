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

/// GitHub provider client.
#[derive(Debug, Clone)]
pub struct GitHub {
    oauth: Option<OAuthConfig>,
}

impl GitHub {
    /// Create a new GitHub client with OAuth configuration.
    pub fn new(oauth: Option<OAuthConfig>) -> Self {
        Self { oauth }
    }

    /// Create a new GitHub client from environment variables.
    ///
    /// Reads `GITHUB_CLIENT_ID`, `GITHUB_CLIENT_SECRET`, and `GITHUB_REDIRECT_URI`.
    pub fn from_env() -> Self {
        let oauth =
            if let (Ok(client_id), Ok(client_secret), Ok(redirect_uri)) = (
                std::env::var("GITHUB_CLIENT_ID"),
                std::env::var("GITHUB_CLIENT_SECRET"),
                std::env::var("GITHUB_REDIRECT_URI"),
            ) {
                Some(OAuthConfig {
                    client_id,
                    client_secret,
                    redirect_uri,
                })
            } else {
                None
            };

        Self { oauth }
    }

    /// Check if OAuth is configured.
    pub fn is_oauth_configured(&self) -> bool {
        self.oauth.is_some()
    }

    /// Get the OAuth client secret (for CSRF state signing).
    ///
    /// Returns None if OAuth is not configured.
    pub fn client_secret(&self) -> Option<&str> {
        self.oauth.as_ref().map(|o| o.client_secret.as_str())
    }
}
