//! NoOp implementations of data handlers for testing and initial setup.
//!
//! These implementations do nothing but log operations. They can be replaced
//! with real implementations that write to the Library database.

use async_trait::async_trait;
use value_object::Ulid;

use inbound_sync_domain::{PropertyMapping, WebhookEndpoint};

use crate::providers::github::GitHubDataHandler;
use crate::providers::hubspot::{
    HubSpotDataHandler, HubSpotObject, ObjectType as HubSpotObjectType,
};
use crate::providers::linear::{Issue, LinearDataHandler, Project};
use crate::providers::notion::NotionDataHandler;
use crate::providers::square::{SquareDataHandler, SquareObjectType};
use crate::providers::stripe::{StripeDataHandler, StripeObjectType};

/// NoOp implementation of GitHubDataHandler.
///
/// Logs operations but does not write to the database.
#[derive(Debug, Clone, Default)]
pub struct NoOpGitHubDataHandler;

#[async_trait]
impl GitHubDataHandler for NoOpGitHubDataHandler {
    async fn upsert_data(
        &self,
        endpoint: &WebhookEndpoint,
        path: &str,
        content: &str,
        _mapping: Option<&PropertyMapping>,
    ) -> errors::Result<String> {
        let data_id = format!(
            "noop_github_{}",
            Ulid::new().to_string().to_lowercase()
        );

        tracing::info!(
            endpoint_id = %endpoint.id(),
            path = path,
            content_len = content.len(),
            data_id = %data_id,
            "NoOp: Would upsert GitHub file"
        );

        Ok(data_id)
    }

    async fn delete_data(
        &self,
        endpoint: &WebhookEndpoint,
        data_id: &str,
    ) -> errors::Result<()> {
        tracing::info!(
            endpoint_id = %endpoint.id(),
            data_id = data_id,
            "NoOp: Would delete GitHub file data"
        );

        Ok(())
    }
}

/// NoOp implementation of LinearDataHandler.
#[derive(Debug, Clone, Default)]
pub struct NoOpLinearDataHandler;

#[async_trait]
impl LinearDataHandler for NoOpLinearDataHandler {
    async fn upsert_issue(
        &self,
        endpoint: &WebhookEndpoint,
        issue: &Issue,
        _mapping: Option<&PropertyMapping>,
    ) -> errors::Result<String> {
        let data_id = format!(
            "noop_linear_issue_{}",
            Ulid::new().to_string().to_lowercase()
        );

        tracing::info!(
            endpoint_id = %endpoint.id(),
            issue_id = %issue.id,
            identifier = %issue.identifier,
            title = %issue.title,
            data_id = %data_id,
            "NoOp: Would upsert Linear issue"
        );

        Ok(data_id)
    }

    async fn delete_issue(
        &self,
        endpoint: &WebhookEndpoint,
        data_id: &str,
    ) -> errors::Result<()> {
        tracing::info!(
            endpoint_id = %endpoint.id(),
            data_id = data_id,
            "NoOp: Would delete Linear issue"
        );

        Ok(())
    }

    async fn upsert_project(
        &self,
        endpoint: &WebhookEndpoint,
        project: &Project,
        _mapping: Option<&PropertyMapping>,
    ) -> errors::Result<String> {
        let data_id = format!(
            "noop_linear_project_{}",
            Ulid::new().to_string().to_lowercase()
        );

        tracing::info!(
            endpoint_id = %endpoint.id(),
            project_id = %project.id,
            project_name = %project.name,
            data_id = %data_id,
            "NoOp: Would upsert Linear project"
        );

        Ok(data_id)
    }

    async fn delete_project(
        &self,
        endpoint: &WebhookEndpoint,
        data_id: &str,
    ) -> errors::Result<()> {
        tracing::info!(
            endpoint_id = %endpoint.id(),
            data_id = data_id,
            "NoOp: Would delete Linear project"
        );

        Ok(())
    }
}

/// NoOp implementation of HubSpotDataHandler.
#[derive(Debug, Clone, Default)]
pub struct NoOpHubSpotDataHandler;

#[async_trait]
impl HubSpotDataHandler for NoOpHubSpotDataHandler {
    async fn upsert_object(
        &self,
        endpoint: &WebhookEndpoint,
        object_type: HubSpotObjectType,
        object: &HubSpotObject,
        _mapping: Option<&PropertyMapping>,
    ) -> errors::Result<String> {
        let data_id = format!(
            "noop_hubspot_{}_{}",
            object_type.as_str(),
            Ulid::new().to_string().to_lowercase()
        );

        tracing::info!(
            endpoint_id = %endpoint.id(),
            object_type = object_type.as_str(),
            object_id = %object.id,
            data_id = %data_id,
            "NoOp: Would upsert HubSpot object"
        );

        Ok(data_id)
    }

