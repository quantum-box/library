//! Square event processor implementation.

use std::sync::Arc;

use async_trait::async_trait;

use inbound_sync_domain::{
    ProcessingStats, Provider, ProviderConfig, SyncDirection, SyncState,
    SyncStateRepository, WebhookEndpoint, WebhookEvent,
};

use crate::usecase::EventProcessor;

use super::payload::{EventAction, SquareEvent, SquareObjectType};

/// Square event processor.
///
/// Handles Square webhook events:
/// - catalog.version.updated (triggers catalog sync)
/// - customer.created / customer.updated / customer.deleted
/// - order.created / order.updated / order.fulfillment_updated
/// - payment.created / payment.updated
/// - inventory.count.updated
#[derive(Debug)]
pub struct SquareEventProcessor {
    square_client: Arc<dyn SquareClient>,
    sync_state_repo: Arc<dyn SyncStateRepository>,
    data_handler: Arc<dyn SquareDataHandler>,
}

impl SquareEventProcessor {
    pub fn new(
        square_client: Arc<dyn SquareClient>,
        sync_state_repo: Arc<dyn SyncStateRepository>,
        data_handler: Arc<dyn SquareDataHandler>,
    ) -> Self {
        Self {
            square_client,
            sync_state_repo,
            data_handler,
        }
    }

    /// Process a Square event.
    async fn process_square_event(
        &self,
        square_event: &SquareEvent,
        endpoint: &WebhookEndpoint,
    ) -> errors::Result<ProcessingStats> {
        let mut stats = ProcessingStats::default();

        // Get Square config
        let (sync_objects, fetch_latest) = match endpoint.config() {
            ProviderConfig::Square {
                sync_objects,
                fetch_latest,
            } => (sync_objects, fetch_latest),
            _ => {
                return Err(errors::Error::invalid(
                    "Invalid provider config for Square",
                ))
            }
        };

        // Parse event type
        let Some((object_type, action)) = square_event.parse_event_type()
        else {
            tracing::debug!(
                event_type = %square_event.event_type,
                "Unknown event type"
            );
            stats.skipped += 1;
            return Ok(stats);
        };

        // Handle catalog.version.updated specially (batch sync).
        // This event triggers a full catalog sync, so skip
        // sync_objects filtering (the object type is "catalog"
        // but users configure "catalog_item" etc.).
        if matches!(object_type, SquareObjectType::Catalog)
            && matches!(action, EventAction::VersionUpdated)
        {
            return self
                .handle_catalog_version_updated(square_event, endpoint)
                .await;
        }

        // Check if this object type is configured
        if !sync_objects.is_empty()
            && !sync_objects.contains(&object_type.as_str().to_string())
        {
            tracing::debug!(
                object_type = object_type.as_str(),
                "Object type not in configured types, skipping"
            );
            return Ok(stats);
        }

        let Some(object_id) = square_event.get_object_id() else {
            tracing::warn!("No object ID in Square event");
            stats.skipped += 1;
            return Ok(stats);
        };

        let external_id =
            format!("square:{}:{}", object_type.as_str(), object_id);

        match action {
            EventAction::Created => {
                // Get the object data
                let object_data = if *fetch_latest {
                    self.square_client
                        .get_object(
                            endpoint.tenant_id(),
                            object_type,
                            object_id,
                        )
                        .await?
                } else {
                    square_event.get_object().clone()
                };

                let data_id = self
                    .data_handler
                    .upsert_object(
                        endpoint,
                        object_type,
                        &object_data,
                        endpoint.mapping(),
                    )
                    .await?;

                let state = SyncState::create(
                    endpoint.id().clone(),
                    &data_id,
                    &external_id,
                    SyncDirection::Inbound,
                );
                self.sync_state_repo.save(&state).await?;

                tracing::info!(
                    object_type = object_type.as_str(),
                    object_id = object_id,
                    data_id = data_id,
                    "Square object created"
                );
                stats.created += 1;
            }
            EventAction::Updated
            | EventAction::FulfillmentUpdated
            | EventAction::PaymentMade
            | EventAction::CountUpdated => {
                let existing_state = self
                    .sync_state_repo
                    .find_by_external_id(endpoint.id(), &external_id)
                    .await?;

                // Get the object data
                let object_data = if *fetch_latest {
                    self.square_client
                        .get_object(
                            endpoint.tenant_id(),
                            object_type,
                            object_id,
                        )
                        .await?
                } else {
                    square_event.get_object().clone()
                };

                let data_id = self
                    .data_handler
                    .upsert_object(
                        endpoint,
                        object_type,
                        &object_data,
                        endpoint.mapping(),
                    )
                    .await?;

                let state = if let Some(mut existing) = existing_state {
                    existing.update_inbound(None, Some(data_id.clone()));
                    existing
                } else {
                    // Object was created outside of webhook, create sync state
                    SyncState::create(
                        endpoint.id().clone(),
                        &data_id,
                        &external_id,
                        SyncDirection::Inbound,
                    )
                };
                self.sync_state_repo.save(&state).await?;

                tracing::info!(
                    object_type = object_type.as_str(),
                    object_id = object_id,
                    data_id = data_id,
                    "Square object updated"
                );
                stats.updated += 1;
            }
            EventAction::Deleted => {
                let existing_state = self
                    .sync_state_repo
                    .find_by_external_id(endpoint.id(), &external_id)
                    .await?;

                if let Some(state) = existing_state {
                    self.data_handler
                        .delete_object(
                            endpoint,
                            object_type,
                            state.data_id(),
                        )
                        .await?;
                    self.sync_state_repo.delete(state.id()).await?;

                    tracing::info!(
                        object_type = object_type.as_str(),
                        object_id = object_id,
                        "Square object deleted"
                    );
                    stats.deleted += 1;
                } else {
                    tracing::debug!(
                        object_type = object_type.as_str(),
                        object_id = object_id,
                        "No sync state for deleted object, skipping"
                    );
                    stats.skipped += 1;
                }
            }
            EventAction::VersionUpdated => {
                // Already handled above for Catalog type
                stats.skipped += 1;
            }
        }

        Ok(stats)
    }

