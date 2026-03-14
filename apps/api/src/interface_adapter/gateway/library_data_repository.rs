use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use persistence::Db;
use serde_json::Value as JsonValue;
use sqlx::query_scalar;
use value_object::{TenantId, Ulid};

use crate::domain::{RepoId, RepoRepository};
use database_manager::domain::{DataId, DatabaseId, PropertyType};
use database_manager::usecase::FindAllPropertiesInputData;
use database_manager::{
    AddDataInputData, AddPropertyInputData, DeleteDataInputData,
    PropertyDataInputData, UpdateDataInputData,
};
use inbound_sync::providers::github::LibraryDataRepository;
use inbound_sync::sdk::{OperatorMultiTenancy, SystemExecutor};
use inbound_sync::{SyncStateRepository, WebhookEndpoint};

#[derive(Debug, Clone)]
pub struct LibraryDataRepositoryImpl {
    database_app: Arc<database_manager::App>,
    repo_repository: Arc<dyn RepoRepository>,
    sync_state_repo: Arc<dyn SyncStateRepository>,
    database_manager_db: Arc<Db>,
}

impl LibraryDataRepositoryImpl {
    pub fn new(
        database_app: Arc<database_manager::App>,
        repo_repository: Arc<dyn RepoRepository>,
        sync_state_repo: Arc<dyn SyncStateRepository>,
        database_manager_db: Arc<Db>,
    ) -> Self {
        Self {
            database_app,
            repo_repository,
            sync_state_repo,
            database_manager_db,
        }
    }

    fn build_context(
        tenant_id: &TenantId,
    ) -> (SystemExecutor, OperatorMultiTenancy) {
        let executor = SystemExecutor;
        let multi_tenancy = OperatorMultiTenancy::new(tenant_id.clone());
        (executor, multi_tenancy)
    }

    /// Resolve the database ID and the owning organization's tenant ID
    /// from the webhook endpoint's linked repository.
    ///
    /// The returned `TenantId` is the **organization's** tenant ID
    /// (stored in `library.repos.org_id`), which must be used for all
    /// `database_manager` operations because `database_manager.objects`
    /// stores data under the org's tenant, not the operator's.
    async fn resolve_repo_context(
        &self,
        endpoint: &WebhookEndpoint,
    ) -> errors::Result<(DatabaseId, TenantId)> {
        let repo_id = endpoint.repository_id().ok_or_else(|| {
            errors::Error::bad_request(
                "Webhook endpoint is not linked to a repository",
            )
        })?;
        let repo_id: RepoId = repo_id.parse()?;
        let repo = self
            .repo_repository
            .get_by_id(endpoint.tenant_id(), &repo_id)
            .await?
            .ok_or_else(|| errors::Error::not_found("repository"))?;
        let database_id = repo
            .databases()
            .first()
            .cloned()
            .ok_or_else(|| errors::Error::not_found("database"))?;
        let org_tenant_id: TenantId = repo.organization_id().clone();
        Ok((database_id, org_tenant_id))
    }

    async fn ensure_property_ids(
        &self,
        executor: &SystemExecutor,
        multi_tenancy: &OperatorMultiTenancy,
        tenant_id: &TenantId,
        database_id: &DatabaseId,
        properties: &HashMap<String, JsonValue>,
    ) -> errors::Result<HashMap<String, String>> {
        let existing = self
            .database_app
            .find_all_properties()
            .execute(FindAllPropertiesInputData {
                tenant_id: tenant_id.clone(),
                database_id: database_id.clone(),
            })
            .await?;
        let mut property_ids: HashMap<String, String> = existing
            .into_iter()
            .map(|property| {
                (property.name().to_string(), property.id().to_string())
            })
            .collect();

        for name in properties.keys() {
            if property_ids.contains_key(name) {
                continue;
            }
            let property = self
                .database_app
                .add_property()
                .execute(AddPropertyInputData {
                    executor,
                    multi_tenancy,
                    tenant_id,
                    database_id,
                    name,
                    property_type: Self::property_type_for(name),
                })
                .await?;
            property_ids.insert(name.clone(), property.id().to_string());
        }

        Ok(property_ids)
    }

    fn property_type_for(name: &str) -> PropertyType {
        if name == "content" {
            PropertyType::Markdown
        } else {
            PropertyType::String
        }
    }

    fn parse_linear_issue_id(external_id: &str) -> Option<&str> {
        let mut parts = external_id.splitn(3, ':');
        match (parts.next(), parts.next(), parts.next()) {
            (Some("linear"), Some("issue"), Some(issue_id)) => {
                Some(issue_id)
            }
            _ => None,
        }
    }

    async fn find_data_id_by_ext_linear(
        &self,
        endpoint: &WebhookEndpoint,
        issue_id: &str,
    ) -> errors::Result<Option<String>> {
        let (database_id, org_tenant_id) =
            self.resolve_repo_context(endpoint).await?;
        let properties = self
            .database_app
            .find_all_properties()
            .execute(FindAllPropertiesInputData {
                tenant_id: org_tenant_id.clone(),
                database_id: database_id.clone(),
            })
            .await?;

        let ext_linear = properties
            .iter()
            .find(|property| property.name() == "ext_linear");
        let ext_linear = match ext_linear {
            Some(property) => property,
            None => return Ok(None),
        };

        let field_num = ext_linear.property_num();
        if *field_num > 50 {
            return Ok(None);
        }
        let column = format!("value{field_num}");
        let query = format!(
            "SELECT id FROM tachyon_apps_database_manager.data \
            WHERE tenant_id = ? AND object_id = ? \
            AND JSON_UNQUOTE(JSON_EXTRACT(CAST(`{column}` AS JSON), '$.issue_id')) = ? \
            ORDER BY updated_at DESC \
            LIMIT 1"
        );

        let data_id: Option<String> = query_scalar(&query)
            .bind(org_tenant_id.to_string())
            .bind(database_id.to_string())
            .bind(issue_id)
            .fetch_optional(self.database_manager_db.pool().as_ref())
            .await
            .map_err(|e| {
                errors::Error::internal_server_error(e.to_string())
            })?;

        Ok(data_id)
    }

