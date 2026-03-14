//! GitHub OAuth implementation.

use serde::Deserialize;
use std::fmt::Debug;

/// OAuth token returned after authorization code exchange.
#[derive(Debug, Clone)]
pub struct OAuthToken {
    /// Access token for API calls.
    pub access_token: String,
    /// Token type (usually "bearer").
    pub token_type: String,
    /// Token expiration in seconds.
    pub expires_in: i64,
    /// Refresh token (if available).
    pub refresh_token: Option<String>,
    /// Granted scopes.
    pub scope: Option<String>,
    /// Provider-specific user identifier.
    pub provider_user_id: String,
}

/// OAuth provider trait for authorization and token exchange.
#[async_trait::async_trait]
pub trait OAuthProvider: Debug + Send + Sync {
    /// Provider name (e.g., "github").
    fn provider_name(&self) -> &'static str;
    /// Build the authorization URL.
    fn authorization_url(
        &self,
        scope: &[&str],
        state: &str,
    ) -> errors::Result<String>;
    /// Token endpoint URL.
    fn token_endpoint(&self) -> &'static str;
    /// Exchange an authorization code for an access token.
    async fn exchange_token(
        &self,
        code: &str,
    ) -> errors::Result<OAuthToken>;
    /// Refresh an expired access token.
    async fn refresh_token(
        &self,
        refresh_token: &str,
    ) -> errors::Result<OAuthToken>;
}

use crate::GitHub;

const GITHUB_AUTHORIZE_URL: &str =
    "https://github.com/login/oauth/authorize";
const GITHUB_TOKEN_URL: &str =
    "https://github.com/login/oauth/access_token";
const GITHUB_USER_URL: &str = "https://api.github.com/user";
const GITHUB_USER_REPOS_URL: &str = "https://api.github.com/user/repos";
const GITHUB_REPOS_URL: &str = "https://api.github.com/repos";

/// Default OAuth scopes for GitHub.
///
/// - `repo`: Full control of private repositories
/// - `read:user`: Read access to user profile data
pub const DEFAULT_SCOPES: [&str; 2] = ["repo", "read:user"];

#[derive(Debug, Deserialize)]
struct GitHubTokenResponse {
    access_token: String,
    token_type: String,
    scope: Option<String>,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GitHubUser {
    id: i64,
    login: String,
}

/// GitHub repository information.
#[derive(Debug, Clone, Deserialize)]
pub struct GitHubRepository {
    /// Repository ID.
    pub id: i64,
    /// Repository name.
    pub name: String,
    /// Full name (owner/repo format).
    pub full_name: String,
    /// Repository description.
    pub description: Option<String>,
    /// Whether the repository is private.
    pub private: bool,
    /// HTML URL to the repository.
    pub html_url: String,
    /// Default branch name.
    pub default_branch: Option<String>,
}

/// GitHub file/directory information from Contents API.
#[derive(Debug, Clone, Deserialize)]
pub struct GitHubContent {
    /// File/directory name.
    pub name: String,
    /// Full path in the repository.
    pub path: String,
    /// SHA hash of the content.
    pub sha: String,
    /// Size in bytes (0 for directories).
    pub size: i64,
    /// Type: "file", "dir", "symlink", or "submodule".
    #[serde(rename = "type")]
    pub content_type: String,
    /// Download URL (only for files).
    pub download_url: Option<String>,
    /// HTML URL to view on GitHub.
    pub html_url: Option<String>,
    /// Base64-encoded content (only for small files < 1MB).
    pub content: Option<String>,
    /// Encoding of the content field.
    pub encoding: Option<String>,
}

/// Result of listing directory contents.
#[derive(Debug, Clone)]
pub struct GitHubDirectoryListing {
    /// List of files and directories.
    pub contents: Vec<GitHubContent>,
    /// Whether the listing was truncated (more than 1000 items).
    pub truncated: bool,
}

#[derive(Debug, Deserialize)]
struct GitHubErrorResponse {
    error: String,
    error_description: Option<String>,
}

#[async_trait::async_trait]
impl OAuthProvider for GitHub {
    fn provider_name(&self) -> &'static str {
        "github"
    }

    #[tracing::instrument(skip(state))]
    fn authorization_url(
        &self,
        scope: &[&str],
        state: &str,
    ) -> errors::Result<String> {
        let oauth = self.oauth.as_ref().ok_or_else(|| {
            errors::Error::provider_error(
                "github",
                "OAuth is not configured",
            )
        })?;

        let scope_str = scope.join(" ");

        let url = format!(
            "{}?client_id={}&redirect_uri={}&scope={}&state={}",
            GITHUB_AUTHORIZE_URL,
            oauth.client_id,
            urlencoding::encode(&oauth.redirect_uri),
            urlencoding::encode(&scope_str),
            state
        );

        Ok(url)
    }

