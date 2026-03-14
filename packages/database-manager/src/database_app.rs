//! DatabaseApp trait and supporting types.
//!
//! Previously defined in `tachyon_apps::database`, now
//! local to the database-manager crate.

use chrono::{DateTime, Utc};
use std::fmt::Debug;
use tachyon_sdk::auth::{ExecutorAction, MultiTenancyAction};
use value_object::OffsetPaginator;

#[async_trait::async_trait]
pub trait DatabaseApp: Debug + Send + Sync + 'static {
    // database
    async fn create_database(
        &self,
        executor: &dyn ExecutorAction,
        multi_tenancy: &dyn MultiTenancyAction,
        name: &str,
    ) -> errors::Result<Database>;
    async fn update_database(
        &self,
        executor: &dyn ExecutorAction,
        multi_tenancy: &dyn MultiTenancyAction,
        id: &str,
        name: &str,
    ) -> errors::Result<Database>;
    async fn get_database(
        &self,
        executor: &dyn ExecutorAction,
        multi_tenancy: &dyn MultiTenancyAction,
        id: &str,
    ) -> errors::Result<Database>;
    async fn delete_database(
        &self,
        executor: &dyn ExecutorAction,
        multi_tenancy: &dyn MultiTenancyAction,
        id: &str,
    ) -> errors::Result<()>;
    async fn list_databases(
        &self,
        executor: &dyn ExecutorAction,
        multi_tenancy: &dyn MultiTenancyAction,
    ) -> errors::Result<Vec<Database>>;

    // property
    async fn add_property(
        &self,
        executor: &dyn ExecutorAction,
        multi_tenancy: &dyn MultiTenancyAction,
        database_id: &str,
        name: &str,
        property_type: PropertyType,
    ) -> errors::Result<Property>;
    async fn update_property(
        &self,
        executor: &dyn ExecutorAction,
        multi_tenancy: &dyn MultiTenancyAction,
        database_id: &str,
        id: &str,
        name: Option<&str>,
        property_type: Option<PropertyType>,
    ) -> errors::Result<Property>;
    async fn delete_property(
        &self,
        executor: &dyn ExecutorAction,
        multi_tenancy: &dyn MultiTenancyAction,
        database_id: &str,
        id: &str,
    ) -> errors::Result<()>;
    async fn list_properties(
        &self,
        executor: &dyn ExecutorAction,
        multi_tenancy: &dyn MultiTenancyAction,
        database_id: &str,
    ) -> errors::Result<Vec<Property>>;

    // data
    async fn add_data(
        &self,
        executor: &dyn ExecutorAction,
        multi_tenancy: &dyn MultiTenancyAction,
        database_id: &str,
        name: &str,
        property_data: Vec<PropertyData>,
    ) -> errors::Result<Data>;
    async fn update_data(
        &self,
        executor: &dyn ExecutorAction,
        multi_tenancy: &dyn MultiTenancyAction,
        id: &str,
        name: &str,
        property_data: Vec<PropertyData>,
    ) -> errors::Result<Data>;
    async fn delete_data(
        &self,
        executor: &dyn ExecutorAction,
        multi_tenancy: &dyn MultiTenancyAction,
        database_id: &str,
        id: &str,
    ) -> errors::Result<()>;
    async fn list_data(
        &self,
        executor: &dyn ExecutorAction,
        multi_tenancy: &dyn MultiTenancyAction,
        database_id: &str,
        query: Option<&str>,
        page: Option<u32>,
        page_size: Option<u32>,
    ) -> errors::Result<(Vec<Data>, OffsetPaginator)>;
}

#[derive(Debug, Clone)]
pub struct Database {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Property {
    pub id: String,
    pub database_id: String,
    pub name: String,
    pub property_type: PropertyType,
}

#[derive(Debug, Clone)]
pub enum PropertyType {
    String,
    Markdown,
    Id(TypeId),
}

#[derive(Debug, Clone)]
pub struct TypeId {
    pub auto_generate: bool,
}

#[derive(Debug, Clone)]
pub struct Data {
    pub id: String,
    pub database_id: String,
    pub name: String,
    pub property_data: Vec<PropertyData>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct PropertyData {
    pub property_id: String,
    pub value: String,
}
