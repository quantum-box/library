use crate::database_app::{
    Data as AppData, Database as AppDatabase, DatabaseApp,
    Property as AppProperty, PropertyData as AppPropertyData,
    PropertyType as AppPropertyType, TypeId as AppTypeId,
};
use crate::domain::{self, *};
use crate::usecase::boundary::*;
use errors;
use std::fmt::Debug;
use std::sync::Arc;
use tachyon_sdk::auth::{ExecutorAction, MultiTenancyAction};
use value_object::OffsetPaginator;

#[derive(Debug)]
pub struct DatabaseAppImpl {
    app: Arc<crate::App>,
}

impl DatabaseAppImpl {
    pub fn new(app: Arc<crate::App>) -> Self {
        Self { app }
    }
}

#[async_trait::async_trait]
impl DatabaseApp for DatabaseAppImpl {
    async fn create_database(
        &self,
        executor: &dyn ExecutorAction,
        multi_tenancy: &dyn MultiTenancyAction,
        name: &str,
    ) -> errors::Result<AppDatabase> {
        let operator_id = multi_tenancy.get_operator_id()?;
        let input = CreateDatabaseInputData {
            executor,
            multi_tenancy,
            database_id: None,
            tenant_id: &operator_id,
            name,
        };
        let database = self.app.create_database().execute(input).await?;
        Ok(AppDatabase {
            id: database.id().to_string(),
            name: database.name().to_string(),
        })
    }

    async fn update_database(
        &self,
        _executor: &dyn ExecutorAction,
        _multi_tenancy: &dyn MultiTenancyAction,
        _id: &str,
        _name: &str,
    ) -> errors::Result<AppDatabase> {
        // TODO: implement update_database
        unimplemented!()
    }

    async fn get_database(
        &self,
        executor: &dyn ExecutorAction,
        multi_tenancy: &dyn MultiTenancyAction,
        id: &str,
    ) -> errors::Result<AppDatabase> {
        let operator_id = multi_tenancy.get_operator_id()?;
        let database_id = DatabaseId::new(id)?;
        let input = GetDatabaseInputData {
            executor,
            multi_tenancy,
            tenant_id: &operator_id,
            database_id: &database_id,
            page: None,
            page_size: None,
        };
        let (database, _, _, _) =
            self.app.get_database_usecase().execute(input).await?;
        Ok(AppDatabase {
            id: database.id().to_string(),
            name: database.name().to_string(),
        })
    }

    async fn delete_database(
        &self,
        executor: &dyn ExecutorAction,
        multi_tenancy: &dyn MultiTenancyAction,
        id: &str,
    ) -> errors::Result<()> {
        let operator_id = multi_tenancy.get_operator_id()?;
        let database_id = DatabaseId::new(id)?;
        let database_id_str = database_id.to_string();
        let input = DeleteDatabaseInputData {
            executor,
            multi_tenancy,
            tenant_id: &operator_id,
            database_id: database_id_str.as_str(),
        };
        self.app.delete_database_usecase().execute(&input).await?;
        Ok(())
    }

    async fn list_databases(
        &self,
        executor: &dyn ExecutorAction,
        multi_tenancy: &dyn MultiTenancyAction,
    ) -> errors::Result<Vec<AppDatabase>> {
        let operator_id = multi_tenancy.get_operator_id()?;
        let input = FindDatabasesInputData {
            executor,
            multi_tenancy,
            tenant_id: &operator_id,
        };
        let databases =
            self.app.find_database_usecase().execute(&input).await?;
        Ok(databases
            .into_iter()
            .map(|database| AppDatabase {
                id: database.id().to_string(),
                name: database.name().to_string(),
            })
            .collect())
    }