    async fn delete_object(
        &self,
        endpoint: &WebhookEndpoint,
        object_type: HubSpotObjectType,
        data_id: &str,
    ) -> errors::Result<()> {
        tracing::info!(
            endpoint_id = %endpoint.id(),
            object_type = object_type.as_str(),
            data_id = data_id,
            "NoOp: Would delete HubSpot object"
        );

        Ok(())
    }
}

/// NoOp implementation of StripeDataHandler.
#[derive(Debug, Clone, Default)]
pub struct NoOpStripeDataHandler;

#[async_trait]
impl StripeDataHandler for NoOpStripeDataHandler {
    async fn upsert_object(
        &self,
        endpoint: &WebhookEndpoint,
        object_type: StripeObjectType,
        object_data: &serde_json::Value,
        _mapping: Option<&PropertyMapping>,
    ) -> errors::Result<String> {
        let object_id = object_data
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let data_id = format!(
            "noop_stripe_{}_{}",
            object_type.as_str(),
            Ulid::new().to_string().to_lowercase()
        );

        tracing::info!(
            endpoint_id = %endpoint.id(),
            object_type = object_type.as_str(),
            object_id = object_id,
            data_id = %data_id,
            "NoOp: Would upsert Stripe object"
        );

        Ok(data_id)
    }

    async fn delete_object(
        &self,
        endpoint: &WebhookEndpoint,
        object_type: StripeObjectType,
        data_id: &str,
    ) -> errors::Result<()> {
        tracing::info!(
            endpoint_id = %endpoint.id(),
            object_type = object_type.as_str(),
            data_id = data_id,
            "NoOp: Would delete Stripe object"
        );

        Ok(())
    }
}

/// NoOp implementation of NotionDataHandler.
#[derive(Debug, Clone, Default)]
pub struct NoOpNotionDataHandler;

#[async_trait]
impl NotionDataHandler for NoOpNotionDataHandler {
    async fn upsert_page(
        &self,
        endpoint: &WebhookEndpoint,
        page: &serde_json::Value,
        _mapping: Option<&PropertyMapping>,
    ) -> errors::Result<()> {
        let page_id =
            page.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");

        tracing::info!(
            endpoint_id = %endpoint.id(),
            page_id = page_id,
            "NoOp: Would upsert Notion page"
        );

        Ok(())
    }

    async fn delete_page(
        &self,
        endpoint: &WebhookEndpoint,
        page_id: &str,
    ) -> errors::Result<()> {
        tracing::info!(
            endpoint_id = %endpoint.id(),
            page_id = page_id,
            "NoOp: Would delete Notion page"
        );

        Ok(())
    }

    async fn upsert_database(
        &self,
        endpoint: &WebhookEndpoint,
        database: &serde_json::Value,
        _mapping: Option<&PropertyMapping>,
    ) -> errors::Result<()> {
        let db_id = database
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        tracing::info!(
            endpoint_id = %endpoint.id(),
            database_id = db_id,
            "NoOp: Would upsert Notion database"
        );

        Ok(())
    }

    async fn delete_database(
        &self,
        endpoint: &WebhookEndpoint,
        database_id: &str,
    ) -> errors::Result<()> {
        tracing::info!(
            endpoint_id = %endpoint.id(),
            database_id = database_id,
            "NoOp: Would delete Notion database"
        );

        Ok(())
    }
}

/// NoOp implementation of SquareDataHandler.
#[derive(Debug, Clone, Default)]
pub struct NoOpSquareDataHandler;

#[async_trait]
impl SquareDataHandler for NoOpSquareDataHandler {
    async fn upsert_object(
        &self,
        endpoint: &WebhookEndpoint,
        object_type: SquareObjectType,
        object_data: &serde_json::Value,
        _mapping: Option<&PropertyMapping>,
    ) -> errors::Result<String> {
        let object_id = object_data
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let data_id = format!(
            "noop_square_{}_{}",
            object_type.as_str(),
            Ulid::new().to_string().to_lowercase()
        );

        tracing::info!(
            endpoint_id = %endpoint.id(),
            object_type = object_type.as_str(),
            object_id = object_id,
            data_id = %data_id,
            "NoOp: Would upsert Square object"
        );

        Ok(data_id)
    }

    async fn delete_object(
        &self,
        endpoint: &WebhookEndpoint,
        object_type: SquareObjectType,
        data_id: &str,
    ) -> errors::Result<()> {
        tracing::info!(
            endpoint_id = %endpoint.id(),
            object_type = object_type.as_str(),
            data_id = data_id,
            "NoOp: Would delete Square object"
        );

        Ok(())
    }
}
