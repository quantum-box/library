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
use github_provider::api_url;

const USER_AGENT: &str = "inbound-sync/0.1.0";

// Every value in these URLs (repository and branch from endpoint settings,
// file paths from push payloads, pull request file lists and repository
// trees) is validated and added as its own percent-encoded segment or query
// pair by `github_provider::api_url`, so a path such as `docs/%2e%2e/...`
// cannot leave `/repos/{repo}/contents/` and make the tenant's token read
// another repository.

fn commits_url(repo: &str, branch: &str) -> errors::Result<reqwest::Url> {
    let mut tail = vec!["commits"];
    tail.extend(api_url::ref_segments(branch)?);
    api_url::full_name_url(repo, &tail)
}

fn contents_url(
    repo: &str,
    path: &str,
    branch: &str,
) -> errors::Result<reqwest::Url> {
    // An empty path would name the repository root, never a file.
    api_url::file_path_segments(path)?;
    let (owner, name) = api_url::split_repository(repo)?;
    api_url::contents_url(owner, name, path, Some(branch))
}

fn pr_files_url(
    repo: &str,
    pr_number: u64,
) -> errors::Result<reqwest::Url> {
    let number = pr_number.to_string();
    let mut url =
        api_url::full_name_url(repo, &["pulls", &number, "files"])?;
    url.query_pairs_mut().append_pair("per_page", "100");
    Ok(url)
}

fn tree_url(repo: &str, branch: &str) -> errors::Result<reqwest::Url> {
    let mut tail = vec!["git", "trees"];
    tail.extend(api_url::ref_segments(branch)?);
    let mut url = api_url::full_name_url(repo, &tail)?;
    url.query_pairs_mut().append_pair("recursive", "1");
    Ok(url)
}

/// Whether a repository file path can be fetched through the contents API.
///
/// Paths that fail this are never fetched; callers skip them.
pub(crate) fn is_fetchable_file_path(path: &str) -> bool {
    api_url::is_fetchable_file_path(path)
}

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

#[derive(Deserialize)]
struct GitHubCommitHead {
    sha: String,
}

async fn parse_branch_head(
    response: reqwest::Response,
) -> errors::Result<String> {
    if !response.status().is_success() {
        return Err(errors::Error::invalid(format!(
            "Could not resolve GitHub branch: {}",
            response.status()
        )));
    }
    let head: GitHubCommitHead =
        response.json().await.map_err(|error| {
            errors::Error::internal_server_error(format!(
                "Could not parse GitHub branch: {error}"
            ))
        })?;
    Ok(head.sha)
}

/// Static token implementation of GitHubClient.
///
/// This implementation uses a pre-configured access token and ignores the
/// tenant_id parameter. Useful for testing or simple static configurations.
#[async_trait]
impl GitHubClient for GitHubApiClient {
    async fn get_branch_head(
        &self,
        _tenant_id: &TenantId,
        repo: &str,
        branch: &str,
    ) -> errors::Result<String> {
        let url = commits_url(repo, branch)?;
        parse_branch_head(self.get_with_retry(url.as_str()).await?).await
    }

