//! GitHub API pull processor for Initial Sync and On-demand Pull.

use std::sync::Arc;

use async_trait::async_trait;

use inbound_sync_domain::{
    ProcessingStats, Provider, ProviderConfig, SyncDirection,
    SyncOperation, SyncState, SyncStateRepository, WebhookEndpoint,
};

use crate::usecase::ApiPullProcessor;

use super::event_processor::{GitHubClient, GitHubDataHandler};

/// GitHub API pull processor.
///
/// Handles pulling data from GitHub API for Initial Sync and On-demand Pull operations.
#[derive(Debug)]
pub struct GitHubApiPullProcessor {
    github_client: Arc<dyn GitHubClient>,
    data_handler: Arc<dyn GitHubDataHandler>,
}

impl GitHubApiPullProcessor {
    pub fn new(
        github_client: Arc<dyn GitHubClient>,
        data_handler: Arc<dyn GitHubDataHandler>,
    ) -> Self {
        Self {
            github_client,
            data_handler,
        }
    }
}

#[async_trait]
impl ApiPullProcessor for GitHubApiPullProcessor {
    fn provider(&self) -> Provider {
        Provider::Github
    }

    async fn pull_all(
        &self,
        endpoint: &WebhookEndpoint,
        sync_state_repo: &Arc<dyn SyncStateRepository>,
        operation: &mut SyncOperation,
    ) -> errors::Result<ProcessingStats> {
        let config = match endpoint.config() {
            ProviderConfig::Github {
                repository,
                branch,
                path_pattern,
            } => (repository, branch, path_pattern.as_deref()),
            _ => {
                return Err(errors::Error::invalid(
                    "Invalid provider config for GitHub",
                ));
            }
        };

        let (repo, branch, path_pattern) = config;

        // Fetch all files from repository
        let contents = self
            .github_client
            .list_repository_contents(
                endpoint.tenant_id(),
                repo,
                branch,
                path_pattern,
            )
            .await?;

        let mut stats = ProcessingStats::default();
        let total = contents.len();

        tracing::info!(
            endpoint_id = %endpoint.id(),
            total_files = total,
            "Starting initial sync"
        );

        for (idx, content) in contents.iter().enumerate() {
            // Update progress
            operation.update_progress(format!(
                "{}/{} files",
                idx + 1,
                total
            ));

            // Generate external ID
            let external_id = format!("{}:{}", repo, content.path);

            // Check if already synced
            if let Some(state) = sync_state_repo
                .find_by_external_id(endpoint.id(), &external_id)
                .await?
            {
                if !state.has_external_changed(&content.sha) {
                    stats.skipped += 1;
                    continue;
                }
            }

            // Sync data
            match &content.content {
                Some(file_content) => {
                    match self
                        .data_handler
                        .upsert_data(
                            endpoint,
                            &content.path,
                            file_content,
                            endpoint.mapping(),
                        )
                        .await
                    {
                        Ok(data_id) => {
                            // Create or update sync state
                            let mut state = SyncState::create(
                                endpoint.id().clone(),
                                data_id,
                                external_id,
                                SyncDirection::Inbound,
                            );
                            state.update_inbound(
                                Some(content.sha.clone()),
                                None,
                            );
                            sync_state_repo.save(&state).await?;
                            stats.created += 1;
                        }
                        Err(e) => {
                            tracing::warn!(
                                path = %content.path,
                                error = %e,
                                "Failed to upsert data"
                            );
                            stats.skipped += 1;
                        }
                    }
                }
                None => {
                    stats.skipped += 1;
                }
            }
        }

        tracing::info!(
            endpoint_id = %endpoint.id(),
            created = stats.created,
            updated = stats.updated,
            skipped = stats.skipped,
            "Initial sync completed"
        );

        Ok(stats)
    }

    async fn pull_specific(
        &self,
        endpoint: &WebhookEndpoint,
        external_ids: Vec<String>,
        _sync_state_repo: &Arc<dyn SyncStateRepository>,
    ) -> errors::Result<ProcessingStats> {
        let config = match endpoint.config() {
            ProviderConfig::Github {
                repository,
                branch,
                path_pattern: _,
            } => (repository, branch),
            _ => {
                return Err(errors::Error::invalid(
                    "Invalid provider config for GitHub",
                ));
            }
        };

        let (repo, branch) = config;
        let mut stats = ProcessingStats::default();

        for external_id in external_ids {
            // Parse external ID: "owner/repo:path"
            let parts: Vec<&str> = external_id.split(':').collect();
            if parts.len() != 2 {
                tracing::warn!(
                    external_id = %external_id,
                    "Invalid external ID format"
                );
                stats.skipped += 1;
                continue;
            }

            let path = parts[1];

            // Fetch file content
            match self
                .github_client
                .get_file_content(endpoint.tenant_id(), repo, path, branch)
                .await
            {
                Ok(content) => {
                    match self
                        .data_handler
                        .upsert_data(
                            endpoint,
                            path,
                            &content,
                            endpoint.mapping(),
                        )
                        .await
                    {
                        Ok(_) => {
                            stats.updated += 1;
                        }
                        Err(e) => {
                            tracing::warn!(
                                path = %path,
                                error = %e,
                                "Failed to upsert data"
                            );
                            stats.skipped += 1;
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(path = %path, error = %e, "Failed to fetch file");
                    stats.skipped += 1;
                }
            }
        }

        Ok(stats)
    }
}