    /// Handle catalog.version.updated event.
    ///
    /// Fetches all catalog items via the Square API and
    /// compares with existing SyncState to detect changes.
    async fn handle_catalog_version_updated(
        &self,
        square_event: &SquareEvent,
        endpoint: &WebhookEndpoint,
    ) -> errors::Result<ProcessingStats> {
        let mut stats = ProcessingStats::default();

        let catalog_version = square_event
            .get_object()
            .get("catalog_version")
            .and_then(|v| v.as_i64());

        tracing::info!(
            catalog_version = ?catalog_version,
            merchant_id = %square_event.merchant_id,
            "Processing catalog version update - fetching all items"
        );

        // Paginate through all catalog items
        let mut cursor: Option<String> = None;
        loop {
            let (items, next_cursor) = self
                .square_client
                .list_catalog_items(endpoint.tenant_id(), cursor.as_deref())
                .await?;

            for item in &items {
                let object_id = item
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");

                let object_type = item
                    .get("type")
                    .and_then(|v| v.as_str())
                    .and_then(SquareObjectType::parse_str)
                    .unwrap_or(SquareObjectType::CatalogItem);

                let external_id = format!(
                    "square:{}:{}",
                    object_type.as_str(),
                    object_id
                );

                // Check existing sync state
                let existing = self
                    .sync_state_repo
                    .find_by_external_id(endpoint.id(), &external_id)
                    .await?;

                let is_update = existing.is_some();
                let updated_at = item
                    .get("updated_at")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                // Skip if unchanged
                if let Some(ref state) = existing {
                    if !state.has_external_changed(updated_at) {
                        stats.skipped += 1;
                        continue;
                    }
                }

                // Upsert the item
                let data_id = self
                    .data_handler
                    .upsert_object(
                        endpoint,
                        object_type,
                        item,
                        endpoint.mapping(),
                    )
                    .await?;

                let state = if let Some(mut s) = existing {
                    s.update_inbound(
                        Some(updated_at.to_string()),
                        Some(data_id.clone()),
                    );
                    s
                } else {
                    SyncState::create(
                        endpoint.id().clone(),
                        &data_id,
                        &external_id,
                        SyncDirection::Inbound,
                    )
                };
                self.sync_state_repo.save(&state).await?;

                if is_update {
                    stats.updated += 1;
                } else {
                    stats.created += 1;
                }
            }

            cursor = next_cursor;
            if cursor.is_none() {
                break;
            }
        }

        tracing::info!(
            created = stats.created,
            updated = stats.updated,
            skipped = stats.skipped,
            "Catalog version update sync completed"
        );

        Ok(stats)
    }
}

