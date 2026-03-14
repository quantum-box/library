//! Notion event processor implementation.

use async_trait::async_trait;
use std::fmt::Debug;
use std::sync::Arc;

use inbound_sync_domain::{
    ProcessingStats, PropertyMapping, WebhookEndpoint,
};

use super::client::NotionClient;
use super::payload::{NotionAction, NotionObjectType, NotionWebhookEvent};
use crate::usecase::EventProcessor;

/// Trait for handling Notion data updates in Library.
#[async_trait]
pub trait NotionDataHandler: Send + Sync + Debug {
    /// Create or update a page in Library.
    async fn upsert_page(
        &self,
        endpoint: &WebhookEndpoint,
        page: &serde_json::Value,
        mapping: Option<&PropertyMapping>,
    ) -> errors::Result<()>;

    /// Delete a page from Library.
    async fn delete_page(
        &self,
        endpoint: &WebhookEndpoint,
        page_id: &str,
    ) -> errors::Result<()>;

    /// Create or update a database in Library.
    async fn upsert_database(
        &self,
        endpoint: &WebhookEndpoint,
        database: &serde_json::Value,
        mapping: Option<&PropertyMapping>,
    ) -> errors::Result<()>;

    /// Delete a database from Library.
    async fn delete_database(
        &self,
        endpoint: &WebhookEndpoint,
        database_id: &str,
    ) -> errors::Result<()>;
}

/// Notion event processor.
#[derive(Debug)]
pub struct NotionEventProcessor {
    notion_client: Arc<dyn NotionClient>,
    data_handler: Arc<dyn NotionDataHandler>,
}

impl NotionEventProcessor {
    /// Create a new Notion event processor.
    pub fn new(
        notion_client: Arc<dyn NotionClient>,
        data_handler: Arc<dyn NotionDataHandler>,
    ) -> Self {
        Self {
            notion_client,
            data_handler,
        }
    }

    /// Process a page event.
    async fn process_page_event(
        &self,
        endpoint: &WebhookEndpoint,
        event: &NotionWebhookEvent,
        mapping: Option<&PropertyMapping>,
    ) -> errors::Result<ProcessingStats> {
        let mut stats = ProcessingStats::default();
        let page_id = &event.data.object.id;

        match event.action() {
            NotionAction::Created => {
                // Fetch latest page data from API
                let page = self
                    .notion_client
                    .get_page(endpoint.tenant_id(), page_id)
                    .await?;
                self.data_handler
                    .upsert_page(endpoint, &page, mapping)
                    .await?;
                stats.created = 1;
            }
            NotionAction::Updated | NotionAction::Unarchived => {
                // Fetch latest page data from API
                let page = self
                    .notion_client
                    .get_page(endpoint.tenant_id(), page_id)
                    .await?;
                self.data_handler
                    .upsert_page(endpoint, &page, mapping)
                    .await?;
                stats.updated = 1;
            }
            NotionAction::Deleted | NotionAction::Archived => {
                self.data_handler.delete_page(endpoint, page_id).await?;
                stats.deleted = 1;
            }
            NotionAction::Unknown => {
                tracing::warn!(
                    event_type = %event.event_type,
                    "Unknown Notion page action"
                );
                stats.skipped = 1;
            }
        }

        Ok(stats)
    }

    /// Process a database event.
    async fn process_database_event(
        &self,
        endpoint: &WebhookEndpoint,
        event: &NotionWebhookEvent,
        mapping: Option<&PropertyMapping>,
    ) -> errors::Result<ProcessingStats> {
        let mut stats = ProcessingStats::default();
        let database_id = &event.data.object.id;

        match event.action() {
            NotionAction::Created => {
                let database = self
                    .notion_client
                    .get_database(endpoint.tenant_id(), database_id)
                    .await?;
                self.data_handler
                    .upsert_database(endpoint, &database, mapping)
                    .await?;
                stats.created = 1;
            }
            NotionAction::Updated | NotionAction::Unarchived => {
                let database = self
                    .notion_client
                    .get_database(endpoint.tenant_id(), database_id)
                    .await?;
                self.data_handler
                    .upsert_database(endpoint, &database, mapping)
                    .await?;
                stats.updated = 1;
            }
            NotionAction::Deleted | NotionAction::Archived => {
                self.data_handler
                    .delete_database(endpoint, database_id)
                    .await?;
                stats.deleted = 1;
            }
            NotionAction::Unknown => {
                tracing::warn!(
                    event_type = %event.event_type,
                    "Unknown Notion database action"
                );
                stats.skipped = 1;
            }
        }

        Ok(stats)
    }
}

