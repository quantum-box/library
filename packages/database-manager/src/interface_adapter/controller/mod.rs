mod pb {
    tonic::include_proto!("database_manager");
}

use domain::{Data, DataId, Database, DatabaseId, Property, PropertyData};
use pb::database_manager_server::{DatabaseManager, DatabaseManagerServer};
use std::sync::Arc;
use value_object::*;

use crate::*;

impl From<Database> for pb::Database {
    fn from(val: Database) -> Self {
        Self {
            id: val.id().to_string(),
            name: val.name().to_string(),
        }
    }
}

impl From<Property> for pb::Property {
    fn from(val: Property) -> Self {
        Self {
            id: val.id().to_string(),
            database_id: val.database_id().to_string(),
            name: val.name().to_string(),
            r#type: val.property_type().to_string(),
            is_indexed: *val.is_indexed(),
            property_num: *val.property_num(),
        }
    }
}

impl From<PropertyData> for pb::PropertyData {
    fn from(val: PropertyData) -> Self {
        Self {
            property_id: val.property_id().to_string(),
            value: val.string_value(),
        }
    }
}

impl From<Data> for pb::Data {
    fn from(val: Data) -> Self {
        Self {
            id: val.id().to_string(),
            database_id: val.database_id().to_string(),
            name: val.name().to_string(),
            property_data: val
                .property_data()
                .iter()
                .map(|p| p.clone().into())
                .collect::<Vec<pb::PropertyData>>(),
        }
    }
}

pub struct DatabaseManagerHandlerImpl<DD, GD, FD, GDD, CD, AP>
where
    DD: GetDatabaseDefinitionInputPort,
    GD: GetDatabaseInputPort,
    FD: FindDatabasesInputPort,
    GDD: GetDataInputPort,
    CD: CreateDatabaseInputPort,
    AP: AddPropertyInputPort,
{
    get_database_definition: Arc<DD>,
    get_database: Arc<GD>,
    find_databases: Arc<FD>,
    get_data: Arc<GDD>,
    create_database: Arc<CD>,
    add_property: Arc<AP>,
}

impl<DD, GD, FD, GDD, CD, AP>
    DatabaseManagerHandlerImpl<DD, GD, FD, GDD, CD, AP>
where
    DD: GetDatabaseDefinitionInputPort + Sync + Send + 'static,
    GD: GetDatabaseInputPort + Sync + Send + 'static,
    FD: FindDatabasesInputPort + Sync + Send + 'static,
    GDD: GetDataInputPort + Sync + Send + 'static,
    CD: CreateDatabaseInputPort + Sync + Send + 'static,
    AP: AddPropertyInputPort + Sync + Send + 'static,
{
    pub fn new(
        get_database_definition: Arc<DD>,
        get_database: Arc<GD>,
        find_databases: Arc<FD>,
        get_data: Arc<GDD>,
        create_database: Arc<CD>,
        add_property: Arc<AP>,
    ) -> DatabaseManagerServer<Self> {
        DatabaseManagerServer::new(Self {
            get_database_definition,
            get_database,
            find_databases,
            get_data,
            create_database,
            add_property,
        })
    }
}

#[tonic::async_trait]
impl<DD, GD, FD, GDD, CD, AP> DatabaseManager
    for DatabaseManagerHandlerImpl<DD, GD, FD, GDD, CD, AP>