    fn json_value_to_string(value: &JsonValue) -> String {
        match value {
            JsonValue::String(value) => value.clone(),
            JsonValue::Number(value) => value.to_string(),
            JsonValue::Bool(value) => value.to_string(),
            JsonValue::Null => String::new(),
            JsonValue::Array(_) | JsonValue::Object(_) => {
                serde_json::to_string(value).unwrap_or_default()
            }
        }
    }

    fn build_property_data(
        properties: HashMap<String, JsonValue>,
        property_ids: &HashMap<String, String>,
    ) -> errors::Result<Vec<PropertyDataInputData>> {
        let mut property_data = Vec::new();
        for (name, value) in properties {
            let property_id = property_ids.get(&name).ok_or_else(|| {
                errors::Error::internal_server_error(format!(
                    "Property not found: {name}"
                ))
            })?;
            property_data.push(PropertyDataInputData {
                property_id: property_id.clone(),
                value: Self::json_value_to_string(&value),
            });
        }
        Ok(property_data)
    }

    fn enrich_properties(
        mut properties: HashMap<String, JsonValue>,
        content: &str,
        include_id: bool,
    ) -> HashMap<String, JsonValue> {
        if !properties.contains_key("content") {
            properties.insert(
                "content".to_string(),
                JsonValue::String(content.to_string()),
            );
        }
        if include_id && !properties.contains_key("id") {
            properties.insert(
                "id".to_string(),
                JsonValue::String(Ulid::new().to_string()),
            );
        }
        properties
    }
}

#[async_trait]
impl LibraryDataRepository for LibraryDataRepositoryImpl {
    async fn create_data(
        &self,
        endpoint: &WebhookEndpoint,
        name: &str,
        content: &str,
        properties: HashMap<String, JsonValue>,
    ) -> errors::Result<String> {
        let (database_id, org_tenant_id) =
            self.resolve_repo_context(endpoint).await?;
        let properties = Self::enrich_properties(properties, content, true);
        let (executor, multi_tenancy) = Self::build_context(&org_tenant_id);
        let property_ids = self
            .ensure_property_ids(
                &executor,
                &multi_tenancy,
                &org_tenant_id,
                &database_id,
                &properties,
            )
            .await?;
        let property_data =
            Self::build_property_data(properties, &property_ids)?;
        let input = AddDataInputData {
            executor: &executor,
            multi_tenancy: &multi_tenancy,
            tenant_id: &org_tenant_id,
            database_id: &database_id,
            name,
            property_data,
        };
        let data =
            self.database_app.add_data_usecase().execute(input).await?;
        Ok(data.id().to_string())
    }

    async fn update_data(
        &self,
        endpoint: &WebhookEndpoint,
        data_id: &str,
        name: &str,
        content: &str,
        properties: HashMap<String, JsonValue>,
    ) -> errors::Result<()> {
        let (database_id, org_tenant_id) =
            self.resolve_repo_context(endpoint).await?;
        let properties =
            Self::enrich_properties(properties, content, false);
        let (executor, multi_tenancy) = Self::build_context(&org_tenant_id);
        let property_ids = self
            .ensure_property_ids(
                &executor,
                &multi_tenancy,
                &org_tenant_id,
                &database_id,
                &properties,
            )
            .await?;
        let property_data =
            Self::build_property_data(properties, &property_ids)?;
        let data_id: DataId = data_id.parse()?;
        let input = UpdateDataInputData {
            executor: &executor,
            multi_tenancy: &multi_tenancy,
            tenant_id: &org_tenant_id,
            database_id: &database_id,
            data_id: &data_id,
            name,
            data: property_data,
        };
        self.database_app
            .update_data_usecase()
            .execute(input)
            .await?;
        Ok(())
    }

    async fn delete_data(
        &self,
        endpoint: &WebhookEndpoint,
        data_id: &str,
    ) -> errors::Result<()> {
        let (database_id, org_tenant_id) =
            self.resolve_repo_context(endpoint).await?;
        let (executor, multi_tenancy) = Self::build_context(&org_tenant_id);
        let tenant_id = org_tenant_id.to_string();
        let database_id = database_id.to_string();
        let input = DeleteDataInputData {
            executor: &executor,
            multi_tenancy: &multi_tenancy,
            tenant_id: tenant_id.as_str(),
            database_id: database_id.as_str(),
            data_id,
        };
        self.database_app
            .delete_data_usecase()
            .execute(&input)
            .await?;
        Ok(())
    }

    async fn find_by_external_id(
        &self,
        endpoint: &WebhookEndpoint,
        external_id: &str,
    ) -> errors::Result<Option<String>> {
        let state = self
            .sync_state_repo
            .find_by_external_id(endpoint.id(), external_id)
            .await?;
        if let Some(state) = state {
            return Ok(Some(state.data_id().to_string()));
        }

        if let Some(issue_id) = Self::parse_linear_issue_id(external_id) {
            return self
                .find_data_id_by_ext_linear(endpoint, issue_id)
                .await;
        }

        Ok(None)
    }
}
