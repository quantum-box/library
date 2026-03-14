//! GitHub API client implementation.
//!
//! This module provides two implementations of the `GitHubClient` trait:
//!
//! 1. `GitHubApiClient` - Uses a pre-configured access token (for testing/static configs)
//! 2. `OAuthGitHubClient` - Dynamically fetches tokens via `OAuthTokenProvider`
//!
//! # OAuth Integration
//!
//! For production use, prefer `OAuthGitHubClient` which integrates with the
//! AuthApp OAuth system to automatically manage token retrieval and refresh.
//!
//! ```ignore
//! use inbound_sync::sdk::{AuthAppTokenProvider, OAuthTokenProvider};
//! use inbound_sync::providers::github::OAuthGitHubClient;
//!
//! let token_provider = Arc::new(AuthAppTokenProvider::new(auth_app));
//! let client = Arc::new(OAuthGitHubClient::new(token_provider));
//! ```

use std::sync::Arc;

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde::Deserialize;
use std::time::Duration;
use tokio::time::sleep;
use value_object::TenantId;

use super::event_processor::{GitHubClient, PullRequestFile};
use super::payload::glob_match;
use crate::sdk::OAuthTokenProvider;

const GITHUB_API_BASE: &str = "https://api.github.com";
const USER_AGENT: &str = "inbound-sync/0.1.0";

/// Rate limiting configuration
const MAX_RETRIES: u32 = 3;
const INITIAL_BACKOFF_MS: u64 = 1000;
const MAX_BACKOFF_MS: u64 = 30000;

/// GitHub API client implementation.
///
/// This implementation uses a pre-configured access token and ignores the
/// tenant_id parameter. Useful for testing or simple static configurations.
#[derive(Debug)]
pub struct GitHubApiClient {
    client: reqwest::Client,
    access_token: String,
}

impl GitHubApiClient {
    /// Create a new GitHub API client.
    pub fn new(access_token: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            access_token,
        }
    }

    /// Build request headers with a specific token.
    fn build_headers_with_token(
        access_token: &str,
    ) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {access_token}")
                .parse()
                .expect("Invalid token"),
        );
        headers.insert(
            reqwest::header::USER_AGENT,
            USER_AGENT.parse().expect("Invalid user agent"),
        );
        headers.insert(
            reqwest::header::ACCEPT,
            "application/vnd.github.v3+json"
                .parse()
                .expect("Invalid accept header"),
        );
        headers
    }

    /// Check if response indicates rate limiting and extract retry-after if
    /// available.
    fn check_rate_limit(response: &reqwest::Response) -> Option<Duration> {
        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS
            || response.status() == reqwest::StatusCode::FORBIDDEN
        {
            if let Some(remaining) = response
                .headers()
                .get("x-ratelimit-remaining")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u32>().ok())
            {
                if remaining == 0 {
                    if let Some(retry_after) = response
                        .headers()
                        .get("retry-after")
                        .and_then(|v| v.to_str().ok())
                        .and_then(|v| v.parse::<u64>().ok())
                    {
                        return Some(Duration::from_secs(retry_after));
                    }

                    if let Some(reset) = response
                        .headers()
                        .get("x-ratelimit-reset")
                        .and_then(|v| v.to_str().ok())
                        .and_then(|v| v.parse::<u64>().ok())
                    {
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs();
                        if reset > now {
                            return Some(Duration::from_secs(reset - now));
                        }
                    }

                    return Some(Duration::from_secs(60));
                }
            }

            if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                return Some(Duration::from_secs(60));
            }
        }
        None
    }

    /// Calculate exponential backoff delay.
    pub fn calculate_backoff(attempt: u32) -> Duration {
        let delay_ms = INITIAL_BACKOFF_MS * 2u64.pow(attempt);
        Duration::from_millis(delay_ms.min(MAX_BACKOFF_MS))
    }

    /// Execute a GET request with retry logic for rate limiting.
    async fn get_with_retry(
        &self,
        url: &str,
    ) -> errors::Result<reqwest::Response> {
        Self::get_with_retry_token(&self.client, url, &self.access_token)
            .await
    }

    /// Execute a GET request with retry logic using a specific token.
    async fn get_with_retry_token(
        client: &reqwest::Client,
        url: &str,
        access_token: &str,
    ) -> errors::Result<reqwest::Response> {
        let mut last_error = None;

        for attempt in 0..MAX_RETRIES {
            let response = client
                .get(url)
                .headers(Self::build_headers_with_token(access_token))
                .send()
                .await
                .map_err(|e| {
                    errors::Error::internal_server_error(format!(
                        "HTTP request failed: {e}"
                    ))
                })?;

            if let Some(wait_duration) = Self::check_rate_limit(&response) {
                let wait_secs =
                    wait_duration.as_secs().min(MAX_BACKOFF_MS / 1000);
                tracing::warn!(
                    attempt = attempt + 1,
                    wait_secs = wait_secs,
                    "GitHub API rate limited, waiting before retry"
                );
                sleep(Duration::from_secs(wait_secs)).await;
                continue;
            }

            if response.status().is_server_error() {
                let backoff = Self::calculate_backoff(attempt);
                tracing::warn!(
                    attempt = attempt + 1,
                    status = %response.status(),
                    backoff_ms = backoff.as_millis() as u64,
                    "GitHub API server error, retrying with backoff"
                );
                last_error =
                    Some(format!("Server error: {}", response.status()));
                sleep(backoff).await;
                continue;
            }

            return Ok(response);
        }

        Err(errors::Error::internal_server_error(
            last_error
                .unwrap_or_else(|| "Max retries exceeded".to_string()),
        ))
    }
}

