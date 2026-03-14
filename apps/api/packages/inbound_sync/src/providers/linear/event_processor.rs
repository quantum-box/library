//! Linear event processor implementation.

use std::sync::Arc;

use async_trait::async_trait;

use inbound_sync_domain::{
    ProcessingStats, Provider, ProviderConfig, SyncDirection, SyncState,
    SyncStateRepository, WebhookEndpoint, WebhookEvent,
};

use crate::usecase::EventProcessor;

use super::payload::{Issue, LinearWebhookEvent, Project, Team};

/// Linear event processor.
///
/// Handles Linear webhook events for:
/// - Issue: create, update, remove
/// - Project: create, update, remove
/// - Cycle: create, update, remove (future)
#[derive(Debug)]
pub struct LinearEventProcessor {
    #[allow(dead_code)]
    linear_client: Arc<dyn LinearClient>,
    sync_state_repo: Arc<dyn SyncStateRepository>,
    data_handler: Arc<dyn LinearDataHandler>,
}

impl LinearEventProcessor {
    pub fn new(
        linear_client: Arc<dyn LinearClient>,
        sync_state_repo: Arc<dyn SyncStateRepository>,
        data_handler: Arc<dyn LinearDataHandler>,
    ) -> Self {
        Self {
            linear_client,
            sync_state_repo,
            data_handler,
        }
    }

    /// Process an issue event.
    async fn process_issue_event(
        &self,
        event: &LinearWebhookEvent,
        issue: &Issue,
        endpoint: &WebhookEndpoint,
    ) -> errors::Result<ProcessingStats> {
        let mut stats = ProcessingStats::default();

        // Get Linear config
        let (team_id, project_id) = match endpoint.config() {
            ProviderConfig::Linear {
                team_id,
                project_id,
                ..
            } => (team_id, project_id),
            _ => {
                return Err(errors::Error::invalid(
                    "Invalid provider config for Linear",
                ))
            }
        };

        // Check if this issue belongs to the configured team
        if let Some(configured_team_id) = team_id {
            if let Some(team) = &issue.team {
                if &team.id != configured_team_id {
                    tracing::debug!(
                        issue_team_id = %team.id,
                        configured_team_id = %configured_team_id,
                        "Issue team does not match configured team, skipping"
                    );
                    return Ok(stats);
                }
            }
        }

        // Check if this issue belongs to the configured project
        if let Some(configured_project_id) = project_id {
            if let Some(project) = &issue.project {
                if &project.id != configured_project_id {
                    tracing::debug!(
                        issue_project_id = %project.id,
                        configured_project_id = %configured_project_id,
                        "Issue project does not match configured project, skipping"
                    );
                    return Ok(stats);
                }
            }
        }

        let external_id = format!("linear:issue:{}", issue.id);

        match event.action.as_str() {
            "create" => {
                let data_id = self
                    .data_handler
                    .upsert_issue(endpoint, issue, endpoint.mapping())
                    .await?;

                let state = SyncState::create(
                    endpoint.id().clone(),
                    &data_id,
                    &external_id,
                    SyncDirection::Inbound,
                );
                self.sync_state_repo.save(&state).await?;

                tracing::info!(
                    identifier = %issue.identifier,
                    data_id = data_id,
                    "Linear issue created"
                );
                stats.created += 1;
            }
            "update" => {
                let existing_state = self
                    .sync_state_repo
                    .find_by_external_id(endpoint.id(), &external_id)
                    .await?;

                let data_id = self
                    .data_handler
                    .upsert_issue(endpoint, issue, endpoint.mapping())
                    .await?;

                let state = if let Some(mut existing) = existing_state {
                    existing.update_inbound(None, Some(data_id.clone()));
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
                    identifier = %issue.identifier,
                    data_id = data_id,
                    "Linear issue updated"
                );
                stats.updated += 1;
            }
            "remove" => {
                let existing_state = self
                    .sync_state_repo
                    .find_by_external_id(endpoint.id(), &external_id)
                    .await?;

                if let Some(state) = existing_state {
                    self.data_handler
                        .delete_issue(endpoint, state.data_id())
                        .await?;
                    self.sync_state_repo.delete(state.id()).await?;

                    tracing::info!(
                        identifier = %issue.identifier,
                        "Linear issue deleted"
                    );
                    stats.deleted += 1;
                } else {
                    tracing::debug!(
                        identifier = %issue.identifier,
                        "No sync state for removed issue, skipping"
                    );
                    stats.skipped += 1;
                }
            }
            other => {
                tracing::debug!(action = other, "Unsupported issue action");
                stats.skipped += 1;
            }
        }

        Ok(stats)
    }

