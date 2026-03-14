//! NoOp implementations of provider clients for testing and initial setup.
//!
//! These implementations do nothing but log operations. They can be replaced
//! with real implementations that make actual API calls.

use async_trait::async_trait;
use inbound_sync_domain::{
    ApiKeyValidationResult, ApiKeyValidator, OAuthProvider,
};
use value_object::TenantId;

use crate::providers::github::{
    GitHubClient, PullRequestFile, RepositoryContent,
};
use crate::providers::hubspot::{
    HubSpotClient, HubSpotObject, ObjectType as HubSpotObjectType,
};
use crate::providers::linear::{Issue, LinearClient, Project, Team};
use crate::providers::notion::{NotionClient, NotionQueryResult};
use crate::providers::square::{SquareClient, SquareObjectType};
use crate::providers::stripe::{StripeClient, StripeObjectType};

/// NoOp implementation of GitHubClient.
///
/// Logs operations but does not make actual API calls.
#[derive(Debug, Clone, Default)]
pub struct NoOpGitHubClient;

#[async_trait]
impl GitHubClient for NoOpGitHubClient {
    async fn get_file_content(
        &self,
        _tenant_id: &TenantId,
        repo: &str,
        path: &str,
        branch: &str,
    ) -> errors::Result<String> {
        tracing::info!(
            repo = repo,
            path = path,
            branch = branch,
            "NoOp: Would fetch file content from GitHub"
        );

        Ok(String::new())
    }

    async fn get_pr_files(
        &self,
        _tenant_id: &TenantId,
        repo: &str,
        pr_number: u64,
    ) -> errors::Result<Vec<PullRequestFile>> {
        tracing::info!(
            repo = repo,
            pr_number = pr_number,
            "NoOp: Would fetch PR files from GitHub"
        );

        Ok(vec![])
    }

    async fn list_repository_contents(
        &self,
        _tenant_id: &TenantId,
        repo: &str,
        branch: &str,
        path_pattern: Option<&str>,
    ) -> errors::Result<Vec<RepositoryContent>> {
        tracing::info!(
            repo = repo,
            branch = branch,
            path_pattern = ?path_pattern,
            "NoOp: Would list repository contents from GitHub"
        );

        Ok(vec![])
    }
}

/// NoOp implementation of LinearClient.
#[derive(Debug, Clone, Default)]
pub struct NoOpLinearClient;

#[async_trait]
impl LinearClient for NoOpLinearClient {
    async fn get_issue(
        &self,
        _tenant_id: &TenantId,
        issue_id: &str,
    ) -> errors::Result<Issue> {
        tracing::info!(
            issue_id = issue_id,
            "NoOp: Would fetch issue from Linear"
        );

        Ok(Issue {
            id: issue_id.to_string(),
            identifier: "NOOP-1".to_string(),
            title: "NoOp Issue".to_string(),
            description: None,
            priority: None,
            state: None,
            assignee: None,
            creator: None,
            team: None,
            project: None,
            cycle: None,
            labels: vec![],
            due_date: None,
            estimate: None,
            created_at: String::new(),
            updated_at: String::new(),
            completed_at: None,
            canceled_at: None,
            url: None,
        })
    }

    async fn get_project(
        &self,
        _tenant_id: &TenantId,
        project_id: &str,
    ) -> errors::Result<Project> {
        tracing::info!(
            project_id = project_id,
            "NoOp: Would fetch project from Linear"
        );

        Ok(Project {
            id: project_id.to_string(),
            name: "NoOp Project".to_string(),
            description: None,
            state: None,
            icon: None,
            color: None,
            lead: None,
            target_date: None,
            start_date: None,
            created_at: String::new(),
            updated_at: String::new(),
            url: None,
        })
    }

    async fn list_issues(
        &self,
        _tenant_id: &TenantId,
        team_id: Option<&str>,
        project_id: Option<&str>,
    ) -> errors::Result<Vec<Issue>> {
        tracing::info!(
            team_id = ?team_id,
            project_id = ?project_id,
            "NoOp: Would list issues from Linear"
        );
        Ok(vec![])
    }