#[derive(Debug, Deserialize)]
struct GitHubContentsResponse {
    #[allow(dead_code)]
    sha: String,
    content: Option<String>,
    #[allow(dead_code)]
    encoding: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitHubErrorResponse {
    message: String,
    #[allow(dead_code)]
    documentation_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitHubPullRequestFile {
    filename: String,
    status: String,
    #[serde(default)]
    additions: u32,
    #[serde(default)]
    deletions: u32,
    #[serde(default)]
    changes: u32,
    previous_filename: Option<String>,
}

/// Static token implementation of GitHubClient.
///
/// This implementation uses a pre-configured access token and ignores the
/// tenant_id parameter. Useful for testing or simple static configurations.
#[async_trait]
impl GitHubClient for GitHubApiClient {
    async fn get_file_content(
        &self,
        _tenant_id: &TenantId,
        repo: &str,
        path: &str,
        branch: &str,
    ) -> errors::Result<String> {
        let url = format!(
            "{GITHUB_API_BASE}/repos/{repo}/contents/{path}?ref={branch}"
        );

        tracing::debug!(url = %url, "Fetching file from GitHub");

        let response = self.get_with_retry(&url).await?;
        let status = response.status();

        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(errors::Error::not_found(format!(
                "File not found: {repo}/{path}"
            )));
        }

        if !status.is_success() {
            let error: GitHubErrorResponse =
                response.json().await.unwrap_or(GitHubErrorResponse {
                    message: "Unknown error".to_string(),
                    documentation_url: None,
                });
            return Err(errors::Error::internal_server_error(format!(
                "GitHub API error: {}",
                error.message
            )));
        }

        let data: GitHubContentsResponse =
            response.json().await.map_err(|e| {
                errors::Error::internal_server_error(format!(
                    "Failed to parse response: {e}"
                ))
            })?;

        // Decode base64 content
        let content = if let Some(encoded) = data.content {
            // Remove newlines that GitHub adds
            let cleaned = encoded.replace('\n', "");
            let decoded = BASE64.decode(&cleaned).map_err(|e| {
                errors::Error::internal_server_error(format!(
                    "Failed to decode content: {e}"
                ))
            })?;
            String::from_utf8(decoded).map_err(|e| {
                errors::Error::internal_server_error(format!(
                    "Content is not valid UTF-8: {e}"
                ))
            })?
        } else {
            String::new()
        };

        Ok(content)
    }

    async fn get_pr_files(
        &self,
        _tenant_id: &TenantId,
        repo: &str,
        pr_number: u64,
    ) -> errors::Result<Vec<PullRequestFile>> {
        let url = format!(
            "{GITHUB_API_BASE}/repos/{repo}/pulls/{pr_number}/files?per_page=100"
        );

        tracing::debug!(url = %url, "Fetching PR files from GitHub");

        let response = self.get_with_retry(&url).await?;
        let status = response.status();

        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(errors::Error::not_found(format!(
                "Pull request not found: {repo}#{pr_number}"
            )));
        }

        if !status.is_success() {
            let error: GitHubErrorResponse =
                response.json().await.unwrap_or(GitHubErrorResponse {
                    message: "Unknown error".to_string(),
                    documentation_url: None,
                });
            return Err(errors::Error::internal_server_error(format!(
                "GitHub API error: {}",
                error.message
            )));
        }

        let files: Vec<GitHubPullRequestFile> =
            response.json().await.map_err(|e| {
                errors::Error::internal_server_error(format!(
                    "Failed to parse PR files response: {e}"
                ))
            })?;

        Ok(files
            .into_iter()
            .map(|f| PullRequestFile {
                filename: f.filename,
                status: f.status,
                additions: f.additions,
                deletions: f.deletions,
                changes: f.changes,
                previous_filename: f.previous_filename,
            })
            .collect())
    }