    async fn get_file_content(
        &self,
        _tenant_id: &TenantId,
        repo: &str,
        path: &str,
        branch: &str,
    ) -> errors::Result<String> {
        let url = contents_url(repo, path, branch)?;

        tracing::debug!(url = %url, "Fetching file from GitHub");

        let response = self.get_with_retry(url.as_str()).await?;
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
        let url = pr_files_url(repo, pr_number)?;

        tracing::debug!(url = %url, "Fetching PR files from GitHub");

        let response = self.get_with_retry(url.as_str()).await?;
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
        let url = tree_url(repo, branch)?;

        tracing::debug!(url = %url, "Fetching repository tree");

        let response = self.get_with_retry(url.as_str()).await?;

        #[derive(Deserialize)]
        struct TreeResponse {
            tree: Vec<TreeItem>,
            #[serde(default)]
            truncated: bool,
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
        if tree.truncated {
            return Err(errors::Error::service_unavailable(
                "GitHub returned a truncated repository tree",
            ));
        }

        let mut results = Vec::new();
        for item in tree.tree {
            if item.item_type != "blob" {
                continue;
            }
            let lower_path = item.path.to_ascii_lowercase();
            if !lower_path.ends_with(".md")
                && !lower_path.ends_with(".markdown")
            {
                continue;
            }

            if let Some(pattern) = path_pattern {
                if !glob_match(pattern, &item.path) {
                    continue;
                }
            }
            if !is_fetchable_file_path(&item.path) {
                tracing::warn!(
                    path = %item.path,
                    "Skipping repository file whose path cannot be fetched safely"
                );
                continue;
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
                Err(error) => return Err(error),
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
    async fn get_branch_head(
        &self,
        tenant_id: &TenantId,
        repo: &str,
        branch: &str,
    ) -> errors::Result<String> {
        let url = commits_url(repo, branch)?;
        let token = self.get_access_token(tenant_id).await?;
        let response = GitHubApiClient::get_with_retry_token(
            &self.client,
            url.as_str(),
            &token,
        )
        .await?;
        parse_branch_head(response).await
    }

    async fn get_file_content(
        &self,
        tenant_id: &TenantId,
        repo: &str,
        path: &str,
        branch: &str,
    ) -> errors::Result<String> {
        let url = contents_url(repo, path, branch)?;
        let access_token = self.get_access_token(tenant_id).await?;

        tracing::debug!(
            url = %url,
            tenant_id = %tenant_id,
            "Fetching file from GitHub with OAuth token"
        );

        let response = GitHubApiClient::get_with_retry_token(
            &self.client,
            url.as_str(),
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
        let url = pr_files_url(repo, pr_number)?;
        let access_token = self.get_access_token(tenant_id).await?;

        tracing::debug!(
            url = %url,
            tenant_id = %tenant_id,
            "Fetching PR files from GitHub with OAuth token"
        );

        let response = GitHubApiClient::get_with_retry_token(
            &self.client,
            url.as_str(),
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

        let url = tree_url(repo, branch)?;

        let response = GitHubApiClient::get_with_retry_token(
            &self.client,
            url.as_str(),
            &access_token.access_token,
        )
        .await?;

        #[derive(Deserialize)]
        struct TreeResponse {
            tree: Vec<TreeItem>,
            #[serde(default)]
            truncated: bool,
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
        if tree.truncated {
            return Err(errors::Error::service_unavailable(
                "GitHub returned a truncated repository tree",
            ));
        }

        let mut results = Vec::new();
        for item in tree.tree {
            if item.item_type != "blob" {
                continue;
            }
            let lower_path = item.path.to_ascii_lowercase();
            if !lower_path.ends_with(".md")
                && !lower_path.ends_with(".markdown")
            {
                continue;
            }

            if let Some(pattern) = path_pattern {
                if !glob_match(pattern, &item.path) {
                    continue;
                }
            }
            if !is_fetchable_file_path(&item.path) {
                tracing::warn!(
                    path = %item.path,
                    "Skipping repository file whose path cannot be fetched safely"
                );
                continue;
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
                Err(error) => return Err(error),
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

    /// The path a pushed file named with encoded parent segments would have
    /// sent the tenant's token to before: `/repos/x/y/contents/z.md`.
    const TRAVERSAL: &str =
        "docs/%2e%2e/%2e%2e/%2e%2e/%2e%2e/%2e%2e/repos/x/y/contents/z.md";

    fn rejected(result: errors::Result<reqwest::Url>) -> bool {
        matches!(result, Err(errors::Error::BadRequest { .. }))
    }

    #[test]
    fn contents_url_encodes_each_segment_and_the_ref() {
        let url = contents_url(
            "octo-org/docs.site",
            "guides/a b/ü.md",
            "feature/x",
        )
        .unwrap();
        assert_eq!(
            url.as_str(),
            "https://api.github.com/repos/octo-org/docs.site/contents/\
             guides/a%20b/%C3%BC.md?ref=feature%2Fx"
        );
    }

    #[test]
    fn encoded_parent_segments_cannot_leave_the_repository() {
        assert!(rejected(contents_url("o/r", TRAVERSAL, "main")));
        for path in [
            "docs/../secret.md",
            "docs/./a.md",
            "docs//a.md",
            "/a.md",
            "docs/",
            "docs/.%2e/a.md",
            "docs/%2E%2E/a.md",
            "docs/%2e/a.md",
            "docs/a%2fb.md",
            "docs/a%5c..md",
            "docs\\..\\a.md",
            "x.env?a=.md",
            "a.md#frag",
        ] {
            assert!(rejected(contents_url("o/r", path, "main")), "{path}");
            assert!(!is_fetchable_file_path(path), "{path}");
        }
    }

    #[test]
    fn a_percent_in_a_name_stays_literal() {
        // `%41` is not a dot segment, so it is fetchable, but it must reach
        // GitHub as the literal name, not be decoded on the way.
        let url = contents_url("o/r", "docs/100%41.md", "main").unwrap();
        assert_eq!(url.path(), "/repos/o/r/contents/docs/100%2541.md");
        assert_eq!(url.query(), Some("ref=main"));
    }

    #[test]
    fn repository_and_ref_are_validated() {
        for repo in [
            "o", "o/r/x", "/r", "o/", "../r", "o/..", "o/%2e%2e", "o/r?x",
            "o r/x",
        ] {
            assert!(rejected(contents_url(repo, "a.md", "main")), "{repo}");
            assert!(rejected(commits_url(repo, "main")), "{repo}");
            assert!(rejected(tree_url(repo, "main")), "{repo}");
            assert!(rejected(pr_files_url(repo, 1)), "{repo}");
        }
        for branch in [
            "", "../x", "a/../b", "a//b", ".hidden", "a?b", "a#b", "a%2e",
            "a b", "x.lock", "a\\b",
        ] {
            assert!(
                rejected(contents_url("o/r", "a.md", branch)),
                "{branch}"
            );
            assert!(rejected(commits_url("o/r", branch)), "{branch}");
            assert!(rejected(tree_url("o/r", branch)), "{branch}");
        }
    }

    #[test]
    fn other_urls_keep_their_shape() {
        assert_eq!(
            commits_url("o/r", "release/1.0").unwrap().as_str(),
            "https://api.github.com/repos/o/r/commits/release/1.0"
        );
        assert_eq!(
            tree_url("o/r", "main").unwrap().as_str(),
            "https://api.github.com/repos/o/r/git/trees/main?recursive=1"
        );
        assert_eq!(
            pr_files_url("o/r", 42).unwrap().as_str(),
            "https://api.github.com/repos/o/r/pulls/42/files?per_page=100"
        );
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
