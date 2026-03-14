//! Square API pull processor for Initial Sync and On-demand Pull.

use std::sync::Arc;

use async_trait::async_trait;

use inbound_sync_domain::{
    ProcessingStats, Provider, ProviderConfig, SyncDirection,
    SyncOperation, SyncState, SyncStateRepository, WebhookEndpoint,
};

use crate::usecase::ApiPullProcessor;

use super::event_processor::{SquareClient, SquareDataHandler};
use super::payload::SquareObjectType;

/// Square API pull processor.
///
/// Handles pulling data from Square API for Initial Sync
/// and On-demand Pull operations.
#[derive(Debug)]
pub struct SquareApiPullProcessor {
    square_client: Arc<dyn SquareClient>,
    data_handler: Arc<dyn SquareDataHandler>,
}

impl SquareApiPullProcessor {
    pub fn new(
        square_client: Arc<dyn SquareClient>,
        data_handler: Arc<dyn SquareDataHandler>,
    ) -> Self {
        Self {
            square_client,
            data_handler,
        }
    }

    /// Pull all catalog items with pagination.
    async fn pull_catalog_items(
        &self,
        endpoint: &WebhookEndpoint,
        sync_state_repo: &Arc<dyn SyncStateRepository>,
        operation: &mut SyncOperation,
    ) -> errors::Result<ProcessingStats> {
        let mut stats = ProcessingStats::default();
        let mut cursor: Option<String> = None;
        let mut page = 0u32;

        loop {
            page += 1;
            operation.update_progress(format!(
                "Fetching catalog items page {page}"
            ));

            let (items, next_cursor) = self
                .square_client
                .list_catalog_items(endpoint.tenant_id(), cursor.as_deref())
                .await?;

            tracing::info!(
                page = page,
                item_count = items.len(),
                "Fetched catalog items page"
            );

            for item in &items {
                let item_stats = self
                    .sync_item(
                        endpoint,
                        sync_state_repo,
                        SquareObjectType::CatalogItem,
                        item,
                    )
                    .await?;
                stats.created += item_stats.created;
                stats.updated += item_stats.updated;
                stats.skipped += item_stats.skipped;
            }

            cursor = next_cursor;
            if cursor.is_none() {
                break;
            }
        }

        Ok(stats)
    }

    /// Pull all customers with pagination.
    async fn pull_customers(
        &self,
        endpoint: &WebhookEndpoint,
        sync_state_repo: &Arc<dyn SyncStateRepository>,
        operation: &mut SyncOperation,
    ) -> errors::Result<ProcessingStats> {
        let mut stats = ProcessingStats::default();
        let mut cursor: Option<String> = None;
        let mut page = 0u32;

        loop {
            page += 1;
            operation
                .update_progress(format!("Fetching customers page {page}"));

            let (customers, next_cursor) = self
                .square_client
                .list_customers(endpoint.tenant_id(), cursor.as_deref())
                .await?;

            tracing::info!(
                page = page,
                customer_count = customers.len(),
                "Fetched customers page"
            );

            for customer in &customers {
                let item_stats = self
                    .sync_item(
                        endpoint,
                        sync_state_repo,
                        SquareObjectType::Customer,
                        customer,
                    )
                    .await?;
                stats.created += item_stats.created;
                stats.updated += item_stats.updated;
                stats.skipped += item_stats.skipped;
            }

            cursor = next_cursor;
            if cursor.is_none() {
                break;
            }
        }

        Ok(stats)
    }

    /// Sync a single item: check SyncState, upsert, save state.
    async fn sync_item(
        &self,
        endpoint: &WebhookEndpoint,
        sync_state_repo: &Arc<dyn SyncStateRepository>,
        object_type: SquareObjectType,
        object: &serde_json::Value,
    ) -> errors::Result<ProcessingStats> {
        let mut stats = ProcessingStats::default();

        let object_id = object
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let external_id =
            format!("square:{}:{}", object_type.as_str(), object_id);

        // Check existing sync state
        let existing = sync_state_repo
            .find_by_external_id(endpoint.id(), &external_id)
            .await?;

        // Check if object has changed via updated_at
        let updated_at = object
            .get("updated_at")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let is_update = existing.is_some();

        if let Some(ref state) = existing {
            if !state.has_external_changed(updated_at) {
                stats.skipped += 1;
                return Ok(stats);
            }
        }

        // Upsert data
        match self
            .data_handler
            .upsert_object(
                endpoint,
                object_type,
                object,
                endpoint.mapping(),
            )
            .await
        {
            Ok(data_id) => {
                let state = if let Some(mut s) = existing {
                    s.update_inbound(
                        Some(updated_at.to_string()),
                        Some(data_id.clone()),
                    );
                    s
                } else {
                    let mut s = SyncState::create(
                        endpoint.id().clone(),
                        &data_id,
                        &external_id,
                        SyncDirection::Inbound,
                    );
                    s.update_inbound(Some(updated_at.to_string()), None);
                    s
                };
                sync_state_repo.save(&state).await?;

                if is_update {
                    stats.updated += 1;
                } else {
                    stats.created += 1;
                }
            }
            Err(e) => {
                tracing::warn!(
                    object_type = object_type.as_str(),
                    object_id = object_id,
                    error = %e,
                    "Failed to upsert Square object"
                );
                stats.skipped += 1;
            }
        }

        Ok(stats)
    }
}

