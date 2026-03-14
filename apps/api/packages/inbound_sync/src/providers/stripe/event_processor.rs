//! Stripe event processor implementation.

use std::sync::Arc;

use async_trait::async_trait;

use inbound_sync_domain::{
    ProcessingStats, Provider, ProviderConfig, SyncDirection, SyncState,
    SyncStateRepository, WebhookEndpoint, WebhookEvent,
};

use crate::usecase::EventProcessor;

use super::payload::{EventAction, StripeEvent, StripeObjectType};

/// Stripe event processor.
///
/// Handles Stripe webhook events:
/// - product.created / product.updated / product.deleted
/// - price.created / price.updated / price.deleted
/// - customer.created / customer.updated / customer.deleted (optional)
#[derive(Debug)]
pub struct StripeEventProcessor {
    stripe_client: Arc<dyn StripeClient>,
    sync_state_repo: Arc<dyn SyncStateRepository>,
    data_handler: Arc<dyn StripeDataHandler>,
}

impl StripeEventProcessor {
    pub fn new(
        stripe_client: Arc<dyn StripeClient>,
        sync_state_repo: Arc<dyn SyncStateRepository>,
        data_handler: Arc<dyn StripeDataHandler>,
    ) -> Self {
        Self {
            stripe_client,
            sync_state_repo,
            data_handler,
        }
    }

    /// Process a Stripe event.
    async fn process_stripe_event(
        &self,
        stripe_event: &StripeEvent,
        endpoint: &WebhookEndpoint,
    ) -> errors::Result<ProcessingStats> {
        let mut stats = ProcessingStats::default();

        // Get Stripe config
        let (sync_objects, fetch_latest) = match endpoint.config() {
            ProviderConfig::Stripe {
                sync_objects,
                fetch_latest,
            } => (sync_objects, fetch_latest),
            _ => {
                return Err(errors::Error::invalid(
                    "Invalid provider config for Stripe",
                ))
            }
        };

        // Parse event type
        let Some((object_type, action)) = stripe_event.parse_event_type()
        else {
            tracing::debug!(
                event_type = %stripe_event.event_type,
                "Unknown event type"
            );
            stats.skipped += 1;
            return Ok(stats);
        };

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

        let Some(object_id) = stripe_event.get_object_id() else {
            tracing::warn!("No object ID in Stripe event");
            stats.skipped += 1;
            return Ok(stats);
        };

        let external_id =
            format!("stripe:{}:{}", object_type.as_str(), object_id);

        match action {
            EventAction::Created => {
                // Get the object data
                let object_data = if *fetch_latest {
                    self.stripe_client
                        .get_object(
                            endpoint.tenant_id(),
                            object_type,
                            object_id,
                        )
                        .await?
                } else {
                    stripe_event.get_object().clone()
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
                    "Stripe object created"
                );
                stats.created += 1;
            }
            EventAction::Updated => {
                let existing_state = self
                    .sync_state_repo
                    .find_by_external_id(endpoint.id(), &external_id)
                    .await?;

                // Get the object data
                let object_data = if *fetch_latest {
                    self.stripe_client
                        .get_object(
                            endpoint.tenant_id(),
                            object_type,
                            object_id,
                        )
                        .await?
                } else {
                    stripe_event.get_object().clone()
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
                    "Stripe object updated"
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
                        "Stripe object deleted"
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
        }

        Ok(stats)
    }
}

#[async_trait]
impl EventProcessor for StripeEventProcessor {
    fn provider(&self) -> Provider {
        Provider::Stripe
    }

    async fn process(
        &self,
        event: &WebhookEvent,
        endpoint: &WebhookEndpoint,
    ) -> errors::Result<ProcessingStats> {
        // Parse the Stripe event
        let stripe_event: StripeEvent = serde_json::from_value(
            event.payload().clone(),
        )
        .map_err(|e| {
            errors::Error::invalid(format!(
                "Failed to parse Stripe webhook event: {e}"
            ))
        })?;

        self.process_stripe_event(&stripe_event, endpoint).await
    }
}

/// Trait for Stripe API client.
#[async_trait]
pub trait StripeClient: Send + Sync + std::fmt::Debug {
    /// Get an object by type and ID.
    async fn get_object(
        &self,
        tenant_id: &value_object::TenantId,
        object_type: StripeObjectType,
        object_id: &str,
    ) -> errors::Result<serde_json::Value>;

    /// List all objects of a given type (for Initial Sync).
    async fn list_objects(
        &self,
        tenant_id: &value_object::TenantId,
        object_type: StripeObjectType,
        limit: Option<u32>,
    ) -> errors::Result<Vec<serde_json::Value>>;
}

/// Trait for handling Library data operations for Stripe.
#[async_trait]
pub trait StripeDataHandler: Send + Sync + std::fmt::Debug {
    /// Upsert a Stripe object in Library.
    ///
    /// Returns the Library data ID.
    async fn upsert_object(
        &self,
        endpoint: &WebhookEndpoint,
        object_type: StripeObjectType,
        object: &serde_json::Value,
        mapping: Option<&inbound_sync_domain::PropertyMapping>,
    ) -> errors::Result<String>;

    /// Delete a Stripe object from Library.
    async fn delete_object(
        &self,
        endpoint: &WebhookEndpoint,
        object_type: StripeObjectType,
        data_id: &str,
    ) -> errors::Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_external_id_format() {
        let object_type = StripeObjectType::Product;
        let object_id = "prod_12345";
        let external_id =
            format!("stripe:{}:{}", object_type.as_str(), object_id);
        assert_eq!(external_id, "stripe:product:prod_12345");
    }
}
