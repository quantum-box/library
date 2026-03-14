//! GitHub event processor implementation.

use std::sync::Arc;

use async_trait::async_trait;

use inbound_sync_domain::{
    ProcessingStats, Provider, ProviderConfig, SyncDirection, SyncState,
    SyncStateRepository, WebhookEndpoint, WebhookEvent,
};

use crate::usecase::EventProcessor;

use super::payload::{ChangeType, PullRequestEvent, PushEvent};

/// GitHub event processor.
///
/// Handles GitHub webhook events, particularly push events, by:
/// 1. Parsing the webhook payload
/// 2. Filtering files based on path patterns
/// 3. Fetching file contents from GitHub API
/// 4. Creating/updating/deleting data in Library
#[derive(Debug)]
pub struct GitHubEventProcessor {
    github_client: Arc<dyn GitHubClient>,
    sync_state_repo: Arc<dyn SyncStateRepository>,
    data_handler: Arc<dyn GitHubDataHandler>,
}

impl GitHubEventProcessor {
    pub fn new(
        github_client: Arc<dyn GitHubClient>,
        sync_state_repo: Arc<dyn SyncStateRepository>,
        data_handler: Arc<dyn GitHubDataHandler>,
    ) -> Self {
        Self {
            github_client,
            sync_state_repo,
            data_handler,
        }
    }

    /// Process a push event.
    async fn process_push(
        &self,
        push: &PushEvent,
        endpoint: &WebhookEndpoint,
    ) -> errors::Result<ProcessingStats> {
        let mut stats = ProcessingStats::default();

        // Get GitHub config
        let (repo, branch, path_pattern) = match endpoint.config() {
            ProviderConfig::Github {
                repository,
                branch,
                path_pattern,
            } => (repository, branch, path_pattern.as_deref()),
            _ => {
                return Err(errors::Error::invalid(
                    "Invalid provider config for GitHub",
                ))
            }
        };

        // Check if this push is for the configured branch
        let push_branch = push.branch().unwrap_or("");
        if push_branch != branch {
            tracing::debug!(
                push_branch = push_branch,
                configured_branch = branch,
                "Push is not for configured branch, skipping"
            );
            return Ok(stats);
        }

        // Verify repository matches
        if &push.repository.full_name != repo {
            tracing::warn!(
                push_repo = %push.repository.full_name,
                configured_repo = repo,
                "Repository mismatch"
            );
            return Ok(stats);
        }

        // Get all changed files
        let changed_files = push.all_changed_files();

        // Filter by path pattern if configured
        let files_to_process: Vec<_> = if let Some(pattern) = path_pattern {
            changed_files
                .into_iter()
                .filter(|f| f.matches_pattern(pattern))
                .collect()
        } else {
            changed_files
        };

        tracing::info!(
            file_count = files_to_process.len(),
            pattern = path_pattern,
            "Processing changed files"
        );

        // Process each file
        for file in files_to_process {
            match file.change_type {
                ChangeType::Added | ChangeType::Modified => {
                    match self
                        .process_added_or_modified(
                            endpoint,
                            repo,
                            branch,
                            &file.path,
                            &push.after,
                        )
                        .await
                    {
                        Ok(created) => {
                            if created {
                                stats.created += 1;
                            } else {
                                stats.updated += 1;
                            }
                        }
                        Err(e) => {
                            tracing::error!(
                                file = %file.path,
                                error = %e,
                                "Failed to process file"
                            );
                            stats.skipped += 1;
                        }
                    }
                }
                ChangeType::Removed => {
                    match self
                        .process_removed(endpoint, repo, &file.path)
                        .await
                    {
                        Ok(deleted) => {
                            if deleted {
                                stats.deleted += 1;
                            } else {
                                stats.skipped += 1;
                            }
                        }
                        Err(e) => {
                            tracing::error!(
                                file = %file.path,
                                error = %e,
                                "Failed to process file deletion"
                            );
                            stats.skipped += 1;
                        }
                    }
                }
            }
        }

        Ok(stats)
    }

    /// Process an added or modified file.
    async fn process_added_or_modified(
        &self,
        endpoint: &WebhookEndpoint,
        repo: &str,
        branch: &str,
        path: &str,
        commit_sha: &str,
    ) -> errors::Result<bool> {
        // Fetch file content from GitHub
        let content = self
            .github_client
            .get_file_content(endpoint.tenant_id(), repo, path, branch)
            .await?;

        // Generate external ID for this file
        let external_id = format!("{repo}:{path}");

        // Check if we already have a sync state for this file
        let existing_state = self
            .sync_state_repo
            .find_by_external_id(endpoint.id(), &external_id)
            .await?;

        let is_new = existing_state.is_none();

        // Have the data handler process the content
        let data_id = self
            .data_handler
            .upsert_data(endpoint, path, &content, endpoint.mapping())
            .await?;

        // Update sync state
        let state = if let Some(mut existing) = existing_state {
            existing.update_inbound(
                Some(commit_sha.to_string()),
                Some(data_id.clone()),
            );
            existing
        } else {
            SyncState::create(
                endpoint.id().clone(),
                &data_id,
                &external_id,
                SyncDirection::Inbound,
            )
        };

        self.sync_state_repo.save(&state).await?;

        tracing::info!(
            path = path,
            data_id = data_id,
            is_new = is_new,
            "File synced from GitHub"
        );

        Ok(is_new)
    }