#[async_trait]
impl ApiPullProcessor for SquareApiPullProcessor {
    fn provider(&self) -> Provider {
        Provider::Square
    }

    async fn pull_all(
        &self,
        endpoint: &WebhookEndpoint,
        sync_state_repo: &Arc<dyn SyncStateRepository>,
        operation: &mut SyncOperation,
    ) -> errors::Result<ProcessingStats> {
        let sync_objects = match endpoint.config() {
            ProviderConfig::Square { sync_objects, .. } => {
                sync_objects.clone()
            }
            _ => {
                return Err(errors::Error::invalid(
                    "Invalid provider config for Square",
                ))
            }
        };

        let mut stats = ProcessingStats::default();

        // Pull catalog items if configured (or all by default)
        let should_sync_all = sync_objects.is_empty();
        if should_sync_all
            || sync_objects.contains(&"catalog_item".to_string())
        {
            let s = self
                .pull_catalog_items(endpoint, sync_state_repo, operation)
                .await?;
            stats.created += s.created;
            stats.updated += s.updated;
            stats.skipped += s.skipped;
        }

        // Pull customers
        if should_sync_all || sync_objects.contains(&"customer".to_string())
        {
            let s = self
                .pull_customers(endpoint, sync_state_repo, operation)
                .await?;
            stats.created += s.created;
            stats.updated += s.updated;
            stats.skipped += s.skipped;
        }

        tracing::info!(
            endpoint_id = %endpoint.id(),
            created = stats.created,
            updated = stats.updated,
            skipped = stats.skipped,
            "Square initial sync completed"
        );

        Ok(stats)
    }

    async fn pull_specific(
        &self,
        endpoint: &WebhookEndpoint,
        external_ids: Vec<String>,
        sync_state_repo: &Arc<dyn SyncStateRepository>,
    ) -> errors::Result<ProcessingStats> {
        let mut stats = ProcessingStats::default();

        // Group IDs by type for batch retrieval
        let mut catalog_ids = Vec::new();
        let mut other_ids: Vec<(SquareObjectType, String)> = Vec::new();

        for external_id in &external_ids {
            // Format: "square:{type}:{id}"
            let parts: Vec<&str> = external_id.splitn(3, ':').collect();
            if parts.len() != 3 || parts[0] != "square" {
                tracing::warn!(
                    external_id = %external_id,
                    "Invalid Square external ID format"
                );
                stats.skipped += 1;
                continue;
            }

            let Some(obj_type) = SquareObjectType::parse_str(parts[1])
            else {
                tracing::warn!(
                    object_type = parts[1],
                    "Unknown Square object type"
                );
                stats.skipped += 1;
                continue;
            };

            match obj_type {
                SquareObjectType::CatalogItem
                | SquareObjectType::CatalogCategory
                | SquareObjectType::CatalogItemVariation
                | SquareObjectType::CatalogModifier
                | SquareObjectType::CatalogTax
                | SquareObjectType::CatalogDiscount => {
                    catalog_ids.push((obj_type, parts[2].to_string()));
                }
                _ => {
                    other_ids.push((obj_type, parts[2].to_string()));
                }
            }
        }

        // Batch retrieve catalog objects
        if !catalog_ids.is_empty() {
            let ids: Vec<String> =
                catalog_ids.iter().map(|(_, id)| id.clone()).collect();

            match self
                .square_client
                .batch_retrieve_catalog_objects(endpoint.tenant_id(), &ids)
                .await
            {
                Ok(objects) => {
                    for object in &objects {
                        let obj_type_str = object
                            .get("type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("ITEM");
                        let obj_type =
                            SquareObjectType::parse_str(obj_type_str)
                                .unwrap_or(SquareObjectType::CatalogItem);

                        let s = self
                            .sync_item(
                                endpoint,
                                sync_state_repo,
                                obj_type,
                                object,
                            )
                            .await?;
                        stats.created += s.created;
                        stats.updated += s.updated;
                        stats.skipped += s.skipped;
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "Failed to batch retrieve catalog objects"
                    );
                    stats.skipped += catalog_ids.len() as u32;
                }
            }
        }

        // Fetch other objects individually
        for (obj_type, object_id) in &other_ids {
            match self
                .square_client
                .get_object(endpoint.tenant_id(), *obj_type, object_id)
                .await
            {
                Ok(object) => {
                    let s = self
                        .sync_item(
                            endpoint,
                            sync_state_repo,
                            *obj_type,
                            &object,
                        )
                        .await?;
                    stats.created += s.created;
                    stats.updated += s.updated;
                    stats.skipped += s.skipped;
                }
                Err(e) => {
                    tracing::warn!(
                        object_type = obj_type.as_str(),
                        object_id = %object_id,
                        error = %e,
                        "Failed to fetch Square object"
                    );
                    stats.skipped += 1;
                }
            }
        }

        Ok(stats)
    }
}
