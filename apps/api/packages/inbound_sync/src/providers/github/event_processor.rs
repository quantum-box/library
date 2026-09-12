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
    change_set_sink: Option<Arc<dyn GitHubChangeSetSink>>,
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
            change_set_sink: None,
        }
    }

    /// Route GitHub changes through the provider-neutral review lifecycle.
    pub fn with_change_set_sink(
        mut self,
        sink: Arc<dyn GitHubChangeSetSink>,
    ) -> Self {
        self.change_set_sink = Some(sink);
        self
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
                            &push.before,
                            &push.after,
                        )
                        .await
                    {
                        Ok(Some(created)) => {
                            if created {
                                stats.created += 1;
                            } else {
                                stats.updated += 1;
                            }
                        }
                        Ok(None) => {
                            stats.skipped += 1;
                        }
                        Err(e) => {
                            if self.change_set_sink.is_some() {
                                return Err(e);
                            }
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
                        .process_removed(
                            endpoint,
                            repo,
                            &file.path,
                            &push.before,
                            &push.after,
                        )
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
                            if self.change_set_sink.is_some() {
                                return Err(e);
                            }
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
    ///
    /// Returns `None` when the file is skipped because its commit SHA
    /// matches the recorded sync state (an echo of our own outbound
    /// push, or a webhook redelivery), `Some(true)` when new data was
    /// created, and `Some(false)` when existing data was updated.
    async fn process_added_or_modified(
        &self,
        endpoint: &WebhookEndpoint,
        repo: &str,
        branch: &str,
        path: &str,
        base_commit_sha: &str,
        commit_sha: &str,
    ) -> errors::Result<Option<bool>> {
        if let Some(sink) = &self.change_set_sink {
            // Bind reviewed content to the event's immutable revision. The
            // branch can advance before the durable consumer runs or retries.
            let content = self
                .github_client
                .get_file_content(
                    endpoint.tenant_id(),
                    repo,
                    path,
                    commit_sha,
                )
                .await?;
            let captured = sink
                .capture(GitHubChangeSetInput {
                    endpoint,
                    repository: repo,
                    branch,
                    path,
                    previous_path: None,
                    external_revision: commit_sha,
                    base_external_revision: Some(base_commit_sha),
                    content: Some(&content),
                })
                .await?;
            return Ok(Some(captured));
        }

        // Generate external ID for this file
        let external_id = format!("{repo}:{path}");

        // Check if we already have a sync state for this file
        let existing_state = self
            .sync_state_repo
            .find_by_external_id(endpoint.id(), &external_id)
            .await?;

        // Echo suppression: skip commits this integration already knows
        // about — either an echo of our own outbound push (the commit
        // SHA was recorded by the writeback path) or a webhook
        // redelivery of an already-processed commit.
        if let Some(existing) = &existing_state {
            if !existing.has_external_changed(commit_sha) {
                tracing::debug!(
                    path = path,
                    commit_sha = commit_sha,
                    "Skipping already-synced commit (echo or redelivery)"
                );
                return Ok(None);
            }
        }

        // Fetch file content from GitHub
        let content = self
            .github_client
            .get_file_content(endpoint.tenant_id(), repo, path, branch)
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

        Ok(Some(is_new))
    }

    /// Process a removed file.
    async fn process_removed(
        &self,
        endpoint: &WebhookEndpoint,
        repo: &str,
        path: &str,
        base_commit_sha: &str,
        commit_sha: &str,
    ) -> errors::Result<bool> {
        if let Some(sink) = &self.change_set_sink {
            return sink
                .capture(GitHubChangeSetInput {
                    endpoint,
                    repository: repo,
                    branch: match endpoint.config() {
                        ProviderConfig::Github { branch, .. } => branch,
                        _ => "",
                    },
                    path,
                    previous_path: None,
                    external_revision: commit_sha,
                    base_external_revision: Some(base_commit_sha),
                    content: None,
                })
                .await;
        }

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
                            pr_event.pull_request.base.sha.as_str(),
                            pr_event
                                .pull_request
                                .merge_commit_sha
                                .as_deref()
                                .unwrap_or(&pr_event.pull_request.head.sha),
                        )
                        .await
                    {
                        Ok(Some(created)) => {
                            if created {
                                stats.created += 1;
                            } else {
                                stats.updated += 1;
                            }
                        }
                        Ok(None) => {
                            stats.skipped += 1;
                        }
                        Err(e) => {
                            if self.change_set_sink.is_some() {
                                return Err(e);
                            }
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
                        .process_removed(
                            endpoint,
                            repo,
                            &file.filename,
                            pr_event.pull_request.base.sha.as_str(),
                            pr_event
                                .pull_request
                                .merge_commit_sha
                                .as_deref()
                                .unwrap_or(&pr_event.pull_request.head.sha),
                        )
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
                            if self.change_set_sink.is_some() {
                                return Err(e);
                            }
                            tracing::error!(
                                file = %file.filename,
                                error = %e,
                                "Failed to process file deletion from PR"
                            );
                            stats.skipped += 1;
                        }
                    }
                }
                "renamed" => {
                    let Some(previous_path) =
                        file.previous_filename.as_deref()
                    else {
                        stats.skipped += 1;
                        continue;
                    };
                    if let Some(sink) = &self.change_set_sink {
                        let content = self
                            .github_client
                            .get_file_content(
                                endpoint.tenant_id(),
                                repo,
                                &file.filename,
                                pr_event
                                    .pull_request
                                    .merge_commit_sha
                                    .as_deref()
                                    .unwrap_or(
                                        &pr_event.pull_request.head.sha,
                                    ),
                            )
                            .await?;
                        let captured = sink
                            .capture(GitHubChangeSetInput {
                                endpoint,
                                repository: repo,
                                branch,
                                path: &file.filename,
                                previous_path: Some(previous_path),
                                external_revision: pr_event
                                    .pull_request
                                    .merge_commit_sha
                                    .as_deref()
                                    .unwrap_or(
                                        &pr_event.pull_request.head.sha,
                                    ),
                                base_external_revision: Some(
                                    pr_event.pull_request.base.sha.as_str(),
                                ),
                                content: Some(&content),
                            })
                            .await?;
                        if captured {
                            stats.created += 1;
                        } else {
                            stats.updated += 1;
                        }
                    } else {
                        stats.skipped += 1;
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

pub struct GitHubChangeSetInput<'a> {
    pub endpoint: &'a WebhookEndpoint,
    pub repository: &'a str,
    pub branch: &'a str,
    pub path: &'a str,
    pub previous_path: Option<&'a str>,
    pub external_revision: &'a str,
    pub base_external_revision: Option<&'a str>,
    pub content: Option<&'a str>,
}

#[async_trait]
pub trait GitHubChangeSetSink: Send + Sync + std::fmt::Debug {
    /// Returns true when the external object is not linked to existing Data.
    async fn capture(
        &self,
        input: GitHubChangeSetInput<'_>,
    ) -> errors::Result<bool>;
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

    /// Apply reviewed content to an already-linked Library record. The
    /// default keeps lightweight adapters compatible; the production handler
    /// overrides it to preserve record identity across GitHub renames.
    async fn update_linked_data(
        &self,
        endpoint: &WebhookEndpoint,
        data_id: &str,
        path: &str,
        content: &str,
        mapping: Option<&inbound_sync_domain::PropertyMapping>,
    ) -> errors::Result<String> {
        let _ = data_id;
        self.upsert_data(endpoint, path, content, mapping).await
    }

    /// Delete data from Library.
    async fn delete_data(
        &self,
        endpoint: &WebhookEndpoint,
        data_id: &str,
    ) -> errors::Result<()>;
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use inbound_sync_domain::{
        Provider, ProviderConfig, SyncDirection, SyncState, SyncStateId,
        SyncStateRepository, WebhookEndpoint, WebhookEndpointId,
    };
    use value_object::TenantId;

    use super::{
        GitHubClient, GitHubDataHandler, GitHubEventProcessor,
        PullRequestFile, RepositoryContent,
    };

    #[test]
    fn test_external_id_format() {
        let external_id = format!("{}:{}", "owner/repo", "docs/article.md");
        assert_eq!(external_id, "owner/repo:docs/article.md");
    }

    #[derive(Debug)]
    struct StubGitHubClient;

    #[async_trait::async_trait]
    impl GitHubClient for StubGitHubClient {
        async fn get_file_content(
            &self,
            _tenant_id: &TenantId,
            _repo: &str,
            _path: &str,
            _branch: &str,
        ) -> errors::Result<String> {
            Ok("# Hello".to_string())
        }

        async fn get_pr_files(
            &self,
            _tenant_id: &TenantId,
            _repo: &str,
            _pr_number: u64,
        ) -> errors::Result<Vec<PullRequestFile>> {
            Ok(vec![])
        }

        async fn list_repository_contents(
            &self,
            _tenant_id: &TenantId,
            _repo: &str,
            _branch: &str,
            _path_pattern: Option<&str>,
        ) -> errors::Result<Vec<RepositoryContent>> {
            Ok(vec![])
        }
    }

    #[derive(Debug, Default)]
    struct CountingDataHandler {
        upserts: AtomicUsize,
    }

    #[async_trait::async_trait]
    impl GitHubDataHandler for CountingDataHandler {
        async fn upsert_data(
            &self,
            _endpoint: &WebhookEndpoint,
            _path: &str,
            _content: &str,
            _mapping: Option<&inbound_sync_domain::PropertyMapping>,
        ) -> errors::Result<String> {
            self.upserts.fetch_add(1, Ordering::SeqCst);
            Ok("data_test".to_string())
        }

        async fn delete_data(
            &self,
            _endpoint: &WebhookEndpoint,
            _data_id: &str,
        ) -> errors::Result<()> {
            Ok(())
        }
    }

    /// Sync state repository stub seeded with one state.
    #[derive(Debug)]
    struct SeededSyncStateRepo {
        state: std::sync::Mutex<Option<SyncState>>,
    }

    #[async_trait::async_trait]
    impl SyncStateRepository for SeededSyncStateRepo {
        async fn save(&self, state: &SyncState) -> errors::Result<()> {
            *self.state.lock().unwrap() = Some(state.clone());
            Ok(())
        }

        async fn find_by_id(
            &self,
            _id: &SyncStateId,
        ) -> errors::Result<Option<SyncState>> {
            Ok(self.state.lock().unwrap().clone())
        }

        async fn find_by_external_id(
            &self,
            _endpoint_id: &WebhookEndpointId,
            _external_id: &str,
        ) -> errors::Result<Option<SyncState>> {
            Ok(self.state.lock().unwrap().clone())
        }

        async fn find_by_data_id(
            &self,
            _endpoint_id: &WebhookEndpointId,
            _data_id: &str,
        ) -> errors::Result<Option<SyncState>> {
            Ok(self.state.lock().unwrap().clone())
        }

        async fn find_by_endpoint(
            &self,
            _endpoint_id: &WebhookEndpointId,
        ) -> errors::Result<Vec<SyncState>> {
            Ok(self.state.lock().unwrap().clone().into_iter().collect())
        }

        async fn delete(&self, _id: &SyncStateId) -> errors::Result<()> {
            Ok(())
        }

        async fn delete_by_endpoint(
            &self,
            _endpoint_id: &WebhookEndpointId,
        ) -> errors::Result<u64> {
            Ok(0)
        }
    }

    fn test_endpoint() -> WebhookEndpoint {
        WebhookEndpoint::create(
            TenantId::default(),
            "test",
            Provider::Github,
            ProviderConfig::Github {
                repository: "owner/repo".to_string(),
                branch: "main".to_string(),
                path_pattern: None,
            },
            vec!["push".to_string()],
            "secret_hash",
        )
    }

    fn processor_with_state(
        state: Option<SyncState>,
        handler: Arc<CountingDataHandler>,
    ) -> GitHubEventProcessor {
        GitHubEventProcessor::new(
            Arc::new(StubGitHubClient),
            Arc::new(SeededSyncStateRepo {
                state: std::sync::Mutex::new(state),
            }),
            handler,
        )
    }

    #[tokio::test]
    async fn skips_commit_matching_recorded_external_version() {
        let endpoint = test_endpoint();
        let mut state = SyncState::create(
            endpoint.id().clone(),
            "data_test",
            "owner/repo:docs/a.md",
            SyncDirection::Both,
        );
        // Simulates the outbound writeback having recorded this SHA.
        state.update_outbound(Some("sha_echo".to_string()), None);

        let handler = Arc::new(CountingDataHandler::default());
        let processor = processor_with_state(Some(state), handler.clone());

        let result = processor
            .process_added_or_modified(
                &endpoint,
                "owner/repo",
                "main",
                "docs/a.md",
                "sha_before",
                "sha_echo",
            )
            .await
            .unwrap();

        assert_eq!(result, None);
        assert_eq!(handler.upserts.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn processes_commit_with_changed_external_version() {
        let endpoint = test_endpoint();
        let mut state = SyncState::create(
            endpoint.id().clone(),
            "data_test",
            "owner/repo:docs/a.md",
            SyncDirection::Both,
        );
        state.update_outbound(Some("sha_old".to_string()), None);

        let handler = Arc::new(CountingDataHandler::default());
        let processor = processor_with_state(Some(state), handler.clone());

        let result = processor
            .process_added_or_modified(
                &endpoint,
                "owner/repo",
                "main",
                "docs/a.md",
                "sha_old",
                "sha_new",
            )
            .await
            .unwrap();

        assert_eq!(result, Some(false));
        assert_eq!(handler.upserts.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn processes_new_file_without_sync_state() {
        let endpoint = test_endpoint();
        let handler = Arc::new(CountingDataHandler::default());
        let processor = processor_with_state(None, handler.clone());

        let result = processor
            .process_added_or_modified(
                &endpoint,
                "owner/repo",
                "main",
                "docs/a.md",
                "sha_before",
                "sha_new",
            )
            .await
            .unwrap();

        assert_eq!(result, Some(true));
        assert_eq!(handler.upserts.load(Ordering::SeqCst), 1);
    }

    #[derive(Debug)]
    struct RecordingGitHubClient {
        revisions: std::sync::Mutex<Vec<String>>,
        fail_reads: bool,
        pr_status: Option<&'static str>,
    }

    #[async_trait::async_trait]
    impl GitHubClient for RecordingGitHubClient {
        async fn get_file_content(
            &self,
            _tenant_id: &TenantId,
            _repo: &str,
            _path: &str,
            revision: &str,
        ) -> errors::Result<String> {
            self.revisions.lock().unwrap().push(revision.into());
            if self.fail_reads {
                return Err(errors::Error::service_unavailable(
                    "temporary GitHub failure",
                ));
            }
            Ok("# Reviewed revision".into())
        }
        async fn get_pr_files(
            &self,
            _tenant_id: &TenantId,
            _repo: &str,
            _pr_number: u64,
        ) -> errors::Result<Vec<PullRequestFile>> {
            Ok(self
                .pr_status
                .map(|status| PullRequestFile {
                    filename: "docs/a.md".into(),
                    status: status.into(),
                    additions: 1,
                    deletions: 1,
                    changes: 2,
                    previous_filename: Some("docs/before.md".into()),
                })
                .into_iter()
                .collect())
        }
        async fn list_repository_contents(
            &self,
            _tenant_id: &TenantId,
            _repo: &str,
            _branch: &str,
            _pattern: Option<&str>,
        ) -> errors::Result<Vec<RepositoryContent>> {
            Ok(vec![])
        }
    }

    #[derive(Debug)]
    struct TestChangeSetSink {
        fail_capture: bool,
    }

    #[async_trait::async_trait]
    impl super::GitHubChangeSetSink for TestChangeSetSink {
        async fn capture(
            &self,
            input: super::GitHubChangeSetInput<'_>,
        ) -> errors::Result<bool> {
            assert_eq!(input.branch, "main");
            assert_eq!(input.external_revision, "event-commit");
            if self.fail_capture {
                Err(errors::Error::service_unavailable(
                    "temporary persistence failure",
                ))
            } else {
                Ok(true)
            }
        }
    }

    fn review_processor(
        client: Arc<RecordingGitHubClient>,
        fail_capture: bool,
    ) -> GitHubEventProcessor {
        GitHubEventProcessor::new(
            client,
            Arc::new(SeededSyncStateRepo {
                state: std::sync::Mutex::new(None),
            }),
            Arc::new(CountingDataHandler::default()),
        )
        .with_change_set_sink(Arc::new(TestChangeSetSink { fail_capture }))
    }

    fn review_push(removed: bool) -> super::PushEvent {
        serde_json::from_value(serde_json::json!({
            "ref":"refs/heads/main", "before":"previous-commit", "after":"event-commit",
            "repository": { "id":1, "name":"repo", "full_name":"owner/repo",
                "default_branch":"main", "html_url":"https://github.com/owner/repo",
                "clone_url":"https://github.com/owner/repo.git", "private":true,
                "owner":{"login":"owner", "id":1, "type":"Organization"}},
            "pusher":{"name":"fixture", "email":"fixture@example.invalid"},
            "commits":[{"id":"event-commit", "tree_id":"tree", "message":"test",
                "timestamp":"2026-09-12T00:00:00Z", "url":"https://github.com/owner/repo/commit/event-commit",
                "author":{"name":"fixture", "email":"fixture@example.invalid"},
                "committer":{"name":"fixture", "email":"fixture@example.invalid"},
                "modified":if removed { vec![] } else { vec!["docs/a.md"] },
                "removed":if removed { vec!["docs/a.md"] } else { vec![] }}]
        })).unwrap()
    }

    #[tokio::test]
    async fn review_reads_the_event_commit_even_after_the_branch_moves() {
        let client = Arc::new(RecordingGitHubClient {
            revisions: Default::default(),
            fail_reads: false,
            pr_status: None,
        });
        let processor = review_processor(client.clone(), false);
        processor
            .process_push(&review_push(false), &test_endpoint())
            .await
            .unwrap();
        assert_eq!(*client.revisions.lock().unwrap(), vec!["event-commit"]);
    }

    #[tokio::test]
    async fn review_capture_failures_propagate_for_upserts_and_deletions() {
        for removed in [false, true] {
            let client = Arc::new(RecordingGitHubClient {
                revisions: Default::default(),
                fail_reads: false,
                pr_status: None,
            });
            let processor = review_processor(client, true);
            assert!(
                processor
                    .process_push(&review_push(removed), &test_endpoint())
                    .await
                    .is_err(),
                "failed capture must keep the durable job retryable"
            );
        }
    }

    #[tokio::test]
    async fn review_github_read_failure_keeps_the_webhook_retryable() {
        let client = Arc::new(RecordingGitHubClient {
            revisions: Default::default(),
            fail_reads: true,
            pr_status: None,
        });
        let processor = review_processor(client, false);
        assert!(processor
            .process_push(&review_push(false), &test_endpoint())
            .await
            .is_err());
    }

    fn review_pr() -> super::PullRequestEvent {
        serde_json::from_value(serde_json::json!({
            "action":"closed", "number":1,
            "repository":review_push(false).repository,
            "sender":{"login":"fixture", "id":1, "type":"User"},
            "pull_request": { "id":1, "number":1, "state":"closed", "title":"fixture",
                "merged":true, "merge_commit_sha":"event-commit",
                "html_url":"https://github.com/owner/repo/pull/1",
                "head":{"ref":"feature", "sha":"feature-commit"},
                "base":{"ref":"main", "sha":"previous-commit"},
                "user":{"login":"fixture", "id":1, "type":"User"},
                "created_at":"2026-09-12T00:00:00Z", "updated_at":"2026-09-12T00:00:00Z" }
        })).unwrap()
    }

    #[tokio::test]
    async fn review_pr_reads_the_merge_commit_for_modifications_and_renames(
    ) {
        for status in ["modified", "renamed"] {
            let client = Arc::new(RecordingGitHubClient {
                revisions: Default::default(),
                fail_reads: false,
                pr_status: Some(status),
            });
            let processor = review_processor(client.clone(), false);
            processor
                .process_pull_request(&review_pr(), &test_endpoint())
                .await
                .unwrap();
            assert_eq!(
                *client.revisions.lock().unwrap(),
                vec!["event-commit"]
            );
        }
    }

    #[tokio::test]
    async fn review_pr_capture_failure_keeps_all_change_types_retryable() {
        for status in ["modified", "removed", "renamed"] {
            let client = Arc::new(RecordingGitHubClient {
                revisions: Default::default(),
                fail_reads: false,
                pr_status: Some(status),
            });
            let processor = review_processor(client, true);
            assert!(processor
                .process_pull_request(&review_pr(), &test_endpoint())
                .await
                .is_err());
        }
    }
}
