use std::sync::Arc;

use integration_domain::{
    ExternalChangeType, ExternalObjectLink, ExternalObjectLinkRepository,
    ExternalSyncBindingRepository, InboundChangeSet, InboundChangeSetId,
    InboundChangeSetRepository, LibraryDataId, OAuthProvider,
};
use sha2::{Digest, Sha256};
use tachyon_sdk::auth::{ExecutorAction, MultiTenancyAction};
use value_object::TenantId;

use crate::{
    providers::github::GitHubDataHandler, WebhookEndpointRepository,
};

#[derive(Debug, Clone, Copy)]
pub enum InboundChangeSetDecision {
    Accept,
    Reject,
}

#[async_trait::async_trait]
pub trait ExternalDeliveryRetrier: Send + Sync + std::fmt::Debug {
    async fn retry(
        &self,
        executor: &dyn ExecutorAction,
        multi_tenancy: &dyn MultiTenancyAction,
        tenant_id: &TenantId,
        delivery_id: &integration_domain::OutboundDeliveryId,
    ) -> errors::Result<integration_domain::OutboundDelivery>;
}

#[derive(Debug)]
pub struct DecideInboundChangeSet {
    bindings: Arc<dyn ExternalSyncBindingRepository>,
    links: Arc<dyn ExternalObjectLinkRepository>,
    change_sets: Arc<dyn InboundChangeSetRepository>,
    endpoints: Arc<dyn WebhookEndpointRepository>,
    github_data: Arc<dyn GitHubDataHandler>,
}

impl DecideInboundChangeSet {
    pub fn new(
        bindings: Arc<dyn ExternalSyncBindingRepository>,
        links: Arc<dyn ExternalObjectLinkRepository>,
        change_sets: Arc<dyn InboundChangeSetRepository>,
        endpoints: Arc<dyn WebhookEndpointRepository>,
        github_data: Arc<dyn GitHubDataHandler>,
    ) -> Self {
        Self {
            bindings,
            links,
            change_sets,
            endpoints,
            github_data,
        }
    }

    pub async fn execute(
        &self,
        tenant_id: &TenantId,
        id: &InboundChangeSetId,
        decision: InboundChangeSetDecision,
        note: Option<String>,
    ) -> errors::Result<InboundChangeSet> {
        let mut change_set = self
            .change_sets
            .find_by_id(tenant_id, id)
            .await?
            .ok_or_else(|| {
                errors::Error::not_found("Inbound change set")
            })?;

        match decision {
            InboundChangeSetDecision::Reject => {
                change_set.reject(note)?;
            }
            InboundChangeSetDecision::Accept => {
                // Validate the transition before applying content. A duplicate
                // accept must never overwrite later Library edits or a rejected
                // change before reporting its terminal-decision conflict.
                change_set.accept(note)?;
                self.apply_github_change(tenant_id, &change_set).await?;
            }
        }
        self.change_sets.save(&change_set).await?;
        Ok(change_set)
    }

    async fn apply_github_change(
        &self,
        tenant_id: &TenantId,
        change_set: &InboundChangeSet,
    ) -> errors::Result<()> {
        let binding = self
            .bindings
            .find_by_id(tenant_id, change_set.binding_id())
            .await?
            .ok_or_else(|| {
                errors::Error::not_found("External sync binding")
            })?;
        if binding.provider() != OAuthProvider::Github {
            return Err(errors::Error::invalid(
                "No acceptance adapter is registered for this provider",
            ));
        }

        // A reviewed tombstone advances the accepted external revision and
        // remains audit state. It never turns into an implicit hard delete.
        if change_set.change_type() == ExternalChangeType::Tombstone {
            if let Some(data_id) = change_set.data_id() {
                if let Some(previous) = self
                    .links
                    .find_by_data(
                        tenant_id,
                        change_set.binding_id(),
                        data_id,
                    )
                    .await?
                {
                    self.links
                        .save(
                            tenant_id,
                            &ExternalObjectLink::new(
                                previous.binding_id().clone(),
                                previous.data_id().clone(),
                                change_set.external_object_id(),
                                Some(change_set.external_revision().into()),
                                previous
                                    .last_delivered_library_revision()
                                    .map(str::to_owned),
                                previous
                                    .base_content_hash()
                                    .map(str::to_owned),
                            )?,
                        )
                        .await?;
                }
            }
            return Ok(());
        }

        let payload = change_set.payload();
        let repository =
            payload["repository"].as_str().ok_or_else(|| {
                errors::Error::invalid("Change set repository is missing")
            })?;
        let branch = payload["ref"].as_str().ok_or_else(|| {
            errors::Error::invalid("Change set ref is missing")
        })?;
        let content = payload["content"].as_str().ok_or_else(|| {
            errors::Error::invalid("Change set content is missing")
        })?;
        let endpoint = self
            .endpoints
            .find_by_tenant_and_provider(
                tenant_id,
                inbound_sync_domain::Provider::Github,
            )
            .await?
            .into_iter()
            .find(|endpoint| {
                endpoint.repository_id()
                    == Some(binding.library_repo_id().as_str())
                    && matches!(
                        endpoint.config(),
                        inbound_sync_domain::ProviderConfig::Github {
                            repository: configured_repository,
                            branch: configured_branch,
                            ..
                        } if configured_repository == repository
                            && configured_branch == branch
                    )
            })
            .ok_or_else(|| {
                errors::Error::not_found("GitHub webhook endpoint")
            })?;
        let data_id = if let Some(existing_id) = change_set.data_id() {
            self.github_data
                .update_linked_data(
                    &endpoint,
                    existing_id.as_str(),
                    change_set.external_object_id(),
                    content,
                    endpoint.mapping(),
                )
                .await?
        } else {
            self.github_data
                .upsert_data(
                    &endpoint,
                    change_set.external_object_id(),
                    content,
                    endpoint.mapping(),
                )
                .await?
        };
        let previous = if let Some(existing_id) = change_set.data_id() {
            self.links
                .find_by_data(
                    tenant_id,
                    change_set.binding_id(),
                    existing_id,
                )
                .await?
        } else {
            None
        };
        let link = ExternalObjectLink::new(
            change_set.binding_id().clone(),
            LibraryDataId::parse(data_id)?,
            change_set.external_object_id(),
            Some(change_set.external_revision().into()),
            previous
                .as_ref()
                .and_then(|link| link.last_delivered_library_revision())
                .map(str::to_owned),
            Some(format!("{:x}", Sha256::digest(content.as_bytes()))),
        )?;
        self.links.save(tenant_id, &link).await
    }
}

#[cfg(test)]
#[path = "external_sync_tests.rs"]
mod tests;