    async fn add_property(
        &self,
        executor: &dyn ExecutorAction,
        multi_tenancy: &dyn MultiTenancyAction,
        database_id: &str,
        name: &str,
        property_type: AppPropertyType,
    ) -> errors::Result<AppProperty> {
        let operator_id = multi_tenancy.get_operator_id()?;
        let database_id = DatabaseId::new(database_id)?;
        let property_type = match property_type {
            AppPropertyType::String => domain::PropertyType::String,
            AppPropertyType::Markdown => domain::PropertyType::Markdown,
            AppPropertyType::Id(type_id) => {
                domain::PropertyType::Id(domain::TypeId {
                    auto_generate: type_id.auto_generate,
                })
            }
        };
        let input = AddPropertyInputData {
            executor,
            multi_tenancy,
            tenant_id: &operator_id,
            database_id: &database_id,
            name,
            property_type,
        };
        let property = self.app.add_property().execute(input).await?;
        Ok(AppProperty {
            id: property.id().to_string(),
            database_id: property.database_id().to_string(),
            name: property.name().to_string(),
            property_type: match property.property_type() {
                domain::PropertyType::String => AppPropertyType::String,
                domain::PropertyType::Html
                | domain::PropertyType::Markdown => {
                    AppPropertyType::Markdown
                }
                domain::PropertyType::Id(type_id) => {
                    AppPropertyType::Id(AppTypeId {
                        auto_generate: type_id.auto_generate,
                    })
                }
                _ => AppPropertyType::String,
            },
        })
    }

    async fn update_property(
        &self,
        executor: &dyn ExecutorAction,
        multi_tenancy: &dyn MultiTenancyAction,
        database_id: &str,
        id: &str,
        name: Option<&str>,
        _property_type: Option<AppPropertyType>,
    ) -> errors::Result<AppProperty> {
        let operator_id = multi_tenancy.get_operator_id()?;
        let property_id = PropertyId::new(id)?;
        let database_id = DatabaseId::new(database_id)?;
        let input = UpdatePropertyInputData {
            executor,
            multi_tenancy,
            tenant_id: &operator_id,
            database_id: &database_id,
            property_id: &property_id,
            name,
            property_type: None,
            meta_json: None,
        };
        let property = self.app.update_property().execute(input).await?;
        Ok(AppProperty {
            id: property.id().to_string(),
            database_id: property.database_id().to_string(),
            name: property.name().to_string(),
            property_type: match property.property_type() {
                domain::PropertyType::String => AppPropertyType::String,
                domain::PropertyType::Html
                | domain::PropertyType::Markdown => {
                    AppPropertyType::Markdown
                }
                domain::PropertyType::Id(type_id) => {
                    AppPropertyType::Id(AppTypeId {
                        auto_generate: type_id.auto_generate,
                    })
                }
                _ => AppPropertyType::String,
            },
        })
    }

    async fn delete_property(
        &self,
        executor: &dyn ExecutorAction,
        multi_tenancy: &dyn MultiTenancyAction,
        database_id: &str,
        id: &str,
    ) -> errors::Result<()> {
        let operator_id = multi_tenancy.get_operator_id()?;
        let property_id = PropertyId::new(id)?;
        let database_id = DatabaseId::new(database_id)?;
        let database_id_str = database_id.to_string();
        let property_id_str = property_id.to_string();
        let input = DeletePropertyInputData {
            executor,
            multi_tenancy,
            tenant_id: &operator_id,
            database_id: database_id_str.as_str(),
            property_id: property_id_str.as_str(),
        };
        self.app.delete_property_usecase().execute(&input).await?;
        Ok(())
    }

    async fn list_properties(
        &self,
        executor: &dyn ExecutorAction,
        multi_tenancy: &dyn MultiTenancyAction,
        database_id: &str,
    ) -> errors::Result<Vec<AppProperty>> {
        let operator_id = multi_tenancy.get_operator_id()?;
        let database_id = DatabaseId::new(database_id)?;
        let input = GetDatabaseDefinitionInputData {
            executor,
            multi_tenancy,
            tenant_id: &operator_id,
            database_id: &database_id,
        };
        let (_, properties) = self
            .app
            .get_database_definition_usecase()
            .execute(input)
            .await?;
        Ok(properties
            .into_iter()
            .map(|property| AppProperty {
                id: property.id().to_string(),
                database_id: property.database_id().to_string(),
                name: property.name().to_string(),
                property_type: match property.property_type() {
                    domain::PropertyType::String => AppPropertyType::String,
                    domain::PropertyType::Html
                    | domain::PropertyType::Markdown => {
                        AppPropertyType::Markdown
                    }
                    domain::PropertyType::Id(type_id) => {
                        AppPropertyType::Id(
                            AppTypeId {
                                auto_generate: type_id.auto_generate,
                            },
                        )
                    }
                    _ => AppPropertyType::String,
                },
            })
            .collect())
    }

