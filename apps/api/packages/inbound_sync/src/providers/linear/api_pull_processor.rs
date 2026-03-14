//! Linear API pull processor for Initial Sync and On-demand Pull.

use std::sync::Arc;

use async_trait::async_trait;

use inbound_sync_domain::{
    ProcessingStats, Provider, ProviderConfig, SyncDirection,
    SyncOperation, SyncState, SyncStateRepository, WebhookEndpoint,
};

use crate::usecase::ApiPullProcessor;

use super::event_processor::{LinearClient, LinearDataHandler};

/// Linear API pull processor.
#[derive(Debug)]
pub struct LinearApiPullProcessor {
    linear_client: Arc<dyn LinearClient>,
    data_handler: Arc<dyn LinearDataHandler>,
}

impl LinearApiPullProcessor {
    pub fn new(
        linear_client: Arc<dyn LinearClient>,
        data_handler: Arc<dyn LinearDataHandler>,
    ) -> Self {
        Self {
            linear_client,
            data_handler,
        }
    }
}

#[async_trait]
impl ApiPullProcessor for LinearApiPullProcessor {
    fn provider(&self) -> Provider {
        Provider::Linear
    }

    async fn pull_all(
        &self,
        endpoint: &WebhookEndpoint,
        sync_state_repo: &Arc<dyn SyncStateRepository>,
        operation: &mut SyncOperation,
    ) -> errors::Result<ProcessingStats> {
        let (team_id, project_id) = match endpoint.config() {
            ProviderConfig::Linear {
                team_id,
                project_id,
                ..
            } => (team_id.as_deref(), project_id.as_deref()),
            _ => {
                return Err(errors::Error::invalid(
                    "Invalid provider config for Linear",
                ));
            }
        };

        let mut stats = ProcessingStats::default();

        // Fetch all issues
        let issues = self
            .linear_client
            .list_issues(endpoint.tenant_id(), team_id, project_id)
            .await?;

        let total = issues.len();
        tracing::info!(
            endpoint_id = %endpoint.id(),
            total_issues = total,
            "Starting Linear initial sync"
        );

        for (idx, issue) in issues.iter().enumerate() {
            operation.update_progress(format!(
                "{}/{} issues",
                idx + 1,
                total
            ));

            let external_id = format!("linear:issue:{}", issue.id);

            // Check if already synced
            if let Some(state) = sync_state_repo
                .find_by_external_id(endpoint.id(), &external_id)
                .await?
            {
                if !state.has_external_changed(&issue.updated_at) {
                    stats.skipped += 1;
                    continue;
                }
            }

            // Sync issue
            match self
                .data_handler
                .upsert_issue(endpoint, issue, endpoint.mapping())
                .await
            {
                Ok(data_id) => {
                    let mut state = SyncState::create(
                        endpoint.id().clone(),
                        data_id,
                        external_id,
                        SyncDirection::Inbound,
                    );
                    state.update_inbound(
                        Some(issue.updated_at.clone()),
                        None,
                    );
                    sync_state_repo.save(&state).await?;
                    stats.created += 1;
                }
                Err(e) => {
                    tracing::warn!(
                        issue_id = %issue.id,
                        error = %e,
                        "Failed to upsert issue"
                    );
                    stats.skipped += 1;
                }
            }
        }

        tracing::info!(
            endpoint_id = %endpoint.id(),
            created = stats.created,
            skipped = stats.skipped,
            "Linear initial sync completed"
        );

        Ok(stats)
    }

    async fn pull_specific(
        &self,
        endpoint: &WebhookEndpoint,
        external_ids: Vec<String>,
        _sync_state_repo: &Arc<dyn SyncStateRepository>,
    ) -> errors::Result<ProcessingStats> {
        let mut stats = ProcessingStats::default();

        for external_id in external_ids {
            // Parse external ID: "linear:issue:id" or "linear:project:id"
            let parts: Vec<&str> = external_id.split(':').collect();
            if parts.len() != 3 || parts[0] != "linear" {
                tracing::warn!(external_id = %external_id, "Invalid external ID format");
                stats.skipped += 1;
                continue;
            }

            let resource_type = parts[1];
            let resource_id = parts[2];

            match resource_type {
                "issue" => {
                    match self
                        .linear_client
                        .get_issue(endpoint.tenant_id(), resource_id)
                        .await
                    {
                        Ok(issue) => {
                            match self
                                .data_handler
                                .upsert_issue(
                                    endpoint,
                                    &issue,
                                    endpoint.mapping(),
                                )
                                .await
                            {
                                Ok(_) => stats.updated += 1,
                                Err(e) => {
                                    tracing::warn!(issue_id = %resource_id, error = %e, "Failed to upsert");
                                    stats.skipped += 1;
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!(issue_id = %resource_id, error = %e, "Failed to fetch");
                            stats.skipped += 1;
                        }
                    }
                }
                "project" => {
                    match self
                        .linear_client
                        .get_project(endpoint.tenant_id(), resource_id)
                        .await
                    {
                        Ok(project) => {
                            match self
                                .data_handler
                                .upsert_project(
                                    endpoint,
                                    &project,
                                    endpoint.mapping(),
                                )
                                .await
                            {
                                Ok(_) => stats.updated += 1,
                                Err(e) => {
                                    tracing::warn!(project_id = %resource_id, error = %e, "Failed to upsert");
                                    stats.skipped += 1;
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!(project_id = %resource_id, error = %e, "Failed to fetch");
                            stats.skipped += 1;
                        }
                    }
                }
                _ => {
                    tracing::warn!(resource_type = %resource_type, "Unknown resource type");
                    stats.skipped += 1;
                }
            }
        }

        Ok(stats)
    }
}