    async fn list_projects(
        &self,
        _tenant_id: &TenantId,
        team_id: Option<&str>,
    ) -> errors::Result<Vec<Project>> {
        tracing::info!(
            team_id = ?team_id,
            "NoOp: Would list projects from Linear"
        );
        Ok(vec![])
    }

    async fn list_teams(
        &self,
        _tenant_id: &TenantId,
    ) -> errors::Result<Vec<Team>> {
        tracing::info!("NoOp: Would list teams from Linear");
        Ok(vec![])
    }
}

/// NoOp implementation of HubSpotClient.
#[derive(Debug, Clone, Default)]
pub struct NoOpHubSpotClient;

#[async_trait]
impl HubSpotClient for NoOpHubSpotClient {
    async fn get_object(
        &self,
        _tenant_id: &TenantId,
        object_type: HubSpotObjectType,
        object_id: i64,
    ) -> errors::Result<HubSpotObject> {
        tracing::info!(
            object_type = object_type.as_str(),
            object_id = object_id,
            "NoOp: Would fetch object from HubSpot"
        );

        Ok(HubSpotObject {
            id: object_id.to_string(),
            properties: serde_json::json!({}),
            created_at: String::new(),
            updated_at: String::new(),
            archived: false,
        })
    }
}

/// NoOp implementation of StripeClient.
#[derive(Debug, Clone, Default)]
pub struct NoOpStripeClient;

#[async_trait]
impl StripeClient for NoOpStripeClient {
    async fn get_object(
        &self,
        _tenant_id: &TenantId,
        object_type: StripeObjectType,
        object_id: &str,
    ) -> errors::Result<serde_json::Value> {
        tracing::info!(
            object_type = object_type.as_str(),
            object_id = object_id,
            "NoOp: Would fetch object from Stripe"
        );

        Ok(serde_json::json!({
            "id": object_id,
            "object": object_type.as_str()
        }))
    }

    async fn list_objects(
        &self,
        _tenant_id: &TenantId,
        object_type: StripeObjectType,
        limit: Option<u32>,
    ) -> errors::Result<Vec<serde_json::Value>> {
        tracing::info!(
            object_type = object_type.as_str(),
            limit = ?limit,
            "NoOp: Would list objects from Stripe"
        );
        Ok(vec![])
    }
}

/// NoOp implementation of NotionClient.
#[derive(Debug, Clone, Default)]
pub struct NoOpNotionClient;

#[async_trait]
impl NotionClient for NoOpNotionClient {
    async fn get_page(
        &self,
        _tenant_id: &TenantId,
        page_id: &str,
    ) -> errors::Result<serde_json::Value> {
        tracing::info!(
            page_id = page_id,
            "NoOp: Would fetch page from Notion"
        );

        Ok(serde_json::json!({
            "object": "page",
            "id": page_id,
            "properties": {}
        }))
    }

    async fn get_database(
        &self,
        _tenant_id: &TenantId,
        database_id: &str,
    ) -> errors::Result<serde_json::Value> {
        tracing::info!(
            database_id = database_id,
            "NoOp: Would fetch database from Notion"
        );

        Ok(serde_json::json!({
            "object": "database",
            "id": database_id,
            "properties": {}
        }))
    }

    async fn query_database(
        &self,
        _tenant_id: &TenantId,
        database_id: &str,
        _filter: Option<serde_json::Value>,
        _sorts: Option<Vec<serde_json::Value>>,
        _start_cursor: Option<String>,
        _page_size: Option<u32>,
    ) -> errors::Result<NotionQueryResult> {
        tracing::info!(
            database_id = database_id,
            "NoOp: Would query database from Notion"
        );

        Ok(NotionQueryResult {
            results: vec![],
            has_more: false,
            next_cursor: None,
        })
    }