    async fn add_data(
        &self,
        executor: &dyn ExecutorAction,
        multi_tenancy: &dyn MultiTenancyAction,
        database_id: &str,
        name: &str,
        property_data: Vec<AppPropertyData>,
    ) -> errors::Result<AppData> {
        let operator_id = multi_tenancy.get_operator_id()?;
        let database_id = DatabaseId::new(database_id)?;
        let property_data = property_data
            .into_iter()
            .map(|data| PropertyDataInputData {
                property_id: data.property_id,
                value: data.value,
            })
            .collect();
        let input = AddDataInputData {
            executor,
            multi_tenancy,
            tenant_id: &operator_id,
            name,
            property_data,
            database_id: &database_id,
        };
        let data = self.app.add_data_usecase().execute(input).await?;
        Ok(AppData {
            id: data.id().to_string(),
            database_id: data.database_id().to_string(),
            name: data.name().to_string(),
            property_data: data
                .property_data()
                .iter()
                .map(|data| AppPropertyData {
                    property_id: data.property_id().to_string(),
                    value: data.string_value(),
                })
                .collect(),
            created_at: *data.created_at(),
            updated_at: *data.updated_at(),
        })
    }

    async fn update_data(
        &self,
        executor: &dyn ExecutorAction,
        multi_tenancy: &dyn MultiTenancyAction,
        id: &str,
        name: &str,
        property_data: Vec<AppPropertyData>,
    ) -> errors::Result<AppData> {
        let operator_id = multi_tenancy.get_operator_id()?;
        let data_id = DataId::new(id)?;
        let database_id = DatabaseId::new("")?;
        let property_data = property_data
            .into_iter()
            .map(|data| PropertyDataInputData {
                property_id: data.property_id,
                value: data.value,
            })
            .collect();
        let input = UpdateDataInputData {
            executor,
            multi_tenancy,

            tenant_id: &operator_id,
            database_id: &database_id,
            data_id: &data_id,
            name,
            data: property_data,
        };
        let data = self.app.update_data_usecase().execute(input).await?;
        Ok(AppData {
            id: data.id().to_string(),
            database_id: data.database_id().to_string(),
            name: data.name().to_string(),
            property_data: data
                .property_data()
                .iter()
                .map(|data| AppPropertyData {
                    property_id: data.property_id().to_string(),
                    value: data.string_value(),
                })
                .collect(),
            created_at: *data.created_at(),
            updated_at: *data.updated_at(),
        })
    }

    async fn delete_data(
        &self,
        executor: &dyn ExecutorAction,
        multi_tenancy: &dyn MultiTenancyAction,
        database_id: &str,
        id: &str,
    ) -> errors::Result<()> {
        let operator_id = multi_tenancy.get_operator_id()?;
        let data_id = DataId::new(id)?;
        let database_id = DatabaseId::new(database_id)?;
        let database_id_str = database_id.to_string();
        let data_id_str = data_id.to_string();
        let input = DeleteDataInputData {
            executor,
            multi_tenancy,
            tenant_id: &operator_id,
            database_id: database_id_str.as_str(),
            data_id: data_id_str.as_str(),
        };
        self.app.delete_data_usecase().execute(&input).await?;
        Ok(())
    }

    async fn list_data(
        &self,
        executor: &dyn ExecutorAction,
        multi_tenancy: &dyn MultiTenancyAction,
        database_id: &str,
        _query: Option<&str>,
        page: Option<u32>,
        page_size: Option<u32>,
    ) -> errors::Result<(Vec<AppData>, OffsetPaginator)> {
        let operator_id = multi_tenancy.get_operator_id()?;
        let database_id = DatabaseId::new(database_id)?;
        let input = GetDatabaseInputData {
            executor,
            multi_tenancy,
            tenant_id: &operator_id,
            database_id: &database_id,
            page,
            page_size,
        };
        let (_, _, data, paginator) =
            self.app.get_database_usecase().execute(input).await?;
        Ok((
            data.into_iter()
                .map(|data| AppData {
                    id: data.id().to_string(),
                    database_id: data.database_id().to_string(),
                    name: data.name().to_string(),
                    property_data: data
                        .property_data()
                        .iter()
                        .map(|data| AppPropertyData {
                            property_id: data.property_id().to_string(),
                            value: data.string_value(),
                        })
                        .collect(),
                    created_at: *data.created_at(),
                    updated_at: *data.updated_at(),
                })
                .collect(),
            paginator,
        ))
    }
}
