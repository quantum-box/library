//! GitHub sync provider implementation.
//!
//! Uses the GitHub Contents API to sync files to repositories.

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;

use outbound_sync_domain::{
    ProviderType, RemoteData, SyncAuth, SyncPayload, SyncProvider,
    SyncResult, SyncTarget,
};

const GITHUB_API_BASE: &str = "https://api.github.com";
const USER_AGENT: &str = "database-sync/0.1.0";

/// Rate limiting configuration
const MAX_RETRIES: u32 = 3;
const INITIAL_BACKOFF_MS: u64 = 1000;
const MAX_BACKOFF_MS: u64 = 30000;

/// Check if response indicates rate limiting and extract retry-after if available
fn is_rate_limited(response: &reqwest::Response) -> Option<Duration> {
    if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS
        || response.status() == reqwest::StatusCode::FORBIDDEN
    {
        // Check for rate limit headers
        if let Some(remaining) = response
            .headers()
            .get("x-ratelimit-remaining")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u32>().ok())
        {
            if remaining == 0 {
                // Check retry-after or reset time
                if let Some(retry_after) = response
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse::<u64>().ok())
                {
                    return Some(Duration::from_secs(retry_after));
                }

                // Use reset timestamp if available
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

                // Default wait time if rate limited but no header
                return Some(Duration::from_secs(60));
            }
        }

        // 403 might be rate limit without specific headers
        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Some(Duration::from_secs(60));
        }
    }
    None
}

/// Calculate exponential backoff delay
fn calculate_backoff(attempt: u32) -> Duration {
    let delay_ms = INITIAL_BACKOFF_MS * 2u64.pow(attempt);
    Duration::from_millis(delay_ms.min(MAX_BACKOFF_MS))
}

/// GitHub sync provider.
///
/// Implements file synchronization using the GitHub Contents API.
///
/// # Example
///
/// ```ignore
/// let provider = GitHubSyncProvider::new();
/// let auth = SyncAuth::oauth("ghp_xxx");
/// let target = SyncTarget::git("owner/repo", "docs/article.md");
/// let payload = SyncPayload::markdown("# Hello");
///
/// let result = provider.put_data(&auth, &target, &payload).await?;
/// ```
#[derive(Debug)]
pub struct GitHubSyncProvider {
    client: reqwest::Client,
}

impl GitHubSyncProvider {
    /// Create a new GitHub sync provider
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    /// Build the API URL for contents
    fn contents_url(&self, repo: &str, path: &str) -> String {
        format!("{GITHUB_API_BASE}/repos/{repo}/contents/{path}")
    }

    /// Build request headers
    fn build_headers(&self, auth: &SyncAuth) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", auth.access_token)
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

