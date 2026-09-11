use std::sync::Arc;

use database_manager::domain::{Data, Property};
use inbound_sync_domain::{
    Provider, ProviderConfig, SyncStateRepository,
    WebhookEndpointRepository,
};
use integration_domain::{
    ConnectionRepository, ExternalObjectLink, ExternalObjectLinkRepository,
    ExternalScope, ExternalSyncBinding, ExternalSyncBindingRepository,
    ExternalSyncBindingStatus, ExternalSyncPolicy, LibraryDataId,
    LibraryRepoId, OAuthProvider, OutboundDelivery, OutboundDeliveryId,
    OutboundDeliveryRepository,
};
use outbound_sync::{
    SyncDataInputData, SyncDataInputPort, SyncPayload, SyncTarget,
};
use sqlx::MySqlPool;
use tachyon_sdk::auth::{ExecutorAction, MultiTenancyAction};

pub struct ExternalSyncOutboxDispatch {
    library_pool: Arc<MySqlPool>,
    source_pool: Arc<MySqlPool>,
    connections: Arc<dyn ConnectionRepository>,
    webhook_endpoints: Arc<dyn WebhookEndpointRepository>,
    sync_states: Arc<dyn SyncStateRepository>,
    bindings: Arc<dyn ExternalSyncBindingRepository>,
    links: Arc<dyn ExternalObjectLinkRepository>,
    deliveries: Arc<dyn OutboundDeliveryRepository>,
    sync_data: Arc<dyn SyncDataInputPort>,
}

impl std::fmt::Debug for ExternalSyncOutboxDispatch {
    fn fmt(
        &self,
        formatter: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        formatter
            .debug_struct("ExternalSyncOutboxDispatch")
            .finish_non_exhaustive()
    }
}

