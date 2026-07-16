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
    pub property_id: PropertyId,
    pub value: PropertyValueCommand,
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
    /// 1-origin page number. Defaults to 1.
    pub page: Option<u32>,
    /// Page size in the inclusive range 1..=100. Defaults to 20.
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

/// Output port for deleting one tenant-scoped Database aggregate.
///
/// Implementations must keep the RelationDefinition preflight and every
/// descendant/root delete in one transaction. Loading the Database is part of
/// the same unit so a caller cannot authorize or return a stale aggregate
/// across a delete race.
#[async_trait::async_trait]
pub trait DatabaseDeletionUnitOfWork:
    Debug + Send + Sync + 'static
{
    async fn delete_atomically(
        &self,
        tenant_id: &TenantId,
        database_id: &DatabaseId,
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

/// Application command for changing RelationDefinition policy and its
/// generated inverse Property. Relation values and RelationEdge projection
/// are intentionally outside this boundary.
#[derive(Debug)]
pub struct ReconfigureRelationDefinitionInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,

    pub tenant_id: &'a TenantId,
    pub source_database_id: &'a DatabaseId,
    pub source_property_id: &'a PropertyId,
    pub expected_generation: RelationGeneration,
    pub forward_cardinality: Option<RelationCardinality>,
    pub reverse_cardinality: Option<RelationCardinality>,
    pub inverse: RelationInverseChange,
    pub on_target_delete: Option<RelationOnDelete>,
}

/// Application command for deleting one source Relation Property and any
/// inverse Property generated by its RelationDefinition.
#[derive(Debug)]
pub struct DeleteRelationDefinitionInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,

    pub tenant_id: &'a TenantId,
    pub source_database_id: &'a DatabaseId,
    pub source_property_id: &'a PropertyId,
    pub expected_generation: RelationGeneration,
}

#[async_trait::async_trait]
pub trait RelationDefinitionMutationInputPort:
    Debug + Send + Sync + 'static
{
    async fn reconfigure(
        &self,
        input: ReconfigureRelationDefinitionInputData<'_>,
    ) -> errors::Result<RelationDefinition>;

    async fn delete(
        &self,
        input: DeleteRelationDefinitionInputData<'_>,
    ) -> errors::Result<RelationDefinition>;
}

#[derive(Clone, Debug)]
pub struct SearchDataInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,

    pub tenant_id: &'a TenantId,
    pub database_id: Option<DatabaseId>,
    pub query: &'a str,
    /// 1-origin page number. Defaults to 1.
    pub page: Option<u32>,
    /// Page size in the inclusive range 1..=100. Defaults to 20.
    pub page_size: Option<u32>,
}

#[async_trait::async_trait]
pub trait SearchDataInputPort: Debug + Send + Sync + 'static {
    async fn execute(
        &self,
        input: &SearchDataInputData,
    ) -> errors::Result<(Vec<Data>, OffsetPaginator)>;
}

/// One bounded, resumable PropertyValue backfill request.
///
/// `after_data_id` is an exclusive, stable cursor within the selected
/// Tenant/Database scope. The checksum seed lets an operator continue a
/// boundary-independent XOR checksum across separate invocations.
#[derive(Debug, Clone)]
pub struct PropertyValueBackfillInputData<'a> {
    pub tenant_id: &'a TenantId,
    pub database_id: &'a DatabaseId,
    pub after_data_id: Option<&'a DataId>,
    pub batch_size: u16,
    pub dry_run: bool,
    pub checksum_seed: [u8; 32],
}

/// Value-free operational evidence for one committed or rolled-back chunk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyValueBackfillReport {
    pub scanned_records: u64,
    pub compared_values: u64,
    pub expected_values: u64,
    pub missing_values: u64,
    pub written_values: u64,
    pub matched_values: u64,
    pub absent_values: u64,
    pub opaque_values: u64,
    pub next_cursor: Option<DataId>,
    pub complete: bool,
    pub parity_checksum: String,
}

#[async_trait::async_trait]
pub trait PropertyValueBackfillInputPort:
    Debug + Send + Sync + 'static
{
    async fn execute(
        &self,
        input: &PropertyValueBackfillInputData<'_>,
    ) -> errors::Result<PropertyValueBackfillReport>;
}

/// Persistence boundary used by the application backfill interactor.
///
/// The adapter owns transaction/locking details; the application boundary
/// owns scope, cursor, batch and dry-run semantics.
#[async_trait::async_trait]
pub trait PropertyValueBackfillOutputPort:
    Debug + Send + Sync + 'static
{
    async fn execute_chunk(
        &self,
        input: &PropertyValueBackfillInputData<'_>,
    ) -> errors::Result<PropertyValueBackfillReport>;
}

#[derive(Debug)]
pub struct DeclareIndexDefinitionInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,

    pub tenant_id: &'a TenantId,
    pub database_id: &'a DatabaseId,
    pub target: IndexTarget,
    pub policy: IndexPolicy,
    pub unique: bool,
}

#[derive(Debug)]
pub struct ReconfigureIndexDefinitionInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,

    pub tenant_id: &'a TenantId,
    pub database_id: &'a DatabaseId,
    pub index_definition_id: &'a IndexDefinitionId,
    pub expected_generation: IndexGeneration,
    pub policy: IndexPolicy,
    pub unique: bool,
}

#[derive(Debug)]
pub struct TransitionIndexProjectionInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,

    pub tenant_id: &'a TenantId,
    pub database_id: &'a DatabaseId,
    pub index_definition_id: &'a IndexDefinitionId,
    pub expected_generation: IndexGeneration,
    pub next_state: IndexProjectionState,
}

#[derive(Debug)]
pub struct FindIndexDefinitionByIdInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,

    pub tenant_id: &'a TenantId,
    pub database_id: &'a DatabaseId,
    pub index_definition_id: &'a IndexDefinitionId,
}

#[derive(Debug)]
pub struct FindIndexDefinitionByTargetInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,

    pub tenant_id: &'a TenantId,
    pub database_id: &'a DatabaseId,
    pub target: &'a IndexTarget,
}

#[derive(Debug)]
pub struct FindIndexDefinitionsInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,

    pub tenant_id: &'a TenantId,
    pub database_id: &'a DatabaseId,
}

/// Application input port for the declarative IndexDefinition control plane.
/// Physical projection and query planning are deliberately outside this port.
#[async_trait::async_trait]
pub trait IndexDefinitionInputPort: Debug + Send + Sync + 'static {
    async fn declare(
        &self,
        input: DeclareIndexDefinitionInputData<'_>,
    ) -> errors::Result<IndexDefinition>;

    async fn reconfigure(
        &self,
        input: ReconfigureIndexDefinitionInputData<'_>,
    ) -> errors::Result<IndexDefinition>;

    async fn transition_projection(
        &self,
        input: TransitionIndexProjectionInputData<'_>,
    ) -> errors::Result<IndexDefinition>;

    async fn find_by_id(
        &self,
        input: FindIndexDefinitionByIdInputData<'_>,
    ) -> errors::Result<Option<IndexDefinition>>;

    async fn find_by_target(
        &self,
        input: FindIndexDefinitionByTargetInputData<'_>,
    ) -> errors::Result<Option<IndexDefinition>>;

    async fn find_all_by_database(
        &self,
        input: FindIndexDefinitionsInputData<'_>,
    ) -> errors::Result<Vec<IndexDefinition>>;
}