    /// Execute a GET request with retry logic for rate limiting
    async fn get_with_retry(
        &self,
        url: &str,
        auth: &SyncAuth,
    ) -> errors::Result<reqwest::Response> {
        let mut last_error = None;

        for attempt in 0..MAX_RETRIES {
            let response = self
                .client
                .get(url)
                .headers(self.build_headers(auth))
                .send()
                .await
                .map_err(|e| {
                    errors::Error::http_request_error(e.to_string())
                })?;

            // Check for rate limiting
            if let Some(wait_duration) = is_rate_limited(&response) {
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

            // Check for transient errors that should be retried
            if response.status().is_server_error() {
                let backoff = calculate_backoff(attempt);
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

        Err(errors::Error::provider_error(
            "github",
            last_error
                .unwrap_or_else(|| "Max retries exceeded".to_string()),
        ))
    }

    /// Execute a PUT request with retry logic for rate limiting
    async fn put_with_retry<T: Serialize>(
        &self,
        url: &str,
        auth: &SyncAuth,
        body: &T,
    ) -> errors::Result<reqwest::Response> {
        let mut last_error = None;

        for attempt in 0..MAX_RETRIES {
            let response = self
                .client
                .put(url)
                .headers(self.build_headers(auth))
                .json(body)
                .send()
                .await
                .map_err(|e| {
                    errors::Error::http_request_error(e.to_string())
                })?;

            // Check for rate limiting
            if let Some(wait_duration) = is_rate_limited(&response) {
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

            // Check for transient errors
            if response.status().is_server_error() {
                let backoff = calculate_backoff(attempt);
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

        Err(errors::Error::provider_error(
            "github",
            last_error
                .unwrap_or_else(|| "Max retries exceeded".to_string()),
        ))
    }

    /// Execute a DELETE request with retry logic for rate limiting
    async fn delete_with_retry<T: Serialize>(
        &self,
        url: &str,
        auth: &SyncAuth,
        body: &T,
    ) -> errors::Result<reqwest::Response> {
        let mut last_error = None;

        for attempt in 0..MAX_RETRIES {
            let response = self
                .client
                .delete(url)
                .headers(self.build_headers(auth))
                .json(body)
                .send()
                .await
                .map_err(|e| {
                    errors::Error::http_request_error(e.to_string())
                })?;

            // Check for rate limiting
            if let Some(wait_duration) = is_rate_limited(&response) {
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

            // Check for transient errors
            if response.status().is_server_error() {
                let backoff = calculate_backoff(attempt);
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

        Err(errors::Error::provider_error(
            "github",
            last_error
                .unwrap_or_else(|| "Max retries exceeded".to_string()),
        ))
    }
}

impl Default for GitHubSyncProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GitHubContentsResponse {
    sha: String,
    content: Option<String>,
    encoding: Option<String>,
    size: Option<u64>,
    html_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitHubCreateUpdateResponse {
    content: Option<GitHubContentsResponse>,
    commit: GitHubCommit,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GitHubCommit {
    sha: String,
    html_url: Option<String>,
}

#[derive(Debug, Serialize)]
struct GitHubCreateUpdateRequest {
    message: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    sha: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    branch: Option<String>,
}

#[derive(Debug, Serialize)]
struct GitHubDeleteRequest {
    message: String,
    sha: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    branch: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GitHubErrorResponse {
    message: String,
    documentation_url: Option<String>,
}

#[async_trait]
impl SyncProvider for GitHubSyncProvider {
    fn provider_name(&self) -> &'static str {
        "github"
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::GitRepository
    }

    #[tracing::instrument(skip(self, auth))]
    async fn get_data(
        &self,
        auth: &SyncAuth,
        target: &SyncTarget,
    ) -> errors::Result<Option<RemoteData>> {
        let path = target.resource.as_ref().ok_or_else(|| {
            errors::Error::invalid(
                "Resource path is required for GitHub provider",
            )
        })?;

        let mut url = self.contents_url(&target.container, path);

        // Add branch/ref if specified
        if let Some(ref branch) = target.version {
            url.push_str(&format!("?ref={branch}"));
        }

        tracing::debug!(url = %url, "Fetching file from GitHub");

        let response = self.get_with_retry(&url, auth).await?;

        let status = response.status();

        if status == reqwest::StatusCode::NOT_FOUND {
            tracing::debug!("File not found at {}", url);
            return Ok(None);
        }

        if !status.is_success() {
            let error: GitHubErrorResponse =
                response.json().await.unwrap_or(GitHubErrorResponse {
                    message: "Unknown error".to_string(),
                    documentation_url: None,
                });
            return Err(errors::Error::provider_error(
                "github",
                format!("GitHub API error: {}", error.message),
            ));
        }

        let data: GitHubContentsResponse =
            response.json().await.map_err(|e| {
                errors::Error::provider_error(
                    "github",
                    format!("Failed to parse response: {e}"),
                )
            })?;

        // Decode base64 content
        let content = if let Some(encoded) = data.content {
            // Remove newlines that GitHub adds
            let cleaned = encoded.replace('\n', "");
            let decoded = BASE64.decode(&cleaned).map_err(|e| {
                errors::Error::provider_error(
                    "github",
                    format!("Failed to decode content: {e}"),
                )
            })?;
            String::from_utf8(decoded).map_err(|e| {
                errors::Error::provider_error(
                    "github",
                    format!("Content is not valid UTF-8: {e}"),
                )
            })?
        } else {
            String::new()
        };

        Ok(Some(RemoteData {
            id: data.sha,
            content,
            content_type: Some("text/plain".to_string()),
            size: data.size,
            updated_at: None,
        }))
    }

    #[tracing::instrument(skip(self, auth, payload))]
    async fn put_data(
        &self,
        auth: &SyncAuth,
        target: &SyncTarget,
        payload: &SyncPayload,
    ) -> errors::Result<SyncResult> {
        let path = target.resource.as_ref().ok_or_else(|| {
            errors::Error::invalid(
                "Resource path is required for GitHub provider",
            )
        })?;

        // Get existing file SHA (required for updates)
        let existing = self.get_data(auth, target).await?;

        let url = self.contents_url(&target.container, path);

        // Prepare message
        let message = payload
            .metadata
            .message
            .clone()
            .unwrap_or_else(|| format!("Update {path}"));

        // Encode content to base64
        let content_base64 = BASE64.encode(&payload.content);

        let request = GitHubCreateUpdateRequest {
            message,
            content: content_base64,
            sha: existing.map(|e| e.id),
            branch: target.version.clone(),
        };

        tracing::debug!(url = %url, has_existing = request.sha.is_some(), "Creating/updating file on GitHub");

        let response = self.put_with_retry(&url, auth, &request).await?;

        let status = response.status();

        if !status.is_success() {
            let error: GitHubErrorResponse =
                response.json().await.unwrap_or(GitHubErrorResponse {
                    message: "Unknown error".to_string(),
                    documentation_url: None,
                });
            return Err(errors::Error::provider_error(
                "github",
                format!("GitHub API error: {}", error.message),
            ));
        }

        let data: GitHubCreateUpdateResponse =
            response.json().await.map_err(|e| {
                errors::Error::provider_error(
                    "github",
                    format!("Failed to parse response: {e}"),
                )
            })?;

        tracing::info!(
            commit_sha = %data.commit.sha,
            "File successfully synced to GitHub"
        );

        Ok(SyncResult {
            success: true,
            result_id: Some(data.commit.sha),
            url: data.content.and_then(|c| c.html_url),
            diff: None,
        })
    }

    #[tracing::instrument(skip(self, auth))]
    async fn delete_data(
        &self,
        auth: &SyncAuth,
        target: &SyncTarget,
    ) -> errors::Result<SyncResult> {
        let path = target.resource.as_ref().ok_or_else(|| {
            errors::Error::invalid(
                "Resource path is required for GitHub provider",
            )
        })?;

        // Get existing file SHA (required for deletion)
        let existing = self
            .get_data(auth, target)
            .await?
            .ok_or_else(|| errors::Error::not_found("File not found"))?;

        let url = self.contents_url(&target.container, path);

        let request = GitHubDeleteRequest {
            message: format!("Delete {path}"),
            sha: existing.id,
            branch: target.version.clone(),
        };

        tracing::debug!(url = %url, "Deleting file from GitHub");

        let response = self.delete_with_retry(&url, auth, &request).await?;

        let status = response.status();

        if !status.is_success() {
            let error: GitHubErrorResponse =
                response.json().await.unwrap_or(GitHubErrorResponse {
                    message: "Unknown error".to_string(),
                    documentation_url: None,
                });
            return Err(errors::Error::provider_error(
                "github",
                format!("GitHub API error: {}", error.message),
            ));
        }

        let data: GitHubCreateUpdateResponse =
            response.json().await.map_err(|e| {
                errors::Error::provider_error(
                    "github",
                    format!("Failed to parse response: {e}"),
                )
            })?;

        tracing::info!(
            commit_sha = %data.commit.sha,
            "File successfully deleted from GitHub"
        );

        Ok(SyncResult {
            success: true,
            result_id: Some(data.commit.sha),
            url: None,
            diff: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contents_url() {
        let provider = GitHubSyncProvider::new();
        let url = provider.contents_url("owner/repo", "path/to/file.md");
        assert_eq!(
            url,
            "https://api.github.com/repos/owner/repo/contents/path/to/file.md"
        );
    }
}