    fn token_endpoint(&self) -> &'static str {
        GITHUB_TOKEN_URL
    }

    #[tracing::instrument(skip(code))]
    async fn exchange_token(
        &self,
        code: &str,
    ) -> errors::Result<OAuthToken> {
        let oauth = self.oauth.as_ref().ok_or_else(|| {
            errors::Error::provider_error(
                "github",
                "OAuth is not configured",
            )
        })?;

        let client = reqwest::Client::new();

        let token_request = serde_json::json!({
            "client_id": oauth.client_id,
            "client_secret": oauth.client_secret,
            "code": code,
            "redirect_uri": oauth.redirect_uri,
        });

        tracing::debug!("Exchanging code for token");

        let response = client
            .post(GITHUB_TOKEN_URL)
            .header("Accept", "application/json")
            .json(&token_request)
            .send()
            .await
            .map_err(|e| {
                errors::Error::http_request_error(e.to_string())
            })?;

        if !response.status().is_success() {
            let error_text = response.text().await.map_err(|e| {
                errors::Error::http_request_error(e.to_string())
            })?;
            return Err(errors::Error::provider_error(
                "github",
                format!("Failed to exchange token: {error_text}"),
            ));
        }

        let response_text = response.text().await.map_err(|e| {
            errors::Error::http_request_error(e.to_string())
        })?;

        // Check for error response
        if let Ok(error) =
            serde_json::from_str::<GitHubErrorResponse>(&response_text)
        {
            if error.error == "bad_verification_code" {
                return Err(errors::Error::provider_error(
                    "github",
                    format!(
                        "Invalid authorization code: {}",
                        error.error_description.unwrap_or_default()
                    ),
                ));
            }
            return Err(errors::Error::provider_error(
                "github",
                format!(
                    "OAuth error: {} - {}",
                    error.error,
                    error.error_description.unwrap_or_default()
                ),
            ));
        }

        let token: GitHubTokenResponse =
            serde_json::from_str(&response_text).map_err(|e| {
                errors::Error::provider_error(
                    "github",
                    format!("Failed to parse token response: {e}"),
                )
            })?;

        // Get user info to populate provider_user_id
        let user = self.get_user(&token.access_token).await?;

        // GitHub tokens don't expire by default (unless using GitHub Apps with expiring tokens)
        // Set a long expiry time for non-expiring tokens
        let expires_in = token.expires_in.unwrap_or(365 * 24 * 60 * 60); // 1 year default

        Ok(OAuthToken {
            access_token: token.access_token,
            token_type: token.token_type,
            expires_in,
            refresh_token: token.refresh_token,
            scope: token.scope,
            provider_user_id: user.login,
        })
    }

    async fn refresh_token(
        &self,
        refresh_token: &str,
    ) -> errors::Result<OAuthToken> {
        let oauth = self.oauth.as_ref().ok_or_else(|| {
            errors::Error::provider_error(
                "github",
                "OAuth is not configured",
            )
        })?;

        let client = reqwest::Client::new();

        let token_request = serde_json::json!({
            "client_id": oauth.client_id,
            "client_secret": oauth.client_secret,
            "refresh_token": refresh_token,
            "grant_type": "refresh_token",
        });

        let response = client
            .post(GITHUB_TOKEN_URL)
            .header("Accept", "application/json")
            .json(&token_request)
            .send()
            .await
            .map_err(|e| {
                errors::Error::http_request_error(e.to_string())
            })?;

        if !response.status().is_success() {
            let error_text = response.text().await.map_err(|e| {
                errors::Error::http_request_error(e.to_string())
            })?;
            return Err(errors::Error::provider_error(
                "github",
                format!("Failed to refresh token: {error_text}"),
            ));
        }

        let token: GitHubTokenResponse =
            response.json().await.map_err(|e| {
                errors::Error::provider_error(
                    "github",
                    format!("Failed to parse token response: {e}"),
                )
            })?;

        // Get user info
        let user = self.get_user(&token.access_token).await?;

        let expires_in = token.expires_in.unwrap_or(365 * 24 * 60 * 60);

        Ok(OAuthToken {
            access_token: token.access_token,
            token_type: token.token_type,
            expires_in,
            refresh_token: token.refresh_token,
            scope: token.scope,
            provider_user_id: user.login,
        })
    }
}