    /// Process a removed file.
    async fn process_removed(
        &self,
        endpoint: &WebhookEndpoint,
        repo: &str,
        path: &str,
    ) -> errors::Result<bool> {
        let external_id = format!("{repo}:{path}");

        // Find the sync state
        let state = self
            .sync_state_repo
            .find_by_external_id(endpoint.id(), &external_id)
            .await?;

        let Some(state) = state else {
            tracing::debug!(
                path = path,
                "No sync state found for removed file, skipping"
            );
            return Ok(false);
        };

        // Delete the data
        self.data_handler
            .delete_data(endpoint, state.data_id())
            .await?;

        // Delete the sync state
        self.sync_state_repo.delete(state.id()).await?;

        tracing::info!(path = path, "File deleted from GitHub sync");

        Ok(true)
    }

    /// Process a pull request event.
    async fn process_pull_request(
        &self,
        pr_event: &PullRequestEvent,
        endpoint: &WebhookEndpoint,
    ) -> errors::Result<ProcessingStats> {
        let mut stats = ProcessingStats::default();

        // Get GitHub config
        let (repo, branch, path_pattern) = match endpoint.config() {
            ProviderConfig::Github {
                repository,
                branch,
                path_pattern,
            } => (repository, branch, path_pattern.as_deref()),
            _ => {
                return Err(errors::Error::invalid(
                    "Invalid provider config for GitHub",
                ))
            }
        };

        // Verify repository matches
        if &pr_event.repository.full_name != repo {
            tracing::debug!(
                pr_repo = %pr_event.repository.full_name,
                configured_repo = repo,
                "Repository mismatch"
            );
            return Ok(stats);
        }

        // Only process merged PRs that target the configured branch
        if !pr_event.is_merged() {
            tracing::debug!(
                action = %pr_event.action,
                "PR not merged, skipping"
            );
            return Ok(stats);
        }

        // Check if PR was merged into the configured branch
        if pr_event.pull_request.base.branch_ref != *branch {
            tracing::debug!(
                base_branch = %pr_event.pull_request.base.branch_ref,
                configured_branch = branch,
                "PR not merged into configured branch, skipping"
            );
            return Ok(stats);
        }

        tracing::info!(
            pr_number = pr_event.number,
            title = %pr_event.pull_request.title,
            changed_files = pr_event.pull_request.changed_files,
            "Processing merged PR"
        );

        // Fetch changed files from the PR
        // Note: For large PRs, we need to use the GitHub API to get the
        // list of changed files
        let changed_files = self
            .github_client
            .get_pr_files(endpoint.tenant_id(), repo, pr_event.number)
            .await?;

        // Filter by path pattern if configured
        let files_to_process: Vec<_> = if let Some(pattern) = path_pattern {
            changed_files
                .into_iter()
                .filter(|f| {
                    super::payload::ChangedFile {
                        path: f.filename.clone(),
                        change_type: ChangeType::Modified,
                    }
                    .matches_pattern(pattern)
                })
                .collect()
        } else {
            changed_files
        };

        tracing::info!(
            file_count = files_to_process.len(),
            pattern = path_pattern,
            "Processing files from merged PR"
        );

        // Process each changed file
        for file in files_to_process {
            match file.status.as_str() {
                "added" | "modified" | "changed" => {
                    match self
                        .process_added_or_modified(
                            endpoint,
                            repo,
                            branch,
                            &file.filename,
                            pr_event
                                .pull_request
                                .merge_commit_sha
                                .as_deref()
                                .unwrap_or(&pr_event.pull_request.head.sha),
                        )
                        .await
                    {
                        Ok(created) => {
                            if created {
                                stats.created += 1;
                            } else {
                                stats.updated += 1;
                            }
                        }
                        Err(e) => {
                            tracing::error!(
                                file = %file.filename,
                                error = %e,
                                "Failed to process file from PR"
                            );
                            stats.skipped += 1;
                        }
                    }
                }
                "removed" => {
                    match self
                        .process_removed(endpoint, repo, &file.filename)
                        .await
                    {
                        Ok(deleted) => {
                            if deleted {
                                stats.deleted += 1;
                            } else {
                                stats.skipped += 1;
                            }
                        }
                        Err(e) => {
                            tracing::error!(
                                file = %file.filename,
                                error = %e,
                                "Failed to process file deletion from PR"
                            );
                            stats.skipped += 1;
                        }
                    }
                }
                other => {
                    tracing::debug!(
                        status = other,
                        file = %file.filename,
                        "Unsupported file status"
                    );
                    stats.skipped += 1;
                }
            }
        }

        Ok(stats)
    }
}