#[async_trait]
impl EventProcessor for NotionEventProcessor {
    fn provider(&self) -> inbound_sync_domain::Provider {
        inbound_sync_domain::Provider::Notion
    }

    async fn process(
        &self,
        event: &inbound_sync_domain::WebhookEvent,
        endpoint: &WebhookEndpoint,
    ) -> errors::Result<ProcessingStats> {
        let payload = event.payload();
        let notion_event: NotionWebhookEvent =
            serde_json::from_value(payload.clone()).map_err(|e| {
                errors::Error::invalid(format!(
                    "Failed to parse Notion webhook event: {e}"
                ))
            })?;

        let mapping = endpoint.mapping();

        tracing::info!(
            event_type = %notion_event.event_type,
            object_id = %notion_event.data.object.id,
            "Processing Notion webhook event"
        );

        match notion_event.object_type() {
            NotionObjectType::Page => {
                self.process_page_event(endpoint, &notion_event, mapping)
                    .await
            }
            NotionObjectType::Database => {
                self.process_database_event(
                    endpoint,
                    &notion_event,
                    mapping,
                )
                .await
            }
            NotionObjectType::Block | NotionObjectType::Comment => {
                // Block and comment events are not fully supported yet
                tracing::debug!(
                    object_type = ?notion_event.object_type(),
                    "Skipping unsupported Notion object type"
                );
                Ok(ProcessingStats {
                    skipped: 1,
                    ..Default::default()
                })
            }
            NotionObjectType::Unknown => {
                tracing::warn!(
                    event_type = %notion_event.event_type,
                    "Unknown Notion object type"
                );
                Ok(ProcessingStats {
                    skipped: 1,
                    ..Default::default()
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct MockNotionClient;

    #[async_trait]
    impl NotionClient for MockNotionClient {
        async fn get_page(
            &self,
            _tenant_id: &value_object::TenantId,
            _page_id: &str,
        ) -> errors::Result<serde_json::Value> {
            Ok(serde_json::json!({
                "object": "page",
                "id": "test-page-id",
                "properties": {}
            }))
        }

        async fn get_database(
            &self,
            _tenant_id: &value_object::TenantId,
            _database_id: &str,
        ) -> errors::Result<serde_json::Value> {
            Ok(serde_json::json!({
                "object": "database",
                "id": "test-db-id",
                "title": []
            }))
        }

        async fn query_database(
            &self,
            _tenant_id: &value_object::TenantId,
            _database_id: &str,
            _filter: Option<serde_json::Value>,
            _sorts: Option<Vec<serde_json::Value>>,
            _start_cursor: Option<String>,
            _page_size: Option<u32>,
        ) -> errors::Result<super::super::client::NotionQueryResult>
        {
            Ok(super::super::client::NotionQueryResult {
                results: vec![],
                has_more: false,
                next_cursor: None,
            })
        }

        async fn get_page_content(
            &self,
            _tenant_id: &value_object::TenantId,
            _page_id: &str,
        ) -> errors::Result<Vec<serde_json::Value>> {
            Ok(vec![])
        }

        async fn get_page_property(
            &self,
            _tenant_id: &value_object::TenantId,
            _page_id: &str,
            _property_id: &str,
        ) -> errors::Result<serde_json::Value> {
            Ok(serde_json::json!({}))
        }

        async fn list_database_pages(
            &self,
            _tenant_id: &value_object::TenantId,
            _database_id: &str,
        ) -> errors::Result<Vec<serde_json::Value>> {
            Ok(vec![])
        }
    }

    #[derive(Debug)]
    struct MockDataHandler;

    #[async_trait]
    impl NotionDataHandler for MockDataHandler {
        async fn upsert_page(
            &self,
            _endpoint: &WebhookEndpoint,
            _page: &serde_json::Value,
            _mapping: Option<&PropertyMapping>,
        ) -> errors::Result<()> {
            Ok(())
        }

        async fn delete_page(
            &self,
            _endpoint: &WebhookEndpoint,
            _page_id: &str,
        ) -> errors::Result<()> {
            Ok(())
        }

        async fn upsert_database(
            &self,
            _endpoint: &WebhookEndpoint,
            _database: &serde_json::Value,
            _mapping: Option<&PropertyMapping>,
        ) -> errors::Result<()> {
            Ok(())
        }

        async fn delete_database(
            &self,
            _endpoint: &WebhookEndpoint,
            _database_id: &str,
        ) -> errors::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_processor_creation() {
        let client = Arc::new(MockNotionClient);
        let handler = Arc::new(MockDataHandler);
        let _processor = NotionEventProcessor::new(client, handler);
    }
}