#[async_trait]
impl EventProcessor for SquareEventProcessor {
    fn provider(&self) -> Provider {
        Provider::Square
    }

    async fn process(
        &self,
        event: &WebhookEvent,
        endpoint: &WebhookEndpoint,
    ) -> errors::Result<ProcessingStats> {
        // Parse the Square event
        let square_event: SquareEvent = serde_json::from_value(
            event.payload().clone(),
        )
        .map_err(|e| {
            errors::Error::invalid(format!(
                "Failed to parse Square webhook event: {e}"
            ))
        })?;

        self.process_square_event(&square_event, endpoint).await
    }
}

/// Trait for Square API client.
///
/// The client is designed to work with OAuth tokens. Implementations can either:
/// 1. Use a pre-configured access token (SquareApiClient)
/// 2. Dynamically fetch tokens via OAuthTokenProvider (OAuthSquareClient)
#[async_trait]
pub trait SquareClient: Send + Sync + std::fmt::Debug {
    /// Get an object by type and ID.
    ///
    /// # Arguments
    ///
    /// * `tenant_id` - The tenant ID for OAuth token retrieval
    /// * `object_type` - The Square object type
    /// * `object_id` - The Square object ID
    async fn get_object(
        &self,
        tenant_id: &value_object::TenantId,
        object_type: SquareObjectType,
        object_id: &str,
    ) -> errors::Result<serde_json::Value>;

    /// Batch retrieve catalog objects.
    ///
    /// # Arguments
    ///
    /// * `tenant_id` - The tenant ID for OAuth token retrieval
    /// * `object_ids` - List of catalog object IDs to retrieve
    async fn batch_retrieve_catalog_objects(
        &self,
        tenant_id: &value_object::TenantId,
        object_ids: &[String],
    ) -> errors::Result<Vec<serde_json::Value>>;

    /// List catalog items with pagination.
    ///
    /// Returns a tuple of (items, next_cursor).
    async fn list_catalog_items(
        &self,
        tenant_id: &value_object::TenantId,
        cursor: Option<&str>,
    ) -> errors::Result<(Vec<serde_json::Value>, Option<String>)>;

    /// List customers with pagination.
    ///
    /// Returns a tuple of (customers, next_cursor).
    async fn list_customers(
        &self,
        tenant_id: &value_object::TenantId,
        cursor: Option<&str>,
    ) -> errors::Result<(Vec<serde_json::Value>, Option<String>)>;

    /// List orders with pagination.
    ///
    /// Requires location IDs to search orders.
    /// Returns a tuple of (orders, next_cursor).
    async fn list_orders(
        &self,
        tenant_id: &value_object::TenantId,
        location_ids: &[String],
        cursor: Option<&str>,
    ) -> errors::Result<(Vec<serde_json::Value>, Option<String>)>;
}

/// Trait for handling Library data operations for Square.
#[async_trait]
pub trait SquareDataHandler: Send + Sync + std::fmt::Debug {
    /// Upsert a Square object in Library.
    ///
    /// Returns the Library data ID.
    async fn upsert_object(
        &self,
        endpoint: &WebhookEndpoint,
        object_type: SquareObjectType,
        object: &serde_json::Value,
        mapping: Option<&inbound_sync_domain::PropertyMapping>,
    ) -> errors::Result<String>;

    /// Delete a Square object from Library.
    async fn delete_object(
        &self,
        endpoint: &WebhookEndpoint,
        object_type: SquareObjectType,
        data_id: &str,
    ) -> errors::Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_external_id_format() {
        let object_type = SquareObjectType::CatalogItem;
        let object_id = "ITEM_12345";
        let external_id =
            format!("square:{}:{}", object_type.as_str(), object_id);
        assert_eq!(external_id, "square:catalog_item:ITEM_12345");
    }

    #[test]
    fn test_external_id_format_customer() {
        let object_type = SquareObjectType::Customer;
        let object_id = "CUST_ABC123";
        let external_id =
            format!("square:{}:{}", object_type.as_str(), object_id);
        assert_eq!(external_id, "square:customer:CUST_ABC123");
    }
}