#[async_trait]
impl EventProcessor for GitHubEventProcessor {
    fn provider(&self) -> Provider {
        Provider::Github
    }

    async fn process(
        &self,
        event: &WebhookEvent,
        endpoint: &WebhookEndpoint,
    ) -> errors::Result<ProcessingStats> {
        // Parse the event type
        let event_type = event.event_type();

        match event_type.as_str() {
            "push" => {
                // Parse push event payload
                let push: PushEvent =
                    serde_json::from_value(event.payload().clone())
                        .map_err(|e| {
                            errors::Error::invalid(format!(
                                "Failed to parse push event: {e}"
                            ))
                        })?;

                // Skip deleted branches
                if push.deleted {
                    tracing::debug!("Skipping branch deletion event");
                    return Ok(ProcessingStats::default());
                }

                self.process_push(&push, endpoint).await
            }
            "pull_request" => {
                // Parse pull request event payload
                let pr_event: PullRequestEvent =
                    serde_json::from_value(event.payload().clone())
                        .map_err(|e| {
                            errors::Error::invalid(format!(
                                "Failed to parse pull_request event: {e}"
                            ))
                        })?;

                self.process_pull_request(&pr_event, endpoint).await
            }
            other => {
                tracing::debug!(
                    event_type = other,
                    "Unsupported event type"
                );
                Ok(ProcessingStats::default())
            }
        }
    }
}

/// Trait for GitHub API client.
///
/// The client is designed to work with OAuth tokens. Implementations can either:
/// 1. Use a pre-configured access token (GitHubApiClient)
/// 2. Dynamically fetch tokens via OAuthTokenProvider (OAuthGitHubClient)
#[async_trait]
pub trait GitHubClient: Send + Sync + std::fmt::Debug {
    /// Get file content from a repository.
    ///
    /// # Arguments
    ///
    /// * `tenant_id` - The tenant ID for OAuth token retrieval
    /// * `repo` - Repository in "owner/repo" format
    /// * `path` - File path within the repository
    /// * `branch` - Branch name
    async fn get_file_content(
        &self,
        tenant_id: &value_object::TenantId,
        repo: &str,
        path: &str,
        branch: &str,
    ) -> errors::Result<String>;

    /// Get list of files changed in a pull request.
    ///
    /// # Arguments
    ///
    /// * `tenant_id` - The tenant ID for OAuth token retrieval
    /// * `repo` - Repository in "owner/repo" format
    /// * `pr_number` - Pull request number
    async fn get_pr_files(
        &self,
        tenant_id: &value_object::TenantId,
        repo: &str,
        pr_number: u64,
    ) -> errors::Result<Vec<PullRequestFile>>;

    /// List all files in a repository (for Initial Sync).
    ///
    /// # Arguments
    ///
    /// * `tenant_id` - The tenant ID for OAuth token retrieval
    /// * `repo` - Repository in "owner/repo" format
    /// * `branch` - Branch name
    /// * `path_pattern` - Optional glob pattern to filter files (e.g., "docs/**/*.md")
    async fn list_repository_contents(
        &self,
        tenant_id: &value_object::TenantId,
        repo: &str,
        branch: &str,
        path_pattern: Option<&str>,
    ) -> errors::Result<Vec<RepositoryContent>>;
}

/// File content in a repository.
#[derive(Debug, Clone)]
pub struct RepositoryContent {
    /// File path
    pub path: String,
    /// File content (None for binary files or directories)
    pub content: Option<String>,
    /// SHA hash of the file
    pub sha: String,
    /// File size in bytes
    pub size: usize,
}

/// File changed in a pull request.
#[derive(Debug, Clone)]
pub struct PullRequestFile {
    /// File path
    pub filename: String,
    /// Change status (added, removed, modified, renamed, etc.)
    pub status: String,
    /// Number of additions
    pub additions: u32,
    /// Number of deletions
    pub deletions: u32,
    /// Number of changes
    pub changes: u32,
    /// Previous filename (for renamed files)
    pub previous_filename: Option<String>,
}

/// Trait for handling Library data operations.
#[async_trait]
pub trait GitHubDataHandler: Send + Sync + std::fmt::Debug {
    /// Upsert data in Library from GitHub file content.
    ///
    /// Returns the Library data ID.
    async fn upsert_data(
        &self,
        endpoint: &WebhookEndpoint,
        path: &str,
        content: &str,
        mapping: Option<&inbound_sync_domain::PropertyMapping>,
    ) -> errors::Result<String>;

    /// Delete data from Library.
    async fn delete_data(
        &self,
        endpoint: &WebhookEndpoint,
        data_id: &str,
    ) -> errors::Result<()>;
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_external_id_format() {
        let external_id = format!("{}:{}", "owner/repo", "docs/article.md");
        assert_eq!(external_id, "owner/repo:docs/article.md");
    }
}
