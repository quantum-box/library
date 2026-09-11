use std::sync::Arc;

use async_trait::async_trait;
use integration_domain::{
    ExternalChangeType, ExternalObjectLink, ExternalObjectLinkRepository,
    ExternalScope, ExternalSyncBinding, ExternalSyncBindingRepository,
    ExternalSyncBindingStatus, ExternalSyncPolicy, InboundChangeSet,
    InboundChangeSetRepository, LibraryDataId, LibraryRepoId,
    OAuthProvider,
};
use serde_json::json;

use inbound_sync_domain::{ConnectionRepository, SyncStateRepository};

use super::{GitHubChangeSetInput, GitHubChangeSetSink};

pub fn github_external_scope(
    repository: &str,
    branch: &str,
    path_pattern: Option<&str>,
) -> errors::Result<ExternalScope> {
    ExternalScope::new(json!({
        "repository": repository,
        "ref": branch,
        "path_pattern": path_pattern,
    }))
}

#[derive(Debug, Clone)]
pub struct DefaultGitHubChangeSetSink {
    bindings: Arc<dyn ExternalSyncBindingRepository>,
    links: Arc<dyn ExternalObjectLinkRepository>,
    change_sets: Arc<dyn InboundChangeSetRepository>,
    connections: Arc<dyn ConnectionRepository>,
    sync_states: Arc<dyn SyncStateRepository>,
}

impl DefaultGitHubChangeSetSink {
    pub fn new(
        bindings: Arc<dyn ExternalSyncBindingRepository>,
        links: Arc<dyn ExternalObjectLinkRepository>,
        change_sets: Arc<dyn InboundChangeSetRepository>,
        connections: Arc<dyn ConnectionRepository>,
        sync_states: Arc<dyn SyncStateRepository>,
    ) -> Self {
        Self {
            bindings,
            links,
            change_sets,
            connections,
            sync_states,
        }
    }
}

#[async_trait]
impl GitHubChangeSetSink for DefaultGitHubChangeSetSink {
    async fn capture(
        &self,
        input: GitHubChangeSetInput<'_>,
    ) -> errors::Result<bool> {
        let repo_id = input.endpoint.repository_id().ok_or_else(|| {
            errors::Error::invalid(
                "GitHub external sync requires a Library repository",
            )
        })?;
        let path_pattern = match input.endpoint.config() {
            inbound_sync_domain::ProviderConfig::Github {
                path_pattern,
                ..
            } => path_pattern.as_deref(),
            _ => None,
        };
        let scope = github_external_scope(
            input.repository,
            input.branch,
            path_pattern,
        )?;
        let library_repo_id = LibraryRepoId::parse(repo_id)?;
        let binding = match self
            .bindings
            .find_by_scope(
                input.endpoint.tenant_id(),
                &library_repo_id,
                OAuthProvider::Github,
                &scope,
            )
            .await?
        {
            Some(binding) => binding,
            None => {
                // Phase-4 cutover parity: endpoint-based GitHub sync existed
                // before bindings did. Materialize the equivalent binding on
                // the first inbound event so deployment does not require a
                // flag day or make existing endpoints stop receiving.
                let connection = self
                    .connections
                    .find_active_by_tenant(input.endpoint.tenant_id())
                    .await?
                    .into_iter()
                    .find(|connection| {
                        connection.provider() == OAuthProvider::Github
                    })
                    .ok_or_else(|| {
                        errors::Error::service_unavailable(
                            "An active GitHub connection is required to migrate the webhook endpoint",
                        )
                    })?;
                let binding = ExternalSyncBinding::create(
                    input.endpoint.tenant_id().clone(),
                    library_repo_id,
                    OAuthProvider::Github,
                    connection.id().clone(),
                    scope,
                    "markdown_document",
                    json!({"content": "body"}),
                )?;
                self.bindings.save(&binding).await?;
                binding
            }
        };
        if binding.status() != ExternalSyncBindingStatus::Active
            || binding.inbound_policy() == ExternalSyncPolicy::Disabled
        {
            return Err(errors::Error::service_unavailable(
                "GitHub external sync binding is paused",
            ));
        }

        let lookup_path = input.previous_path.unwrap_or(input.path);
        let mut link = self
            .links
            .find_by_external_object(
                input.endpoint.tenant_id(),
                binding.id(),
                lookup_path,
            )
            .await?;
        if link.is_none() {
            // Preserve the existing record identity and accepted revisions
            // when this path was already managed by the legacy SyncState.
            let legacy_external_id =
                format!("{}:{}", input.repository, lookup_path);
            if let Some(state) = self
                .sync_states
                .find_by_external_id(
                    input.endpoint.id(),
                    &legacy_external_id,
                )
                .await?
            {
                let migrated = ExternalObjectLink::new(
                    binding.id().clone(),
                    LibraryDataId::parse(state.data_id().to_owned())?,
                    lookup_path,
                    state.external_version().map(str::to_owned),
                    state.local_version().map(str::to_owned),
                    None,
                )?;
                self.links
                    .save(input.endpoint.tenant_id(), &migrated)
                    .await?;
                link = Some(migrated);
            }
        }
        let change_type = if input.previous_path.is_some() {
            ExternalChangeType::Rename
        } else if input.content.is_some() {
            ExternalChangeType::Upsert
        } else {
            ExternalChangeType::Tombstone
        };
        let mut change_set = InboundChangeSet::create(
            input.endpoint.tenant_id().clone(),
            binding.id().clone(),
            link.as_ref().map(|link| link.data_id().clone()),
            input.path,
            input.external_revision,
            input.base_external_revision.map(str::to_owned),
            change_type,
            json!({
                "repository": input.repository,
                "ref": input.branch,
                "path": input.path,
                "previous_path": input.previous_path,
                "content": input.content,
            }),
        )?;
        if let (Some(link), Some(base_revision)) =
            (&link, input.base_external_revision)
        {
            if link
                .last_accepted_external_revision()
                .is_some_and(|accepted| accepted != base_revision)
            {
                change_set.mark_conflict(
                    "external base revision differs from the last accepted revision",
                )?;
            }
        }
        self.change_sets.save(&change_set).await?;
        Ok(link.is_none())
    }
}