impl GitHub {
    /// Get authenticated user information.
    async fn get_user(
        &self,
        access_token: &str,
    ) -> errors::Result<GitHubUser> {
        let client = reqwest::Client::new();

        let response = client
            .get(GITHUB_USER_URL)
            .header("Authorization", format!("Bearer {access_token}"))
            .header("User-Agent", "library-api")
            .header("Accept", "application/vnd.github.v3+json")
            .send()
            .await
            .map_err(|e| {
                errors::Error::http_request_error(e.to_string())
            })?;

        if !response.status().is_success() {
            let error_text = response.text().await.map_err(|e| {
                errors::Error::http_request_error(e.to_string())
            })?;
            return Err(errors::Error::provider_error(
                "github",
                format!("Failed to get user info: {error_text}"),
            ));
        }

        let user: GitHubUser = response.json().await.map_err(|e| {
            errors::Error::provider_error(
                "github",
                format!("Failed to parse user response: {e}"),
            )
        })?;

        Ok(user)
    }

    /// List repositories accessible to the authenticated user.
    ///
    /// # Arguments
    ///
    /// * `access_token` - OAuth access token
    /// * `search` - Optional search query to filter repositories by name
    /// * `per_page` - Number of results per page (max 100)
    /// * `page` - Page number (1-indexed)
    ///
    /// # Example
    ///
    /// ```ignore
    /// let repos = github.list_repositories(
    ///     &access_token,
    ///     Some("my-repo"),
    ///     30,
    ///     1,
    /// ).await?;
    /// ```
    pub async fn list_repositories(
        access_token: &str,
        search: Option<&str>,
        per_page: u32,
        page: u32,
    ) -> errors::Result<Vec<GitHubRepository>> {
        let client = reqwest::Client::new();

        let per_page = per_page.min(100);
        let page = page.max(1);

        let url = format!(
            "{GITHUB_USER_REPOS_URL}?sort=updated&direction=desc&per_page={per_page}&page={page}"
        );

        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {access_token}"))
            .header("User-Agent", "library-api")
            .header("Accept", "application/vnd.github.v3+json")
            .send()
            .await
            .map_err(|e| {
                errors::Error::http_request_error(e.to_string())
            })?;

        if !response.status().is_success() {
            let error_text = response.text().await.map_err(|e| {
                errors::Error::http_request_error(e.to_string())
            })?;
            return Err(errors::Error::provider_error(
                "github",
                format!("Failed to list repositories: {error_text}"),
            ));
        }

        let repos: Vec<GitHubRepository> =
            response.json().await.map_err(|e| {
                errors::Error::provider_error(
                    "github",
                    format!("Failed to parse repositories response: {e}"),
                )
            })?;

        // Filter by search query if provided
        let repos = if let Some(query) = search {
            let query_lower = query.to_lowercase();
            repos
                .into_iter()
                .filter(|repo| {
                    repo.full_name.to_lowercase().contains(&query_lower)
                        || repo.name.to_lowercase().contains(&query_lower)
                })
                .collect()
        } else {
            repos
        };

        Ok(repos)
    }