    async fn list_repository_contents(
        &self,
        tenant_id: &TenantId,
        repo: &str,
        branch: &str,
        path_pattern: Option<&str>,
    ) -> errors::Result<Vec<super::event_processor::RepositoryContent>>
    {
        use super::event_processor::RepositoryContent;

        // Get repository tree (recursive)
        let url = format!(
            "{GITHUB_API_BASE}/repos/{repo}/git/trees/{branch}?recursive=1"
        );

        tracing::debug!(url = %url, "Fetching repository tree");

        let response = self.get_with_retry(&url).await?;

        #[derive(Deserialize)]
        struct TreeResponse {
            tree: Vec<TreeItem>,
        }

        #[derive(Deserialize)]
        struct TreeItem {
            path: String,
            #[serde(rename = "type")]
            item_type: String,
            sha: String,
            size: Option<usize>,
        }

        let tree: TreeResponse = response.json().await.map_err(|e| {
            errors::Error::internal_server_error(format!(
                "Parse error: {e}"
            ))
        })?;

        let mut results = Vec::new();
        for item in tree.tree {
            if item.item_type != "blob" {
                continue;
            }

            if let Some(pattern) = path_pattern {
                if !glob_match(pattern, &item.path) {
                    continue;
                }
            }

            match self
                .get_file_content(tenant_id, repo, &item.path, branch)
                .await
            {
                Ok(content) => {
                    results.push(RepositoryContent {
                        path: item.path,
                        content: Some(content),
                        sha: item.sha,
                        size: item.size.unwrap_or(0),
                    });
                }
                Err(_) => continue,
            }
        }

        Ok(results)
    }
}

/// OAuth-integrated GitHub API client.
///
/// This implementation dynamically fetches OAuth tokens from the `OAuthTokenProvider`
/// for each API call, supporting multi-tenant scenarios where each tenant has
/// their own GitHub OAuth connection.
///
/// # Example
///
/// ```ignore
/// use inbound_sync::sdk::{AuthAppTokenProvider, OAuthTokenProvider};
/// use inbound_sync::providers::github::OAuthGitHubClient;
///
/// let auth_app = /* ... */;
/// let token_provider = Arc::new(AuthAppTokenProvider::new(auth_app));
/// let client = OAuthGitHubClient::new(token_provider);
///
/// // The client will automatically fetch the OAuth token for this tenant
/// let content = client.get_file_content(&tenant_id, "owner/repo", "file.md", "main").await?;
/// ```
#[derive(Debug)]
pub struct OAuthGitHubClient {
    client: reqwest::Client,
    token_provider: Arc<dyn OAuthTokenProvider>,
}

impl OAuthGitHubClient {
    /// Create a new OAuth-integrated GitHub client.
    pub fn new(token_provider: Arc<dyn OAuthTokenProvider>) -> Self {
        Self {
            client: reqwest::Client::new(),
            token_provider,
        }
    }

    /// Get a valid access token for the tenant.
    async fn get_access_token(
        &self,
        tenant_id: &TenantId,
    ) -> errors::Result<String> {
        let token = self
            .token_provider
            .get_token(tenant_id, "github")
            .await?
            .ok_or_else(|| {
                errors::Error::unauthorized(
                    "GitHub is not connected for this tenant. Please complete \
                     OAuth authorization first.",
                )
            })?;

        if token.is_expired() {
            return Err(errors::Error::unauthorized(
                "GitHub OAuth token has expired. Please reconnect.",
            ));
        }

        Ok(token.access_token)
    }
}

#[async_trait]
impl GitHubClient for OAuthGitHubClient {
    async fn get_file_content(
        &self,
        tenant_id: &TenantId,
        repo: &str,
        path: &str,
        branch: &str,
    ) -> errors::Result<String> {
        let access_token = self.get_access_token(tenant_id).await?;
        let url = format!(
            "{GITHUB_API_BASE}/repos/{repo}/contents/{path}?ref={branch}"
        );

        tracing::debug!(
            url = %url,
            tenant_id = %tenant_id,
            "Fetching file from GitHub with OAuth token"
        );

        let response = GitHubApiClient::get_with_retry_token(
            &self.client,
            &url,
            &access_token,
        )
        .await?;
        let status = response.status();

        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(errors::Error::not_found(format!(
                "File not found: {repo}/{path}"
            )));
        }

        if !status.is_success() {
            let error: GitHubErrorResponse =
                response.json().await.unwrap_or(GitHubErrorResponse {
                    message: "Unknown error".to_string(),
                    documentation_url: None,
                });
            return Err(errors::Error::internal_server_error(format!(
                "GitHub API error: {}",
                error.message
            )));
        }