where
    DD: GetDatabaseDefinitionInputPort + Sync + Send + 'static,
    GD: GetDatabaseInputPort + Sync + Send + 'static,
    FD: FindDatabasesInputPort + Sync + Send + 'static,
    GDD: GetDataInputPort + Sync + Send + 'static,
    CD: CreateDatabaseInputPort + Sync + Send + 'static,
    AP: AddPropertyInputPort + Sync + Send + 'static,
{
    async fn create_database(
        &self,
        req: tonic::Request<pb::CreateDatabaseRequest>,
    ) -> Result<tonic::Response<pb::Database>, tonic::Status> {
        let tenant_id = &TenantId::from_str(
            req.metadata()
                .get("x-tenant-id")
                .ok_or(tonic::Status::not_found("x-tenant-id not found"))?
                .to_str()
                .map_err(|_| {
                    tonic::Status::not_found("x-tenant-id not found")
                })?,
        )
        .map_err(|_| tonic::Status::not_found("x-tenant-id not found"))?;
        let inner = req.into_inner();
        let database = self
            .create_database
            .execute(CreateDatabaseInputData {
                executor: &tachyon_sdk::auth::Executor::SystemUser,
                multi_tenancy: &tachyon_sdk::auth::MultiTenancy::default(),

                database_id: None,
                tenant_id,
                name: &inner.name,
            })
            .await
            .map_err(|e| {
                tracing::error!("{:?}", e);
                tonic::Status::internal(format!(
                    "failed to create database: {e}"
                ))
            })?;
        Ok(tonic::Response::new(database.into()))
    }

    async fn add_property(
        &self,
        req: tonic::Request<pb::AddPropertyRequest>,
    ) -> Result<tonic::Response<pb::Property>, tonic::Status> {
        let tenant_id = &TenantId::from_str(
            req.metadata()
                .get("x-tenant-id")
                .ok_or(tonic::Status::not_found("x-tenant-id not found"))?
                .to_str()
                .map_err(|_| {
                    tonic::Status::not_found("x-tenant-id not found")
                })?,
        )
        .map_err(|_| tonic::Status::not_found("x-tenant-id not found"))?;
        let inner = req.into_inner();
        let database_id = &DatabaseId::from_str(&inner.database_id)
            .map_err(|_| {
                tonic::Status::not_found("database_id not found")
            })?;
        let property = self
            .add_property
            .execute(AddPropertyInputData {
                executor: &tachyon_sdk::auth::Executor::SystemUser,
                multi_tenancy: &tachyon_sdk::auth::MultiTenancy::default(),
                tenant_id,
                database_id,
                name: &inner.name,
                // property_type: &inner.property_type,
                property_type: domain::PropertyType::String,
            })
            .await
            .map_err(|e| {
                tracing::error!("{:?}", e);
                tonic::Status::internal(format!(
                    "failed to add property: {e}"
                ))
            })?;
        Ok(tonic::Response::new(property.into()))
    }
    async fn find_databases(
        &self,
        req: tonic::Request<pb::Empty>,
    ) -> Result<tonic::Response<pb::FindDatabasesResponse>, tonic::Status>
    {
        let tenant_id = &TenantId::from_str(
            req.metadata()
                .get("x-tenant-id")
                .ok_or(tonic::Status::not_found("x-tenant-id not found"))?
                .to_str()
                .map_err(|_| {
                    tonic::Status::not_found("x-tenant-id not found")
                })?,
        )
        .map_err(|_| tonic::Status::not_found("x-tenant-id not found"))?;
        let executor = &tachyon_sdk::auth::Executor::SystemUser;
        let multi_tenancy = &tachyon_sdk::auth::MultiTenancy::default();
        let databases = self
            .find_databases
            .execute(&FindDatabasesInputData {
                executor,
                multi_tenancy,
                tenant_id,
            })
            .await
            .map_err(|e| {
                tracing::error!("{:?}", e);
                tonic::Status::internal(format!(
                    "failed to find databases: {e}"
                ))
            })?;
        Ok(tonic::Response::new(pb::FindDatabasesResponse {
            databases: databases.into_iter().map(|d| d.into()).collect(),
        }))
    }

    async fn get_database_definition(
        &self,
        req: tonic::Request<pb::GetDatabaseDefinitionRequest>,
    ) -> Result<
        tonic::Response<pb::GetDatabaseDefinitionResponse>,
        tonic::Status,
    > {
        let executor = &tachyon_sdk::auth::Executor::SystemUser;
        let multi_tenancy = &tachyon_sdk::auth::MultiTenancy::default();

        let tenant_id = &TenantId::from_str(
            req.metadata()
                .get("x-tenant-id")
                .ok_or(tonic::Status::not_found("x-tenant-id not found"))?
                .to_str()
                .map_err(|_| {
                    tonic::Status::not_found("x-tenant-id not found")
                })?,
        )
        .map_err(|_| tonic::Status::not_found("x-tenant-id not found"))?;
        let database_id = &DatabaseId::from_str(
            &req.into_inner().database_id,
        )
        .map_err(|_| tonic::Status::not_found("database_id not found"))?;
        let (database, properties) = self
            .get_database_definition
            .execute(GetDatabaseDefinitionInputData {
                executor,
                multi_tenancy,
                database_id,
                tenant_id,
            })
            .await
            .map_err(|e| {
                tracing::error!("{:?}", e);
                tonic::Status::internal(format!(
                    "failed to get database definition: {e}"
                ))
            })?;
        Ok(tonic::Response::new(pb::GetDatabaseDefinitionResponse {
            database: Some(database.into()),
            properties: properties.into_iter().map(|p| p.into()).collect(),
        }))
    }

    async fn get_database(
        &self,
        req: tonic::Request<pb::GetDatabaseRequest>,
    ) -> Result<tonic::Response<pb::GetDatabaseResponse>, tonic::Status>
    {
        let tenant_id = &TenantId::from_str(
            req.metadata()
                .get("x-tenant-id")
                .ok_or(tonic::Status::not_found("x-tenant-id not found"))?
                .to_str()
                .map_err(|_| {
                    tonic::Status::not_found("x-tenant-id not found")
                })?,
        )
        .map_err(|_| tonic::Status::not_found("x-tenant-id not found"))?;

        let executor = &tachyon_sdk::auth::Executor::SystemUser;
        let multi_tenancy = &tachyon_sdk::auth::MultiTenancy::default();

        let database_id = &DatabaseId::from_str(
            &req.into_inner().database_id,
        )
        .map_err(|_| tonic::Status::not_found("database_id not found"))?;
        let (database, properties, data, _) = self
            .get_database
            .execute(GetDatabaseInputData {
                executor,
                multi_tenancy,
                database_id,
                tenant_id,
                page: None,
                page_size: None,
            })
            .await
            .map_err(|e| {
                tracing::error!("{:?}", e);
                tonic::Status::internal(format!(
                    "failed to get database: {e}"
                ))
            })?;
        Ok(tonic::Response::new(pb::GetDatabaseResponse {
            database: Some(database.into()),
            properties: properties.into_iter().map(|p| p.into()).collect(),
            data: data.into_iter().map(|d| d.into()).collect(),
        }))
    }

    async fn get_data(
        &self,
        req: tonic::Request<pb::GetDataRequest>,
    ) -> Result<tonic::Response<pb::Data>, tonic::Status> {
        let tenant_id = &TenantId::from_str(
            req.metadata()
                .get("x-tenant-id")
                .ok_or(tonic::Status::not_found("x-tenant-id not found"))?
                .to_str()
                .map_err(|_| {
                    tonic::Status::not_found("x-tenant-id not found")
                })?,
        )
        .map_err(|_| tonic::Status::not_found("x-tenant-id not found"))?;

        let executor = &tachyon_sdk::auth::Executor::SystemUser;
        let multi_tenancy = &tachyon_sdk::auth::MultiTenancy::default();

        let inner = req.into_inner();
        let database_id = &DatabaseId::from_str(&inner.database_id)
            .map_err(|_| {
                tonic::Status::not_found("database_id not found")
            })?;
        let data_id = &DataId::from_str(&inner.data_id)
            .map_err(|_| tonic::Status::not_found("data_id not found"))?;
        let data = self
            .get_data
            .execute(&GetDataInputData {
                executor,
                multi_tenancy,
                database_id,
                data_id,
                tenant_id,
            })
            .await
            .map_err(|e| {
                tracing::error!("{:?}", e);
                tonic::Status::internal(format!("failed to get data: {e}"))
            })?;
        Ok(tonic::Response::new(data.into()))
    }
}
