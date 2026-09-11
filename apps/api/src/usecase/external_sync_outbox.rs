use std::sync::Arc;

use chrono::{DateTime, Utc};
use database_manager::{
    domain::{Data, DataId, DatabaseId, Property},
    usecase::{FindAllPropertiesInputData, GetDataInputData},
};
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
use serde::Serialize;
use sqlx::MySqlPool;
use tachyon_sdk::auth::{ExecutorAction, MultiTenancyAction};

const OUTBOX_CONSUMER: &str = "library.external-sync.v1";
const OUTBOX_MAX_ATTEMPTS: u32 = 12;

fn scanner_lease_owner() -> String {
    use rand::RngCore;

    let mut bytes = [0_u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    format!("external-sync-{}", hex::encode(bytes))
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct ClaimedRecordEvent {
    event_id: String,
    tenant_id: String,
    database_id: String,
    aggregate_id: String,
    aggregate_version: u64,
    event_type: String,
}

async fn register_record_events(
    pool: &MySqlPool,
    scan_after: DateTime<Utc>,
    batch_size: u32,
) -> errors::Result<u64> {
    // TiDB cannot reliably plan a join or correlated subquery between these
    // tables while registering deliveries. Read the consumer's ULID cursor in
    // a separate statement, then let the delivery table's unique key make the
    // individual inserts idempotent under concurrent scanners.
    let mut transaction = pool.begin().await?;
    let cursor: Option<String> = sqlx::query_scalar(
        r#"
        SELECT CAST(MAX(event_id) AS CHAR)
        FROM domain_outbox_deliveries
        WHERE consumer_name = ?
        "#,
    )
    .bind(OUTBOX_CONSUMER)
    .fetch_one(&mut *transaction)
    .await?;
    let event_ids: Vec<String> = sqlx::query_scalar(
        r#"
        SELECT CAST(event.event_id AS CHAR)
        FROM domain_outbox_events AS event
        WHERE event.aggregate_type = 'RECORD'
          AND event.occurred_at >= ?
          AND event.event_id > ?
        ORDER BY event.event_id
        LIMIT ?
        "#,
    )
    .bind(scan_after)
    .bind(cursor.unwrap_or_default())
    .bind(batch_size)
    .fetch_all(&mut *transaction)
    .await?;
    let mut registered = 0;
    for event_id in event_ids {
        registered += sqlx::query(
            r#"
            INSERT IGNORE INTO domain_outbox_deliveries (
                event_id, consumer_name
            ) VALUES (?, ?)
            "#,
        )
        .bind(event_id)
        .bind(OUTBOX_CONSUMER)
        .execute(&mut *transaction)
        .await?
        .rows_affected();
    }
    transaction.commit().await?;
    Ok(registered)
}

async fn claim_record_events(
    pool: &MySqlPool,
    lease_owner: &str,
    batch_size: u32,
) -> errors::Result<Vec<ClaimedRecordEvent>> {
    let mut transaction = pool.begin().await?;
    let rows = sqlx::query_as::<_, ClaimedRecordEvent>(
        r#"
        SELECT CAST(event.event_id AS CHAR) AS event_id,
               event.tenant_id, event.database_id,
               event.aggregate_id, event.aggregate_version,
               CAST(event.event_type AS CHAR) AS event_type
        FROM domain_outbox_deliveries AS delivery
        JOIN domain_outbox_events AS event
          ON event.event_id = delivery.event_id
        WHERE delivery.consumer_name = ?
          AND delivery.attempt_count < ?
          AND delivery.next_attempt_at <= NOW(6)
          AND (
            delivery.state = 'PENDING'
            OR (
              delivery.state = 'INFLIGHT'
              AND delivery.lease_expires_at <= NOW(6)
            )
          )
        ORDER BY delivery.next_attempt_at, event.occurred_at,
                 event.event_id
        LIMIT ?
        FOR UPDATE SKIP LOCKED
        "#,
    )
    .bind(OUTBOX_CONSUMER)
    .bind(OUTBOX_MAX_ATTEMPTS)
    .bind(batch_size)
    .fetch_all(&mut *transaction)
    .await?;
    for row in &rows {
        sqlx::query(
            r#"
            UPDATE domain_outbox_deliveries
            SET state = 'INFLIGHT', attempt_count = attempt_count + 1,
                lease_owner = ?,
                lease_expires_at = DATE_ADD(NOW(6), INTERVAL 5 MINUTE),
                last_error = NULL
            WHERE event_id = ? AND consumer_name = ?
            "#,
        )
        .bind(lease_owner)
        .bind(&row.event_id)
        .bind(OUTBOX_CONSUMER)
        .execute(&mut *transaction)
        .await?;
    }
    transaction.commit().await?;
    Ok(rows)
}

async fn complete_record_event(
    pool: &MySqlPool,
    event_id: &str,
    lease_owner: &str,
) -> errors::Result<()> {
    let result = sqlx::query(
        r#"
        UPDATE domain_outbox_deliveries
        SET state = 'DELIVERED', delivered_at = NOW(6),
            lease_owner = NULL, lease_expires_at = NULL,
            last_error = NULL
        WHERE event_id = ? AND consumer_name = ?
          AND state = 'INFLIGHT' AND lease_owner = ?
        "#,
    )
    .bind(event_id)
    .bind(OUTBOX_CONSUMER)
    .bind(lease_owner)
    .execute(pool)
    .await?;
    if result.rows_affected() != 1 {
        return Err(errors::Error::conflict(
            "external sync record-event lease is no longer owned",
        ));
    }
    Ok(())
}

async fn retry_record_event(
    pool: &MySqlPool,
    event_id: &str,
    lease_owner: &str,
    error: &str,
) -> errors::Result<()> {
    let result = sqlx::query(
        r#"
        UPDATE domain_outbox_deliveries
        SET state = IF(attempt_count >= ?, 'DEAD', 'PENDING'),
            next_attempt_at = DATE_ADD(
              NOW(6),
              INTERVAL LEAST(300, POW(2, LEAST(attempt_count, 8))) SECOND
            ),
            lease_owner = NULL, lease_expires_at = NULL,
            last_error = LEFT(?, 4096)
        WHERE event_id = ? AND consumer_name = ?
          AND state = 'INFLIGHT' AND lease_owner = ?
        "#,
    )
    .bind(OUTBOX_MAX_ATTEMPTS)
    .bind(error)
    .bind(event_id)
    .bind(OUTBOX_CONSUMER)
    .bind(lease_owner)
    .execute(pool)
    .await?;
    if result.rows_affected() != 1 {
        return Err(errors::Error::conflict(
            "external sync record-event lease is no longer owned",
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ExternalSyncScanSummary {
    pub registered: u64,
    pub record_events_processed: u32,
    pub record_events_retried: u32,
    pub outbound_deliveries_retried: u32,
}

pub struct ExternalSyncOutboxDispatch {
    library_pool: Arc<MySqlPool>,
    source_pool: Arc<MySqlPool>,
    database: Arc<database_manager::App>,
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
        database: Arc<database_manager::App>,
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
            database,
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
        let event_id: String = sqlx::query_scalar(
            r#"
            SELECT CAST(event_id AS CHAR) FROM domain_outbox_events
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
        self.capture_and_deliver_event(
            executor,
            multi_tenancy,
            data,
            properties,
            &event_id,
            true,
        )
        .await
    }

    async fn capture_and_deliver_event(
        &self,
        executor: &dyn ExecutorAction,
        multi_tenancy: &dyn MultiTenancyAction,
        data: &Data,
        properties: &[Property],
        event_id: &str,
        deliver_now: bool,
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
                event_id,
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
                if !deliver_now {
                    continue;
                }
                let due = existing.next_attempt_at() <= chrono::Utc::now();
                match existing.status() {
                    integration_domain::OutboundDeliveryStatus::Pending => {}
                    integration_domain::OutboundDeliveryStatus::Retrying
                        if due => {}
                    _ => continue,
                }
                delivery = existing;
            }
            if !deliver_now {
                self.deliveries.save(&delivery).await?;
                continue;
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

    /// Recover committed record events that were not captured before the
    /// request process stopped, then retry provider deliveries that are due.
    /// The activation timestamp prevents historical records from being
    /// replayed merely because a binding is created later.
    pub async fn scan_due(
        &self,
        scan_after: DateTime<Utc>,
        event_batch_size: u32,
        delivery_batch_size: u32,
    ) -> errors::Result<ExternalSyncScanSummary> {
        let event_batch_size = event_batch_size.clamp(1, 100);
        let delivery_batch_size = delivery_batch_size.clamp(1, 20);
        let registered = register_record_events(
            self.source_pool.as_ref(),
            scan_after,
            event_batch_size,
        )
        .await?;
        let lease_owner = scanner_lease_owner();
        let events = claim_record_events(
            self.source_pool.as_ref(),
            &lease_owner,
            event_batch_size,
        )
        .await?;
        let mut summary = ExternalSyncScanSummary {
            registered,
            ..Default::default()
        };

        for event in events {
            match self.process_record_event(&event).await {
                Ok(()) => {
                    complete_record_event(
                        self.source_pool.as_ref(),
                        &event.event_id,
                        &lease_owner,
                    )
                    .await?;
                    summary.record_events_processed += 1;
                }
                Err(error) => {
                    tracing::warn!(
                        %error,
                        event_id = %event.event_id,
                        "external sync outbox capture will be retried"
                    );
                    retry_record_event(
                        self.source_pool.as_ref(),
                        &event.event_id,
                        &lease_owner,
                        &error.to_string(),
                    )
                    .await?;
                    summary.record_events_retried += 1;
                }
            }
        }

        let due = self
            .claim_due_outbound_deliveries(delivery_batch_size)
            .await?;
        for delivery in due {
            let executor = inbound_sync::sdk::SystemExecutor;
            let multi_tenancy =
                inbound_sync::sdk::OperatorMultiTenancy::new(
                    delivery.tenant_id().clone(),
                );
            if let Err(error) = self
                .retry_delivery(
                    &executor,
                    &multi_tenancy,
                    delivery.tenant_id(),
                    delivery.id(),
                )
                .await
            {
                tracing::warn!(
                    %error,
                    delivery_id = %delivery.id(),
                    "automatic external delivery retry failed"
                );
                self.release_failed_outbound_claim(delivery.id()).await?;
            }
            summary.outbound_deliveries_retried += 1;
        }
        self.fail_exhausted_outbound_deliveries().await?;
        Ok(summary)
    }

    async fn process_record_event(
        &self,
        event: &ClaimedRecordEvent,
    ) -> errors::Result<()> {
        let latest_version: Option<u64> = sqlx::query_scalar(
            r#"
            SELECT MAX(aggregate_version) FROM domain_outbox_events
            WHERE tenant_id = ? AND database_id = ?
              AND aggregate_type = 'RECORD' AND aggregate_id = ?
            "#,
        )
        .bind(&event.tenant_id)
        .bind(&event.database_id)
        .bind(&event.aggregate_id)
        .fetch_one(self.source_pool.as_ref())
        .await?;
        if latest_version
            .is_some_and(|version| version != event.aggregate_version)
        {
            return Ok(());
        }
        if event.event_type == "database.record.deleted.v1" {
            return Ok(());
        }
        let tenant_id: value_object::TenantId = event.tenant_id.parse()?;
        let database_id = DatabaseId::new(&event.database_id)?;
        let data_id = DataId::new(&event.aggregate_id)?;
        let executor = inbound_sync::sdk::SystemExecutor;
        let multi_tenancy =
            inbound_sync::sdk::OperatorMultiTenancy::new(tenant_id.clone());
        let data = self
            .database
            .get_data_usecase()
            .execute(&GetDataInputData {
                executor: &executor,
                multi_tenancy: &multi_tenancy,
                tenant_id: &tenant_id,
                database_id: &database_id,
                data_id: &data_id,
            })
            .await?;
        if data.record_version().get() != event.aggregate_version {
            return Ok(());
        }
        let properties = self
            .database
            .find_all_properties()
            .execute(FindAllPropertiesInputData {
                tenant_id: tenant_id.clone(),
                database_id,
            })
            .await?;
        self.capture_and_deliver_event(
            &executor,
            &multi_tenancy,
            &data,
            &properties,
            &event.event_id,
            false,
        )
        .await
    }

    async fn claim_due_outbound_deliveries(
        &self,
        batch_size: u32,
    ) -> errors::Result<Vec<OutboundDelivery>> {
        let ids: Vec<String> = sqlx::query_scalar(
            r#"
            SELECT id FROM outbound_deliveries
            WHERE status IN ('pending', 'retrying')
              AND attempt_count < ? AND next_attempt_at <= NOW(6)
            ORDER BY next_attempt_at, id LIMIT ?
            "#,
        )
        .bind(OUTBOX_MAX_ATTEMPTS)
        .bind(batch_size)
        .fetch_all(self.library_pool.as_ref())
        .await?;
        let mut claimed = Vec::new();
        for id in ids {
            let result = sqlx::query(
                r#"
                UPDATE outbound_deliveries
                SET status = 'retrying',
                    next_attempt_at = DATE_ADD(NOW(6), INTERVAL 2 MINUTE),
                    updated_at = NOW(6)
                WHERE id = ? AND status IN ('pending', 'retrying')
                  AND attempt_count < ? AND next_attempt_at <= NOW(6)
                "#,
            )
            .bind(&id)
            .bind(OUTBOX_MAX_ATTEMPTS)
            .execute(self.library_pool.as_ref())
            .await?;
            if result.rows_affected() != 1 {
                continue;
            }
            let id = OutboundDeliveryId::parse(id)?;
            let tenant: String = sqlx::query_scalar(
                "SELECT tenant_id FROM outbound_deliveries WHERE id = ?",
            )
            .bind(id.as_str())
            .fetch_one(self.library_pool.as_ref())
            .await?;
            let tenant_id: value_object::TenantId = tenant.parse()?;
            if let Some(delivery) =
                self.deliveries.find_by_id(&tenant_id, &id).await?
            {
                claimed.push(delivery);
            }
        }
        Ok(claimed)
    }

    async fn fail_exhausted_outbound_deliveries(
        &self,
    ) -> errors::Result<()> {
        sqlx::query(
            r#"
            UPDATE outbound_deliveries
            SET status = 'failed', last_error_category = 'retry_exhausted',
                updated_at = NOW(6)
            WHERE status = 'retrying' AND attempt_count >= ?
            "#,
        )
        .bind(OUTBOX_MAX_ATTEMPTS)
        .execute(self.library_pool.as_ref())
        .await?;
        Ok(())
    }

    async fn release_failed_outbound_claim(
        &self,
        delivery_id: &OutboundDeliveryId,
    ) -> errors::Result<()> {
        sqlx::query(
            r#"
            UPDATE outbound_deliveries
            SET status = IF(attempt_count + 1 >= ?, 'failed', 'retrying'),
                next_attempt_at = DATE_ADD(
                  NOW(6),
                  INTERVAL LEAST(
                    300,
                    POW(2, LEAST(attempt_count + 1, 8))
                  ) SECOND
                ),
                last_error_category = LEFT(?, 64),
                attempt_count = attempt_count + 1,
                updated_at = NOW(6)
            WHERE id = ? AND status = 'retrying'
              AND next_attempt_at > NOW(6)
            "#,
        )
        .bind(OUTBOX_MAX_ATTEMPTS)
        .bind("scanner_precondition")
        .bind(delivery_id.as_str())
        .execute(self.library_pool.as_ref())
        .await?;
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

#[cfg(test)]
mod scanner_database_tests {
    use std::str::FromStr;

    use database_manager::domain::{
        DataId, DatabaseId, RecordEventId, RecordOperationId,
    };
    use sqlx::{mysql::MySqlConnectOptions, MySqlPool};
    use value_object::TenantId;

    use super::{
        claim_record_events, complete_record_event, register_record_events,
        retry_record_event, OUTBOX_CONSUMER,
    };

    #[tokio::test]
    #[ignore = "requires MySQL configured by DEV_DATABASE_URL"]
    async fn scanner_registration_and_lease_recovery_are_durable(
    ) -> anyhow::Result<()> {
        dotenvy::dotenv().ok();
        let base_url =
            std::env::var("DEV_DATABASE_URL").unwrap_or_else(|_| {
                "mysql://root:@127.0.0.1:15000/library".to_string()
            });
        let admin_options =
            MySqlConnectOptions::from_str(&base_url)?.database("mysql");
        let admin = MySqlPool::connect_with(admin_options).await?;
        let database_name = "tachyon_apps_database_manager";
        sqlx::query(
            "CREATE DATABASE IF NOT EXISTS `tachyon_apps_database_manager`",
        )
        .execute(&admin)
        .await?;
        let pool = MySqlPool::connect_with(
            MySqlConnectOptions::from_str(&base_url)?
                .database(database_name),
        )
        .await?;
        sqlx::migrate!("../../packages/database-manager/migrations")
            .run(&pool)
            .await?;

        let operation_id = RecordOperationId::default();
        let event_id = RecordEventId::default();
        let tenant_id = TenantId::default();
        let database_id = DatabaseId::default();
        let data_id = DataId::default();
        sqlx::query(
            r#"
            INSERT INTO record_mutation_operations (
                operation_id, tenant_id, database_id, data_id,
                mutation_kind, actor_kind, actor_id, expected_version,
                fingerprint_version, request_fingerprint
            ) VALUES (?, ?, ?, ?, 'PATCH', 'SYSTEM', 'scanner-test', 1, 1, ?)
            "#,
        )
        .bind(operation_id.to_string())
        .bind(tenant_id.to_string())
        .bind(database_id.to_string())
        .bind(data_id.to_string())
        .bind(vec![0_u8; 32])
        .execute(&pool)
        .await?;
        sqlx::query(
            r#"
            INSERT INTO domain_outbox_events (
                event_id, operation_id, event_sequence, tenant_id,
                database_id, aggregate_type, aggregate_id,
                aggregate_version, event_type, payload, occurred_at
            ) VALUES (?, ?, 1, ?, ?, 'RECORD', ?, 1,
                      'database.record.patched.v1', JSON_OBJECT(), NOW(6))
            "#,
        )
        .bind(event_id.to_string())
        .bind(operation_id.to_string())
        .bind(tenant_id.to_string())
        .bind(database_id.to_string())
        .bind(data_id.to_string())
        .execute(&pool)
        .await?;

        // Docker Desktop can resume the local MySQL VM with a clock a few
        // minutes behind the host, so keep this test cutoff comfortably old.
        let scan_after = chrono::Utc::now() - chrono::Duration::hours(1);
        assert_eq!(register_record_events(&pool, scan_after, 50).await?, 1);
        assert_eq!(register_record_events(&pool, scan_after, 50).await?, 0);

        let first = claim_record_events(&pool, "scanner-a", 50).await?;
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].event_id, event_id.to_string());
        assert!(claim_record_events(&pool, "scanner-b", 50)
            .await?
            .is_empty());

        sqlx::query(
            r#"
            UPDATE domain_outbox_deliveries
            SET lease_expires_at = DATE_SUB(NOW(6), INTERVAL 1 SECOND)
            WHERE event_id = ? AND consumer_name = ?
            "#,
        )
        .bind(event_id.to_string())
        .bind(OUTBOX_CONSUMER)
        .execute(&pool)
        .await?;
        assert_eq!(
            claim_record_events(&pool, "scanner-b", 50).await?.len(),
            1
        );

        retry_record_event(
            &pool,
            event_id.as_ref(),
            "scanner-b",
            "temporary failure",
        )
        .await?;
        let (state, attempts, last_error): (String, u32, Option<String>) =
            sqlx::query_as(
                r#"
                SELECT CAST(state AS CHAR) AS state,
                       attempt_count, last_error
                FROM domain_outbox_deliveries
                WHERE event_id = ? AND consumer_name = ?
                "#,
            )
            .bind(event_id.to_string())
            .bind(OUTBOX_CONSUMER)
            .fetch_one(&pool)
            .await?;
        assert_eq!(state, "PENDING");
        assert_eq!(attempts, 2);
        assert_eq!(last_error.as_deref(), Some("temporary failure"));

        sqlx::query(
            r#"
            UPDATE domain_outbox_deliveries
            SET next_attempt_at = DATE_SUB(NOW(6), INTERVAL 1 SECOND)
            WHERE event_id = ? AND consumer_name = ?
            "#,
        )
        .bind(event_id.to_string())
        .bind(OUTBOX_CONSUMER)
        .execute(&pool)
        .await?;
        assert_eq!(
            claim_record_events(&pool, "scanner-c", 50).await?.len(),
            1
        );
        complete_record_event(&pool, event_id.as_ref(), "scanner-c")
            .await?;
        let (state, attempts, lease_owner): (String, u32, Option<String>) =
            sqlx::query_as(
                r#"
                SELECT CAST(state AS CHAR) AS state,
                       attempt_count, CAST(lease_owner AS CHAR) AS lease_owner
                FROM domain_outbox_deliveries
                WHERE event_id = ? AND consumer_name = ?
                "#,
            )
            .bind(event_id.to_string())
            .bind(OUTBOX_CONSUMER)
            .fetch_one(&pool)
            .await?;
        assert_eq!(state, "DELIVERED");
        assert_eq!(attempts, 3);
        assert!(lease_owner.is_none());

        sqlx::query(
            "DELETE FROM domain_outbox_deliveries WHERE event_id = ?",
        )
        .bind(event_id.to_string())
        .execute(&pool)
        .await?;
        sqlx::query("DELETE FROM domain_outbox_events WHERE event_id = ?")
            .bind(event_id.to_string())
            .execute(&pool)
            .await?;
        sqlx::query(
            "DELETE FROM record_mutation_operations WHERE operation_id = ?",
        )
        .bind(operation_id.to_string())
        .execute(&pool)
        .await?;
        pool.close().await;
        admin.close().await;
        Ok(())
    }
}
