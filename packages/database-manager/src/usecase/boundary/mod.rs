use crate::domain::{self, *};
use errors;
use std::fmt::Debug;
use tachyon_sdk::auth::{ExecutorAction, MultiTenancyAction};
use value_object::*;

#[derive(Debug)]
pub struct CreateDatabaseInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,

    pub database_id: Option<&'a DatabaseId>,
    pub tenant_id: &'a TenantId,
    pub name: &'a str,
}

#[async_trait::async_trait]
pub trait CreateDatabaseInputPort: Debug + Send + Sync + 'static {
    async fn execute(
        &self,
        input: CreateDatabaseInputData<'_>,
    ) -> errors::Result<Database>;
}

#[derive(Debug)]
pub struct AddPropertyInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,

    pub tenant_id: &'a TenantId,
    pub database_id: &'a DatabaseId,
    pub name: &'a str,

    pub property_type: domain::PropertyType,
}

#[async_trait::async_trait]
pub trait AddPropertyInputPort: Debug + Send + Sync + 'static {
    async fn execute(
        &self,
        input: AddPropertyInputData<'_>,
    ) -> errors::Result<Property>;
}

#[derive(Debug, Clone)]
pub struct UpdatePropertyInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,

    pub tenant_id: &'a TenantId,
    pub database_id: &'a DatabaseId,
    pub property_id: &'a PropertyId,

    pub name: Option<&'a str>,
    pub property_type: Option<&'a PropertyType>,
    /// JSON metadata for property configuration (e.g., ext_github repos)
    /// Option<Option<String>>: None = don't update, Some(None) = clear, Some(Some(v)) = set
    pub meta_json: Option<Option<String>>,
}

#[async_trait::async_trait]
pub trait UpdatePropertyInputPort: Debug + Send + Sync + 'static {
    async fn execute(
        &self,
        input: UpdatePropertyInputData<'_>,
    ) -> errors::Result<Property>;
}

// ChoosePropertyType
#[derive(Debug)]
pub struct ChoosePropertyTypeInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    pub tenant_id: &'a TenantId,
    pub name: &'a str,
    pub property_type: PropertyType,
}

#[async_trait::async_trait]
#[cfg_attr(test, mockall::automock)]
pub trait ChoosePropertyTypeInputPort:
    Debug + Send + Sync + 'static
{
    async fn execute(
        &self,
        input: ChoosePropertyTypeInputData<'_>,
    ) -> errors::Result<()>;
}

// AddData
#[derive(Debug)]
pub struct AddDataInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,

    pub tenant_id: &'a TenantId,
    pub name: &'a str,
    pub property_data: Vec<PropertyDataInputData>,
    pub database_id: &'a DatabaseId,
}

#[async_trait::async_trait]
pub trait AddDataInputPort: Debug + Send + Sync + 'static {
    async fn execute(
        &self,
        input: AddDataInputData<'_>,
    ) -> errors::Result<Data>;
}

// UpdatePropertyData
#[derive(Debug)]
pub struct PropertyDataInputData {
    pub property_id: String,
    pub value: String,
}

// UpdateData
#[derive(Debug)]
pub struct UpdateDataInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,

    pub tenant_id: &'a TenantId,
    pub database_id: &'a DatabaseId,
    pub data_id: &'a DataId,
    pub name: &'a str,
    pub data: Vec<PropertyDataInputData>,
}

#[async_trait::async_trait]
pub trait UpdateDataInputPort: Debug + Send + Sync + 'static {
    async fn execute(
        &self,
        input: UpdateDataInputData<'_>,
    ) -> errors::Result<Data>;
}

// GetDatabaseDefinition
#[derive(Debug)]
pub struct GetDatabaseDefinitionInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,

    pub tenant_id: &'a TenantId,
    pub database_id: &'a DatabaseId,
}

#[async_trait::async_trait]
pub trait GetDatabaseDefinitionInputPort:
    Debug + Send + Sync + 'static
{
    async fn execute(
        &self,
        input: GetDatabaseDefinitionInputData<'_>,
    ) -> errors::Result<(Database, Vec<Property>)>;
}

// GetDatabase
#[derive(Debug)]
pub struct GetDatabaseInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,

    pub tenant_id: &'a TenantId,
    pub database_id: &'a DatabaseId,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[async_trait::async_trait]
pub trait GetDatabaseInputPort: Debug + Send + Sync + 'static {
    async fn execute(
        &self,
        input: GetDatabaseInputData<'_>,
    ) -> errors::Result<(Database, Vec<Property>, Vec<Data>, OffsetPaginator)>;
}

// FindDatabases
#[derive(Debug)]
pub struct FindDatabasesInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,

    pub tenant_id: &'a TenantId,
}

#[async_trait::async_trait]
pub trait FindDatabasesInputPort: Debug + Send + Sync + 'static {
    async fn execute(
        &self,
        input: &FindDatabasesInputData<'_>,
    ) -> errors::Result<Vec<Database>>;
}

#[derive(Debug)]
pub struct GetDataInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,

    pub tenant_id: &'a TenantId,
    pub database_id: &'a DatabaseId,
    pub data_id: &'a DataId,
}

#[async_trait::async_trait]
pub trait GetDataInputPort: Debug + Send + Sync + 'static {
    async fn execute(
        &self,
        input: &GetDataInputData<'_>,
    ) -> errors::Result<Data>;
}

// delete database
#[derive(Debug)]
pub struct DeleteDatabaseInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,

    pub tenant_id: &'a str,
    pub database_id: &'a str,
}

#[async_trait::async_trait]
pub trait DeleteDatabaseInputPort: Debug + Send + Sync + 'static {
    async fn execute(
        &self,
        input: &DeleteDatabaseInputData<'_>,
    ) -> errors::Result<Database>;
}

// delete property
#[derive(Debug)]
pub struct DeletePropertyInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,

    pub tenant_id: &'a str,
    pub database_id: &'a str,
    pub property_id: &'a str,
}

#[async_trait::async_trait]
pub trait DeletePropertyInputPort: Debug + Send + Sync + 'static {
    async fn execute(
        &self,
        input: &DeletePropertyInputData<'_>,
    ) -> errors::Result<Property>;
}

// delete data
#[derive(Debug)]
pub struct DeleteDataInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,

    pub tenant_id: &'a str,
    pub database_id: &'a str,
    pub data_id: &'a str,
}

#[async_trait::async_trait]
pub trait DeleteDataInputPort: Debug + Send + Sync + 'static {
    async fn execute(
        &self,
        input: &DeleteDataInputData<'_>,
    ) -> errors::Result<Data>;
}

#[derive(Clone, Debug)]
pub struct AddRelationInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,

    pub tenant_id: &'a str,
    pub database_id: &'a str,
}

#[async_trait::async_trait]
pub trait AddRelationInputPort: Debug + Send + Sync + 'static {
    async fn execute(
        &self,
        input: &AddRelationInputData,
    ) -> errors::Result<()>;
}

#[derive(Clone, Debug)]
pub struct SearchDataInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,

    pub tenant_id: &'a TenantId,
    pub database_id: Option<DatabaseId>,
    pub query: &'a str,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[async_trait::async_trait]
pub trait SearchDataInputPort: Debug + Send + Sync + 'static {
    async fn execute(
        &self,
        input: &SearchDataInputData,
    ) -> errors::Result<(Vec<Data>, OffsetPaginator)>;
}