        let data: GitHubContentsResponse =
            response.json().await.map_err(|e| {
                errors::Error::internal_server_error(format!(
                    "Failed to parse response: {e}"
                ))
            })?;

        // Decode base64 content
        let content = if let Some(encoded) = data.content {
            let cleaned = encoded.replace('\n', "");
            let decoded = BASE64.decode(&cleaned).map_err(|e| {
                errors::Error::internal_server_error(format!(
                    "Failed to decode content: {e}"
                ))
            })?;
            String::from_utf8(decoded).map_err(|e| {
                errors::Error::internal_server_error(format!(
                    "Content is not valid UTF-8: {e}"
                ))
            })?
        } else {
            String::new()
        };

        Ok(content)
    }

    async fn get_pr_files(
        &self,
        tenant_id: &TenantId,
        repo: &str,
        pr_number: u64,
    ) -> errors::Result<Vec<PullRequestFile>> {
        let access_token = self.get_access_token(tenant_id).await?;
        let url = format!(
            "{GITHUB_API_BASE}/repos/{repo}/pulls/{pr_number}/files?per_page=100"
        );

        tracing::debug!(
            url = %url,
            tenant_id = %tenant_id,
            "Fetching PR files from GitHub with OAuth token"
        );

        let response = GitHubApiClient::get_with_retry_token(
            &self.client,
            &url,
            &access_token,
        )
        .await?;
        let status = response.status();

        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(errors::Error::not_found(format!(
                "Pull request not found: {repo}#{pr_number}"
            )));
        }

        if !status.is_success() {
            let error: GitHubErrorResponse =
                response.json().await.unwrap_or(GitHubErrorResponse {
                    message: "Unknown error".to_string(),
                    documentation_url: None,
                });
            return Err(errors::Error::internal_server_error(format!(
                "GitHub API error: {}",
                error.message
            )));
        }

        let files: Vec<GitHubPullRequestFile> =
            response.json().await.map_err(|e| {
                errors::Error::internal_server_error(format!(
                    "Failed to parse PR files response: {e}"
                ))
            })?;

        Ok(files
            .into_iter()
            .map(|f| PullRequestFile {
                filename: f.filename,
                status: f.status,
                additions: f.additions,
                deletions: f.deletions,
                changes: f.changes,
                previous_filename: f.previous_filename,
            })
            .collect())
    }

    async fn list_repository_contents(
        &self,
        tenant_id: &TenantId,
        repo: &str,
        branch: &str,
        path_pattern: Option<&str>,
    ) -> errors::Result<Vec<super::event_processor::RepositoryContent>>
    {
        use super::event_processor::RepositoryContent;

        let token =
            self.token_provider.get_token(tenant_id, "github").await?;
        let access_token = token.as_ref().ok_or_else(|| {
            errors::Error::unauthorized("No GitHub access token available")
        })?;

        let url = format!(
            "{GITHUB_API_BASE}/repos/{repo}/git/trees/{branch}?recursive=1"
        );

        let response = GitHubApiClient::get_with_retry_token(
            &self.client,
            &url,
            &access_token.access_token,
        )
        .await?;

        #[derive(Deserialize)]
        struct TreeResponse {
            tree: Vec<TreeItem>,
        }

        #[derive(Deserialize)]
        struct TreeItem {
            path: String,
            #[serde(rename = "type")]
            item_type: String,
            sha: String,
            size: Option<usize>,
        }

        let tree: TreeResponse = response.json().await.map_err(|e| {
            errors::Error::internal_server_error(format!(
                "Parse error: {e}"
            ))
        })?;

        let mut results = Vec::new();
        for item in tree.tree {
            if item.item_type != "blob" {
                continue;
            }

            if let Some(pattern) = path_pattern {
                if !glob_match(pattern, &item.path) {
                    continue;
                }
            }

            match self
                .get_file_content(tenant_id, repo, &item.path, branch)
                .await
            {
                Ok(content) => {
                    results.push(RepositoryContent {
                        path: item.path,
                        content: Some(content),
                        sha: item.sha,
                        size: item.size.unwrap_or(0),
                    });
                }
                Err(_) => continue,
            }
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = GitHubApiClient::new("test_token".to_string());
        assert!(!client.access_token.is_empty());
    }

    #[test]
    fn test_backoff_calculation() {
        assert_eq!(
            GitHubApiClient::calculate_backoff(0),
            Duration::from_millis(1000)
        );
        assert_eq!(
            GitHubApiClient::calculate_backoff(1),
            Duration::from_millis(2000)
        );
        assert_eq!(
            GitHubApiClient::calculate_backoff(5),
            Duration::from_millis(30000)
        );
    }
}
