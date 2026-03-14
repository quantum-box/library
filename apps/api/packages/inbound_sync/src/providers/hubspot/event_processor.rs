//! HubSpot event processor implementation.

use std::sync::Arc;

use async_trait::async_trait;

use inbound_sync_domain::{
    ProcessingStats, Provider, ProviderConfig, SyncDirection, SyncState,
    SyncStateRepository, WebhookEndpoint, WebhookEvent,
};

use crate::usecase::EventProcessor;

use super::payload::{
    EventAction, HubSpotEvent, HubSpotObject, HubSpotWebhookPayload,
    ObjectType,
};

/// HubSpot event processor.
///
/// Handles HubSpot CRM webhook events:
/// - contact.creation / contact.deletion / contact.propertyChange
/// - company.creation / company.deletion / company.propertyChange
/// - deal.creation / deal.deletion / deal.propertyChange
/// - product.creation / product.deletion / product.propertyChange
#[derive(Debug)]
pub struct HubSpotEventProcessor {
    hubspot_client: Arc<dyn HubSpotClient>,
    sync_state_repo: Arc<dyn SyncStateRepository>,
    data_handler: Arc<dyn HubSpotDataHandler>,
}

impl HubSpotEventProcessor {
    pub fn new(
        hubspot_client: Arc<dyn HubSpotClient>,
        sync_state_repo: Arc<dyn SyncStateRepository>,
        data_handler: Arc<dyn HubSpotDataHandler>,
    ) -> Self {
        Self {
            hubspot_client,
            sync_state_repo,
            data_handler,
        }
    }

    /// Process a single HubSpot event.
    async fn process_event(
        &self,
        event: &HubSpotEvent,
        endpoint: &WebhookEndpoint,
    ) -> errors::Result<ProcessingStats> {
        let mut stats = ProcessingStats::default();

        // Get HubSpot config
        let (portal_id, object_types) = match endpoint.config() {
            ProviderConfig::Hubspot {
                portal_id,
                object_types,
            } => (portal_id, object_types),
            _ => {
                return Err(errors::Error::invalid(
                    "Invalid provider config for HubSpot",
                ))
            }
        };

        // Verify portal ID
        if event.portal_id.to_string() != *portal_id {
            tracing::warn!(
                event_portal = event.portal_id,
                configured_portal = portal_id,
                "Portal ID mismatch"
            );
            return Ok(stats);
        }

        // Parse subscription type
        let Some((object_type, action)) = event.parse_subscription_type()
        else {
            tracing::debug!(
                subscription_type = %event.subscription_type,
                "Unknown subscription type"
            );
            stats.skipped += 1;
            return Ok(stats);
        };

        // Check if this object type is configured
        if !object_types.is_empty()
            && !object_types.contains(&object_type.as_str().to_string())
        {
            tracing::debug!(
                object_type = object_type.as_str(),
                "Object type not in configured types, skipping"
            );
            return Ok(stats);
        }

        let external_id =
            format!("hubspot:{}:{}", object_type.as_str(), event.object_id);

        match action {
            EventAction::Creation | EventAction::Restore => {
                // Fetch the full object from HubSpot API
                let object = self
                    .hubspot_client
                    .get_object(
                        endpoint.tenant_id(),
                        object_type,
                        event.object_id,
                    )
                    .await?;

                let data_id = self
                    .data_handler
                    .upsert_object(
                        endpoint,
                        object_type,
                        &object,
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
                    object_id = event.object_id,
                    data_id = data_id,
                    "HubSpot object created"
                );
                stats.created += 1;
            }
            EventAction::PropertyChange | EventAction::Merge => {
                let existing_state = self
                    .sync_state_repo
                    .find_by_external_id(endpoint.id(), &external_id)
                    .await?;

                // Fetch the full object from HubSpot API
                let object = self
                    .hubspot_client
                    .get_object(
                        endpoint.tenant_id(),
                        object_type,
                        event.object_id,
                    )
                    .await?;

                let data_id = self
                    .data_handler
                    .upsert_object(
                        endpoint,
                        object_type,
                        &object,
                        endpoint.mapping(),
                    )
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
                    object_type = object_type.as_str(),
                    object_id = event.object_id,
                    property = ?event.property_name,
                    data_id = data_id,
                    "HubSpot object updated"
                );
                stats.updated += 1;
            }
            EventAction::Deletion => {
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
                        object_id = event.object_id,
                        "HubSpot object deleted"
                    );
                    stats.deleted += 1;
                } else {
                    tracing::debug!(
                        object_type = object_type.as_str(),
                        object_id = event.object_id,
                        "No sync state for deleted object, skipping"
                    );
                    stats.skipped += 1;
                }
            }
            EventAction::AssociationChange => {
                // Association changes can be processed if needed
                tracing::debug!("Association changes not yet implemented");
                stats.skipped += 1;
            }
        }

        Ok(stats)
    }
}

#[async_trait]
impl EventProcessor for HubSpotEventProcessor {
    fn provider(&self) -> Provider {
        Provider::Hubspot
    }

    async fn process(
        &self,
        event: &WebhookEvent,
        endpoint: &WebhookEndpoint,
    ) -> errors::Result<ProcessingStats> {
        // HubSpot sends an array of events
        let hubspot_events: HubSpotWebhookPayload = serde_json::from_value(
            event.payload().clone(),
        )
        .map_err(|e| {
            errors::Error::invalid(format!(
                "Failed to parse HubSpot webhook payload: {e}"
            ))
        })?;

        let mut total_stats = ProcessingStats::default();

        for hubspot_event in &hubspot_events {
            match self.process_event(hubspot_event, endpoint).await {
                Ok(stats) => {
                    total_stats.created += stats.created;
                    total_stats.updated += stats.updated;
                    total_stats.deleted += stats.deleted;
                    total_stats.skipped += stats.skipped;
                }
                Err(e) => {
                    tracing::error!(
                        event_id = hubspot_event.event_id,
                        error = %e,
                        "Failed to process HubSpot event"
                    );
                    total_stats.skipped += 1;
                }
            }
        }

        Ok(total_stats)
    }
}

/// Trait for HubSpot API client.
#[async_trait]
pub trait HubSpotClient: Send + Sync + std::fmt::Debug {
    /// Get an object by type and ID.
    async fn get_object(
        &self,
        tenant_id: &value_object::TenantId,
        object_type: ObjectType,
        object_id: i64,
    ) -> errors::Result<HubSpotObject>;
}

/// Trait for handling Library data operations for HubSpot.
#[async_trait]
pub trait HubSpotDataHandler: Send + Sync + std::fmt::Debug {
    /// Upsert a HubSpot object in Library.
    ///
    /// Returns the Library data ID.
    async fn upsert_object(
        &self,
        endpoint: &WebhookEndpoint,
        object_type: ObjectType,
        object: &HubSpotObject,
        mapping: Option<&inbound_sync_domain::PropertyMapping>,
    ) -> errors::Result<String>;

    /// Delete a HubSpot object from Library.
    async fn delete_object(
        &self,
        endpoint: &WebhookEndpoint,
        object_type: ObjectType,
        data_id: &str,
    ) -> errors::Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_external_id_format() {
        let object_type = ObjectType::Contact;
        let object_id = 12345i64;
        let external_id =
            format!("hubspot:{}:{}", object_type.as_str(), object_id);
        assert_eq!(external_id, "hubspot:contact:12345");
    }
}