impl ExternalSyncOutboxDispatch {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        library_pool: Arc<MySqlPool>,
        source_pool: Arc<MySqlPool>,
        connections: Arc<dyn ConnectionRepository>,
        webhook_endpoints: Arc<dyn WebhookEndpointRepository>,
        sync_states: Arc<dyn SyncStateRepository>,
        bindings: Arc<dyn ExternalSyncBindingRepository>,
        links: Arc<dyn ExternalObjectLinkRepository>,
        deliveries: Arc<dyn OutboundDeliveryRepository>,
        sync_data: Arc<dyn SyncDataInputPort>,
    ) -> Self {
        Self {
            library_pool,
            source_pool,
            connections,
            webhook_endpoints,
            sync_states,
            bindings,
            links,
            deliveries,
            sync_data,
        }
    }

    pub async fn capture_and_deliver(
        &self,
        executor: &dyn ExecutorAction,
        multi_tenancy: &dyn MultiTenancyAction,
        data: &Data,
        properties: &[Property],
    ) -> errors::Result<()> {
        let data_id = LibraryDataId::parse(data.id().to_string())?;
        let mut links = self
            .links
            .find_by_tenant_and_data(data.tenant_id(), &data_id)
            .await?;
        if links.is_empty() {
            if let Some(link) =
                self.migrate_legacy_link(data, properties, &data_id).await?
            {
                links.push(link);
            } else {
                return Ok(());
            }
        }
        let event_id: String = sqlx::query_scalar(
            r#"
            SELECT event_id FROM domain_outbox_events
            WHERE tenant_id = ? AND database_id = ?
              AND aggregate_type = 'RECORD' AND aggregate_id = ?
              AND aggregate_version = ? LIMIT 1
            "#,
        )
        .bind(data.tenant_id().to_string())
        .bind(data.database_id().to_string())
        .bind(data.id().to_string())
        .bind(data.record_version().get())
        .fetch_optional(self.source_pool.as_ref())
        .await?
        .ok_or_else(|| {
            errors::Error::service_unavailable(
                "Transactional record outbox event is not available yet",
            )
        })?;
        let markdown =
            super::markdown_composer::compose_markdown(data, properties);

        for link in links {
            let Some(binding) = self
                .bindings
                .find_by_id(data.tenant_id(), link.binding_id())
                .await?
            else {
                continue;
            };
            if binding.status() != ExternalSyncBindingStatus::Active
                || binding.outbound_policy() == ExternalSyncPolicy::Disabled
            {
                continue;
            }
            let mut delivery = OutboundDelivery::create(
                data.tenant_id().clone(),
                binding.id().clone(),
                data_id.clone(),
                link.external_object_id(),
                &event_id,
                link.last_accepted_external_revision().map(str::to_owned),
                serde_json::json!({
                    "content": markdown,
                    "message": format!("chore(library): sync {}", data.name().as_str()),
                }),
            )?;
            if let Some(existing) = self
                .deliveries
                .find_by_id(data.tenant_id(), delivery.id())
                .await?
            {
                let due = existing.next_attempt_at() <= chrono::Utc::now();
                match existing.status() {
                    integration_domain::OutboundDeliveryStatus::Pending => {}
                    integration_domain::OutboundDeliveryStatus::Retrying
                        if due => {}
                    _ => continue,
                }
                delivery = existing;
            }
            self.deliver(
                executor,
                multi_tenancy,
                &binding,
                &link,
                &mut delivery,
            )
            .await?;
        }
        Ok(())
    }

    async fn migrate_legacy_link(
        &self,
        data: &Data,
        properties: &[Property],
        data_id: &LibraryDataId,
    ) -> errors::Result<Option<ExternalObjectLink>> {
        let Some(property) = properties
            .iter()
            .find(|property| property.name() == "ext_github")
        else {
            return Ok(None);
        };
        let Some(meta) = data
            .get_property_data(property.id())
            .and_then(|value| {
                super::ext_github_meta::ExtGithubMeta::parse(
                    &value.string_value(),
                )
            })
            .filter(|meta| meta.enabled)
        else {
            return Ok(None);
        };
        let repo_id: String = sqlx::query_scalar(
            r#"
            SELECT repo.id FROM repos AS repo
            JOIN databases AS database_link ON database_link.repo_id = repo.id
            WHERE repo.org_id = ? AND database_link.database_id = ?
            LIMIT 1
            "#,
        )
        .bind(data.tenant_id().to_string())
        .bind(data.database_id().to_string())
        .fetch_optional(self.library_pool.as_ref())
        .await?
        .ok_or_else(|| errors::Error::not_found("Library repository"))?;
        let connection = self
            .connections
            .find_active_by_tenant(data.tenant_id())
            .await?
            .into_iter()
            .find(|connection| connection.provider() == OAuthProvider::Github)
            .ok_or_else(|| {
                errors::Error::service_unavailable(
                    "An active GitHub connection is required to migrate ext_github",
                )
            })?;
        let scope = ExternalScope::new(serde_json::json!({
            "repository": meta.repo,
            "ref": meta.git_ref,
            "path_pattern": null,
        }))?;
        let library_repo_id = LibraryRepoId::parse(repo_id)?;
        let binding = match self
            .bindings
            .find_by_scope(
                data.tenant_id(),
                &library_repo_id,
                OAuthProvider::Github,
                &scope,
            )
            .await?
        {
            Some(binding) => binding,
            None => {
                let binding = ExternalSyncBinding::create(
                    data.tenant_id().clone(),
                    library_repo_id,
                    OAuthProvider::Github,
                    connection.id().clone(),
                    scope,
                    "markdown_document",
                    serde_json::json!({"content": "body"}),
                )?;
                self.bindings.save(&binding).await?;
                binding
            }
        };
        let legacy_external_id = format!("{}:{}", meta.repo, meta.path);
        let mut accepted_external_revision = None;
        let mut delivered_library_revision = None;
        for endpoint in self
            .webhook_endpoints
            .find_by_tenant_and_provider(data.tenant_id(), Provider::Github)
            .await?
        {
            let matches_repository = matches!(
                endpoint.config(),
                ProviderConfig::Github { repository, .. }
                    if repository == &meta.repo
            );
            if !matches_repository {
                continue;
            }
            if let Some(state) = self
                .sync_states
                .find_by_external_id(endpoint.id(), &legacy_external_id)
                .await?
            {
                accepted_external_revision =
                    state.external_version().map(str::to_owned);
                delivered_library_revision =
                    state.local_version().map(str::to_owned);
                break;
            }
        }
        let link = ExternalObjectLink::new(
            binding.id().clone(),
            data_id.clone(),
            meta.path,
            accepted_external_revision,
            delivered_library_revision,
            None,
        )?;
        self.links.save(data.tenant_id(), &link).await?;
        Ok(Some(link))
    }

    pub async fn retry_delivery(
        &self,
        executor: &dyn ExecutorAction,
        multi_tenancy: &dyn MultiTenancyAction,
        tenant_id: &value_object::TenantId,
        delivery_id: &OutboundDeliveryId,
    ) -> errors::Result<OutboundDelivery> {
        let mut delivery = self
            .deliveries
            .find_by_id(tenant_id, delivery_id)
            .await?
            .ok_or_else(|| errors::Error::not_found("Outbound delivery"))?;
        let binding = self
            .bindings
            .find_by_id(tenant_id, delivery.binding_id())
            .await?
            .ok_or_else(|| {
                errors::Error::not_found("External sync binding")
            })?;
        let link = self
            .links
            .find_by_data(
                tenant_id,
                delivery.binding_id(),
                delivery.data_id(),
            )
            .await?
            .ok_or_else(|| {
                errors::Error::not_found("External object link")
            })?;
        delivery.request_retry_from_base(
            link.last_accepted_external_revision().map(str::to_owned),
            chrono::Utc::now(),
        )?;
        self.deliver(
            executor,
            multi_tenancy,
            &binding,
            &link,
            &mut delivery,
        )
        .await?;
        Ok(delivery)
    }

    async fn deliver(
        &self,
        executor: &dyn ExecutorAction,
        multi_tenancy: &dyn MultiTenancyAction,
        binding: &integration_domain::ExternalSyncBinding,
        link: &ExternalObjectLink,
        delivery: &mut OutboundDelivery,
    ) -> errors::Result<()> {
        self.deliveries.save(delivery).await?;
        delivery.mark_attempt(chrono::Utc::now());
        self.deliveries.save(delivery).await?;
        let scope = binding.external_scope().value();
        let repository = scope["repository"].as_str().ok_or_else(|| {
            errors::Error::invalid("GitHub binding repository is missing")
        })?;
        let branch = scope["ref"].as_str().unwrap_or("main");
        let content =
            delivery.payload()["content"].as_str().ok_or_else(|| {
                errors::Error::invalid("Delivery content is missing")
            })?;
        let message = delivery.payload()["message"]
            .as_str()
            .unwrap_or("chore(library): sync document");
        let result = self
            .sync_data
            .execute(&SyncDataInputData {
                executor,
                multi_tenancy,
                data_id: delivery.data_id().to_string(),
                provider: binding.provider().to_string(),
                target: SyncTarget::git_with_branch(
                    repository,
                    delivery.external_object_id(),
                    branch,
                ),
                payload:
                    SyncPayload::markdown_with_message_and_expected_revision(
                        content,
                        message,
                        delivery.base_external_revision().map(str::to_owned),
                    ),
                dry_run: false,
            })
            .await;
        match result {
            Ok(result) => {
                let remote_revision =
                    result.result_id.ok_or_else(|| {
                        errors::Error::service_unavailable(
                            "Provider did not return a remote revision",
                        )
                    })?;
                delivery.mark_delivered(
                    &remote_revision,
                    result.url,
                    chrono::Utc::now(),
                );
                let updated_link = ExternalObjectLink::new(
                    link.binding_id().clone(),
                    link.data_id().clone(),
                    link.external_object_id(),
                    Some(remote_revision),
                    Some(delivery.library_revision().into()),
                    link.base_content_hash().map(str::to_owned),
                )?;
                self.links
                    .save(delivery.tenant_id(), &updated_link)
                    .await?;
            }
            Err(errors::Error::Conflict { .. }) => {
                delivery.mark_conflict(None, chrono::Utc::now());
            }
            Err(
                error @ (errors::Error::Unauthorized { .. }
                | errors::Error::Forbidden { .. }),
            ) => {
                delivery.mark_failed("authorization", chrono::Utc::now());
                let mut binding = binding.clone();
                binding.set_status(
                    integration_domain::ExternalSyncBindingStatus::ReauthorizationRequired,
                );
                self.bindings.save(&binding).await?;
                tracing::warn!(
                    %error,
                    delivery_id = %delivery.id(),
                    "external sync connection requires reauthorization"
                );
            }
            Err(
                error @ (errors::Error::BadRequest { .. }
                | errors::Error::NotFound { .. }
                | errors::Error::PaymentRequired { .. }),
            ) => {
                delivery
                    .mark_failed("provider_rejected", chrono::Utc::now());
                tracing::warn!(
                    %error,
                    delivery_id = %delivery.id(),
                    "external sync delivery reached a terminal provider error"
                );
            }
            Err(error) => {
                delivery
                    .mark_retry("provider_unavailable", chrono::Utc::now());
                tracing::warn!(
                    %error,
                    delivery_id = %delivery.id(),
                    "external sync delivery will require retry"
                );
            }
        }
        self.deliveries.save(delivery).await
    }
}

#[async_trait::async_trait]
impl inbound_sync::usecase::ExternalDeliveryRetrier
    for ExternalSyncOutboxDispatch
{
    async fn retry(
        &self,
        executor: &dyn ExecutorAction,
        multi_tenancy: &dyn MultiTenancyAction,
        tenant_id: &value_object::TenantId,
        delivery_id: &OutboundDeliveryId,
    ) -> errors::Result<OutboundDelivery> {
        self.retry_delivery(executor, multi_tenancy, tenant_id, delivery_id)
            .await
    }
}