    /// Process a project event.
    async fn process_project_event(
        &self,
        event: &LinearWebhookEvent,
        project: &Project,
        endpoint: &WebhookEndpoint,
    ) -> errors::Result<ProcessingStats> {
        let mut stats = ProcessingStats::default();

        let external_id = format!("linear:project:{}", project.id);

        match event.action.as_str() {
            "create" => {
                let data_id = self
                    .data_handler
                    .upsert_project(endpoint, project, endpoint.mapping())
                    .await?;

                let state = SyncState::create(
                    endpoint.id().clone(),
                    &data_id,
                    &external_id,
                    SyncDirection::Inbound,
                );
                self.sync_state_repo.save(&state).await?;

                tracing::info!(
                    project_name = %project.name,
                    data_id = data_id,
                    "Linear project created"
                );
                stats.created += 1;
            }
            "update" => {
                let existing_state = self
                    .sync_state_repo
                    .find_by_external_id(endpoint.id(), &external_id)
                    .await?;

                let data_id = self
                    .data_handler
                    .upsert_project(endpoint, project, endpoint.mapping())
                    .await?;

                let state = if let Some(mut existing) = existing_state {
                    existing.update_inbound(None, Some(data_id.clone()));
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
                    project_name = %project.name,
                    data_id = data_id,
                    "Linear project updated"
                );
                stats.updated += 1;
            }
            "remove" => {
                let existing_state = self
                    .sync_state_repo
                    .find_by_external_id(endpoint.id(), &external_id)
                    .await?;

                if let Some(state) = existing_state {
                    self.data_handler
                        .delete_project(endpoint, state.data_id())
                        .await?;
                    self.sync_state_repo.delete(state.id()).await?;

                    tracing::info!(
                        project_name = %project.name,
                        "Linear project deleted"
                    );
                    stats.deleted += 1;
                } else {
                    tracing::debug!(
                        project_name = %project.name,
                        "No sync state for removed project, skipping"
                    );
                    stats.skipped += 1;
                }
            }
            other => {
                tracing::debug!(
                    action = other,
                    "Unsupported project action"
                );
                stats.skipped += 1;
            }
        }

        Ok(stats)
    }
}

#[async_trait]
impl EventProcessor for LinearEventProcessor {
    fn provider(&self) -> Provider {
        Provider::Linear
    }

    async fn process(
        &self,
        event: &WebhookEvent,
        endpoint: &WebhookEndpoint,
    ) -> errors::Result<ProcessingStats> {
        // Parse the Linear webhook event
        let linear_event: LinearWebhookEvent = serde_json::from_value(
            event.payload().clone(),
        )
        .map_err(|e| {
            errors::Error::invalid(format!(
                "Failed to parse Linear webhook event: {e}"
            ))
        })?;

        match linear_event.event_type.as_str() {
            "Issue" => {
                if let Some(issue) = linear_event.as_issue() {
                    self.process_issue_event(
                        &linear_event,
                        &issue,
                        endpoint,
                    )
                    .await
                } else {
                    Err(errors::Error::invalid(
                        "Failed to parse issue from Linear event",
                    ))
                }
            }
            "Project" => {
                if let Some(project) = linear_event.as_project() {
                    self.process_project_event(
                        &linear_event,
                        &project,
                        endpoint,
                    )
                    .await
                } else {
                    Err(errors::Error::invalid(
                        "Failed to parse project from Linear event",
                    ))
                }
            }
            "Comment" => {
                // Comment events can be processed if needed
                tracing::debug!("Comment events not yet implemented");
                Ok(ProcessingStats::default())
            }
            "Cycle" => {
                // Cycle events can be processed if needed
                tracing::debug!("Cycle events not yet implemented");
                Ok(ProcessingStats::default())
            }
            other => {
                tracing::debug!(
                    event_type = other,
                    "Unsupported Linear event type"
                );
                Ok(ProcessingStats::default())
            }
        }
    }
}

/// Trait for Linear API client.
#[async_trait]
pub trait LinearClient: Send + Sync + std::fmt::Debug {
    /// Get an issue by ID with full details.
    async fn get_issue(
        &self,
        tenant_id: &value_object::TenantId,
        issue_id: &str,
    ) -> errors::Result<Issue>;

    /// Get a project by ID with full details.
    async fn get_project(
        &self,
        tenant_id: &value_object::TenantId,
        project_id: &str,
    ) -> errors::Result<Project>;

    /// List all issues (for Initial Sync).
    ///
    /// # Arguments
    ///
    /// * `tenant_id` - The tenant ID for OAuth token retrieval
    /// * `team_id` - Optional team ID filter
    /// * `project_id` - Optional project ID filter
    async fn list_issues(
        &self,
        tenant_id: &value_object::TenantId,
        team_id: Option<&str>,
        project_id: Option<&str>,
    ) -> errors::Result<Vec<Issue>>;

    /// List all projects (for Initial Sync).
    async fn list_projects(
        &self,
        tenant_id: &value_object::TenantId,
        team_id: Option<&str>,
    ) -> errors::Result<Vec<Project>>;

    /// List all teams.
    async fn list_teams(
        &self,
        tenant_id: &value_object::TenantId,
    ) -> errors::Result<Vec<Team>>;
}

/// Trait for handling Library data operations for Linear.
#[async_trait]
pub trait LinearDataHandler: Send + Sync + std::fmt::Debug {
    /// Upsert an issue in Library.
    ///
    /// Returns the Library data ID.
    async fn upsert_issue(
        &self,
        endpoint: &WebhookEndpoint,
        issue: &Issue,
        mapping: Option<&inbound_sync_domain::PropertyMapping>,
    ) -> errors::Result<String>;

    /// Delete an issue from Library.
    async fn delete_issue(
        &self,
        endpoint: &WebhookEndpoint,
        data_id: &str,
    ) -> errors::Result<()>;

    /// Upsert a project in Library.
    ///
    /// Returns the Library data ID.
    async fn upsert_project(
        &self,
        endpoint: &WebhookEndpoint,
        project: &Project,
        mapping: Option<&inbound_sync_domain::PropertyMapping>,
    ) -> errors::Result<String>;

    /// Delete a project from Library.
    async fn delete_project(
        &self,
        endpoint: &WebhookEndpoint,
        data_id: &str,
    ) -> errors::Result<()>;
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_external_id_format() {
        let issue_id = "issue-uuid-123";
        let external_id = format!("linear:issue:{}", issue_id);
        assert_eq!(external_id, "linear:issue:issue-uuid-123");

        let project_id = "project-uuid-456";
        let external_id = format!("linear:project:{}", project_id);
        assert_eq!(external_id, "linear:project:project-uuid-456");
    }
}