    async fn get_page_content(
        &self,
        _tenant_id: &TenantId,
        page_id: &str,
    ) -> errors::Result<Vec<serde_json::Value>> {
        tracing::info!(
            page_id = page_id,
            "NoOp: Would fetch page content from Notion"
        );

        Ok(vec![])
    }

    async fn get_page_property(
        &self,
        _tenant_id: &TenantId,
        page_id: &str,
        property_id: &str,
    ) -> errors::Result<serde_json::Value> {
        tracing::info!(
            page_id = page_id,
            property_id = property_id,
            "NoOp: Would fetch page property from Notion"
        );

        Ok(serde_json::json!({}))
    }

    async fn list_database_pages(
        &self,
        _tenant_id: &TenantId,
        database_id: &str,
    ) -> errors::Result<Vec<serde_json::Value>> {
        tracing::info!(
            database_id = database_id,
            "NoOp: Would list database pages from Notion"
        );
        Ok(vec![])
    }
}

/// NoOp implementation of SquareClient.
#[derive(Debug, Clone, Default)]
pub struct NoOpSquareClient;

#[async_trait]
impl SquareClient for NoOpSquareClient {
    async fn get_object(
        &self,
        _tenant_id: &TenantId,
        object_type: SquareObjectType,
        object_id: &str,
    ) -> errors::Result<serde_json::Value> {
        tracing::info!(
            object_type = object_type.as_str(),
            object_id = object_id,
            "NoOp: Would fetch object from Square"
        );

        Ok(serde_json::json!({
            "id": object_id,
            "type": object_type.as_str().to_uppercase()
        }))
    }

    async fn batch_retrieve_catalog_objects(
        &self,
        _tenant_id: &TenantId,
        object_ids: &[String],
    ) -> errors::Result<Vec<serde_json::Value>> {
        tracing::info!(
            object_count = object_ids.len(),
            "NoOp: Would batch retrieve catalog objects from Square"
        );

        Ok(object_ids
            .iter()
            .map(|id| {
                serde_json::json!({
                    "id": id,
                    "type": "ITEM"
                })
            })
            .collect())
    }

    async fn list_catalog_items(
        &self,
        _tenant_id: &TenantId,
        _cursor: Option<&str>,
    ) -> errors::Result<(Vec<serde_json::Value>, Option<String>)> {
        tracing::info!("NoOp: Would list catalog items from Square");
        Ok((vec![], None))
    }

    async fn list_customers(
        &self,
        _tenant_id: &TenantId,
        _cursor: Option<&str>,
    ) -> errors::Result<(Vec<serde_json::Value>, Option<String>)> {
        tracing::info!("NoOp: Would list customers from Square");
        Ok((vec![], None))
    }

    async fn list_orders(
        &self,
        _tenant_id: &TenantId,
        _location_ids: &[String],
        _cursor: Option<&str>,
    ) -> errors::Result<(Vec<serde_json::Value>, Option<String>)> {
        tracing::info!("NoOp: Would list orders from Square");
        Ok((vec![], None))
    }
}

/// NoOp implementation of ApiKeyValidator.
///
/// Always returns success for testing and initial setup.
#[derive(Debug, Clone, Default)]
pub struct NoOpApiKeyValidator;

#[async_trait]
impl ApiKeyValidator for NoOpApiKeyValidator {
    async fn validate(
        &self,
        provider: OAuthProvider,
        api_key: &str,
    ) -> errors::Result<ApiKeyValidationResult> {
        tracing::info!(
            provider = ?provider,
            api_key_len = api_key.len(),
            "NoOp: Would validate API key"
        );

        // Return success with a mock account for testing
        Ok(ApiKeyValidationResult::success(
            format!(
                "noop_acct_{}",
                api_key.chars().take(8).collect::<String>()
            ),
            Some("NoOp Account".to_string()),
        ))
    }

    fn uses_api_key(&self, provider: OAuthProvider) -> bool {
        // Stripe and Generic providers typically use API keys
        matches!(provider, OAuthProvider::Stripe | OAuthProvider::Generic)
    }
}
