//! Use case for sending test webhooks.

use std::sync::Arc;

use inbound_sync_domain::{
    Provider, WebhookEndpointId, WebhookEndpointRepository, WebhookEvent,
    WebhookEventRepository,
};

/// Use case for sending test webhooks.
pub struct SendTestWebhook {
    endpoint_repo: Arc<dyn WebhookEndpointRepository>,
    event_repo: Arc<dyn WebhookEventRepository>,
}

impl SendTestWebhook {
    /// Create a new SendTestWebhook use case.
    pub fn new(
        endpoint_repo: Arc<dyn WebhookEndpointRepository>,
        event_repo: Arc<dyn WebhookEventRepository>,
    ) -> Self {
        Self {
            endpoint_repo,
            event_repo,
        }
    }

    /// Send a test webhook to an endpoint.
    pub async fn execute(
        &self,
        endpoint_id: &WebhookEndpointId,
        event_type: &str,
    ) -> errors::Result<WebhookEvent> {
        // Get the endpoint
        let endpoint = self
            .endpoint_repo
            .find_by_id(endpoint_id)
            .await?
            .ok_or_else(|| errors::Error::not_found("Webhook endpoint"))?;

        // Create test payload based on provider
        let payload = create_test_payload(*endpoint.provider(), event_type);

        // Create a new event
        let event = WebhookEvent::create(
            endpoint_id.clone(),
            *endpoint.provider(),
            event_type.to_string(),
            payload,
            None, // No headers for test events
            true, // signature is valid for test events
        );

        // Save the event
        self.event_repo.save(&event).await?;

        tracing::info!(
            endpoint_id = %endpoint_id,
            event_id = %event.id(),
            event_type = %event_type,
            "Test webhook event created"
        );

        Ok(event)
    }
}

/// Create a test payload for a given provider and event type.
fn create_test_payload(
    provider: Provider,
    event_type: &str,
) -> serde_json::Value {
    match provider {
        Provider::Github => create_github_test_payload(event_type),
        Provider::Linear => create_linear_test_payload(event_type),
        Provider::Hubspot => create_hubspot_test_payload(event_type),
        Provider::Stripe => create_stripe_test_payload(event_type),
        Provider::Square => create_square_test_payload(event_type),
        Provider::Notion => create_notion_test_payload(event_type),
        Provider::Airtable => create_airtable_test_payload(event_type),
        Provider::Generic => create_generic_test_payload(event_type),
    }
}

fn create_github_test_payload(event_type: &str) -> serde_json::Value {
    serde_json::json!({
        "action": event_type.split('.').next_back().unwrap_or("test"),
        "ref": "refs/heads/main",
        "repository": {
            "id": 123456789,
            "name": "test-repo",
            "full_name": "test-org/test-repo",
            "owner": {
                "login": "test-org"
            }
        },
        "sender": {
            "login": "test-user",
            "id": 1
        },
        "_test": true,
        "_event_type": event_type
    })
}

fn create_linear_test_payload(event_type: &str) -> serde_json::Value {
    let mut parts = event_type.split('.');
    let event_name = parts.next().unwrap_or("Issue");
    let action = parts.next().unwrap_or("create");
    serde_json::json!({
        "action": action,
        "type": event_name,
        "data": {
            "id": "test-issue-id",
            "title": "Test Issue",
            "state": {
                "name": "Todo"
            },
            "team": {
                "id": "test-team-id",
                "name": "Test Team"
            }
        },
        "_test": true,
        "_event_type": event_type
    })
}

fn create_hubspot_test_payload(event_type: &str) -> serde_json::Value {
    serde_json::json!([{
        "eventId": 1,
        "subscriptionId": 12345,
        "subscriptionType": event_type,
        "objectId": 123456,
        "propertyName": "email",
        "propertyValue": "test@example.com",
        "_test": true
    }])
}

fn create_stripe_test_payload(event_type: &str) -> serde_json::Value {
    serde_json::json!({
        "id": "evt_test_123",
        "type": event_type,
        "data": {
            "object": {
                "id": "prod_test_123",
                "name": "Test Product",
                "active": true
            }
        },
        "_test": true
    })
}

fn create_notion_test_payload(event_type: &str) -> serde_json::Value {
    serde_json::json!({
        "type": event_type,
        "workspace_id": "test-workspace-id",
        "data": {
            "object": {
                "object": event_type.split('.').next().unwrap_or("page"),
                "id": "test-page-id",
                "created_time": "2024-01-01T00:00:00.000Z",
                "last_edited_time": "2024-01-01T00:00:00.000Z",
                "properties": {}
            }
        },
        "_test": true
    })
}

fn create_airtable_test_payload(event_type: &str) -> serde_json::Value {
    serde_json::json!({
        "base": {
            "id": "appTestBase123"
        },
        "webhook": {
            "id": "achTestWebhook"
        },
        "timestamp": "2024-01-01T00:00:00.000Z",
        "actionMetadata": {
            "source": "client",
            "sourceMetadata": {
                "user": {
                    "id": "usrTest123",
                    "email": "test@example.com"
                }
            }
        },
        "payloads": [{
            "tableId": "tblTestTable",
            "recordId": "recTestRecord",
            "changedFieldsById": {
                "fldName": "Test Value"
            }
        }],
        "_test": true,
        "_event_type": event_type
    })
}

fn create_generic_test_payload(event_type: &str) -> serde_json::Value {
    serde_json::json!({
        "event": event_type,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "data": {
            "message": "This is a test webhook payload"
        },
        "_test": true
    })
}

fn create_square_test_payload(event_type: &str) -> serde_json::Value {
    serde_json::json!({
        "merchant_id": "MERCHANT_TEST_123",
        "type": event_type,
        "event_id": "evt_test_123456789",
        "created_at": chrono::Utc::now().to_rfc3339(),
        "data": {
            "type": event_type.split('.').next().unwrap_or("object"),
            "id": "TEST_OBJECT_ID",
            "object": {
                "id": "TEST_OBJECT_ID",
                "type": event_type.split('.').next().unwrap_or("ITEM").to_uppercase(),
                "updated_at": chrono::Utc::now().to_rfc3339(),
                "is_deleted": false
            }
        },
        "_test": true
    })
}