    /// List contents of a directory in a repository.
    ///
    /// # Arguments
    ///
    /// * `access_token` - OAuth access token
    /// * `owner` - Repository owner
    /// * `repo` - Repository name
    /// * `path` - Path to the directory (empty string for root)
    /// * `ref_name` - Branch/tag/commit (optional, defaults to default branch)
    ///
    /// # Example
    ///
    /// ```ignore
    /// let contents = GitHub::list_directory_contents(
    ///     &access_token,
    ///     "owner",
    ///     "repo",
    ///     "docs/articles",
    ///     Some("main"),
    /// ).await?;
    /// ```
    pub async fn list_directory_contents(
        access_token: &str,
        owner: &str,
        repo: &str,
        path: &str,
        ref_name: Option<&str>,
    ) -> errors::Result<GitHubDirectoryListing> {
        let client = reqwest::Client::new();

        let path = path.trim_start_matches('/');
        let mut url =
            format!("{GITHUB_REPOS_URL}/{owner}/{repo}/contents/{path}");

        if let Some(ref_name) = ref_name {
            url = format!("{url}?ref={ref_name}");
        }

        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {access_token}"))
            .header("User-Agent", "library-api")
            .header("Accept", "application/vnd.github.v3+json")
            .send()
            .await
            .map_err(|e| {
                errors::Error::http_request_error(e.to_string())
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.map_err(|e| {
                errors::Error::http_request_error(e.to_string())
            })?;

            if status.as_u16() == 404 {
                return Err(errors::Error::not_found(format!(
                    "Path not found: {repo}/{path}"
                )));
            }

            return Err(errors::Error::provider_error(
                "github",
                format!("Failed to list directory: {error_text}"),
            ));
        }

        // Check if response is a single file or a directory listing
        let response_text = response.text().await.map_err(|e| {
            errors::Error::http_request_error(e.to_string())
        })?;

        // Try to parse as array first (directory listing)
        if let Ok(contents) =
            serde_json::from_str::<Vec<GitHubContent>>(&response_text)
        {
            return Ok(GitHubDirectoryListing {
                contents,
                truncated: false,
            });
        }

        // If it's a single file, wrap it in a vec
        if let Ok(content) =
            serde_json::from_str::<GitHubContent>(&response_text)
        {
            return Ok(GitHubDirectoryListing {
                contents: vec![content],
                truncated: false,
            });
        }

        Err(errors::Error::provider_error(
            "github",
            format!("Failed to parse directory response: {response_text}"),
        ))
    }

    /// Get the content of a file in a repository.
    ///
    /// # Arguments
    ///
    /// * `access_token` - OAuth access token
    /// * `owner` - Repository owner
    /// * `repo` - Repository name
    /// * `path` - Path to the file
    /// * `ref_name` - Branch/tag/commit (optional)
    ///
    /// # Returns
    ///
    /// The file content as a string (decoded from base64).
    pub async fn get_file_content(
        access_token: &str,
        owner: &str,
        repo: &str,
        path: &str,
        ref_name: Option<&str>,
    ) -> errors::Result<String> {
        let client = reqwest::Client::new();

        let path = path.trim_start_matches('/');
        let mut url =
            format!("{GITHUB_REPOS_URL}/{owner}/{repo}/contents/{path}");

        if let Some(ref_name) = ref_name {
            url = format!("{url}?ref={ref_name}");
        }

        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {access_token}"))
            .header("User-Agent", "library-api")
            .header("Accept", "application/vnd.github.v3+json")
            .send()
            .await
            .map_err(|e| {
                errors::Error::http_request_error(e.to_string())
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.map_err(|e| {
                errors::Error::http_request_error(e.to_string())
            })?;

            if status.as_u16() == 404 {
                return Err(errors::Error::not_found(format!(
                    "File not found: {repo}/{path}"
                )));
            }

            return Err(errors::Error::provider_error(
                "github",
                format!("Failed to get file: {error_text}"),
            ));
        }

        let content: GitHubContent =
            response.json().await.map_err(|e| {
                errors::Error::provider_error(
                    "github",
                    format!("Failed to parse file response: {e}"),
                )
            })?;

        // Check if it's a file
        if content.content_type != "file" {
            return Err(errors::Error::bad_request(format!(
                "Path is not a file: {path}"
            )));
        }

        // Decode base64 content
        let encoded = content.content.ok_or_else(|| {
            errors::Error::provider_error(
                "github",
                "File content is empty or too large (use blob API for large files)",
            )
        })?;

        // GitHub returns base64 with newlines, remove them
        let encoded = encoded.replace('\n', "");

        use base64::Engine;
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(&encoded)
            .map_err(|e| {
                errors::Error::provider_error(
                    "github",
                    format!("Failed to decode base64 content: {e}"),
                )
            })?;

        String::from_utf8(decoded).map_err(|e| {
            errors::Error::provider_error(
                "github",
                format!("File is not valid UTF-8: {e}"),
            )
        })
    }

    /// Get the raw content of a file using the download URL.
    ///
    /// This is more efficient for larger files as it doesn't require base64 decoding.
    pub async fn get_raw_file_content(
        access_token: &str,
        owner: &str,
        repo: &str,
        path: &str,
        ref_name: Option<&str>,
    ) -> errors::Result<String> {
        let client = reqwest::Client::new();

        let path = path.trim_start_matches('/');
        let ref_part = ref_name.unwrap_or("HEAD");
        let url = format!(
            "https://raw.githubusercontent.com/{owner}/{repo}/{ref_part}/{path}"
        );

        let response = client
            .get(&url)
            .header("Authorization", format!("token {access_token}"))
            .header("User-Agent", "library-api")
            .send()
            .await
            .map_err(|e| {
                errors::Error::http_request_error(e.to_string())
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.map_err(|e| {
                errors::Error::http_request_error(e.to_string())
            })?;

            if status.as_u16() == 404 {
                return Err(errors::Error::not_found(format!(
                    "File not found: {repo}/{path}"
                )));
            }

            return Err(errors::Error::provider_error(
                "github",
                format!("Failed to get raw file: {error_text}"),
            ));
        }

        response
            .text()
            .await
            .map_err(|e| errors::Error::http_request_error(e.to_string()))
    }
}
