pub extern crate database_domain as domain;

pub mod database_app;
pub mod interface_adapter;
pub mod migration_preflight;
pub mod usecase;

pub mod sdk;

use derive_getters::Getters;
use gateway::{
    DataQueryService, DataRepositoryImpl, DatabaseRepositoryImpl,
    PropertyRepositoryImpl, RelationRepositoryImpl,
};
pub use usecase::boundary::*;
pub use usecase::{DataQuery, FindAllPropertiesInputPort};

use interface_adapter::*;
use std::fmt::Debug;
use std::sync::Arc;
use usecase::{
    AddDataInteractorImpl, AddPropertyInteractorImpl,
    CreateDatabaseInteractorImpl, DeleteDataInteractor,
    DeleteDatabaseInteractor, DeletePropertyInteractor, FindAllProperties,
    FindDatabasesInteractorImpl, GetDataInteractorImpl,
    GetDatabaseDefinition, GetDatabaseInteractorImpl, SearchData,
    UpdateDataInteractorImpl, UpdatePropertyInteractorImpl,
};

// #[async_trait::async_trait]
// pub trait DatabaseManagerClient: Debug + Send + Sync + 'static {
//     async fn create_database(&self, database_name: &str) -> Result<(), Box<dyn std::error::Error>>;
// }

// #[derive(Debug, Clone)]
// pub struct App {}

impl App {
    pub async fn setup_db() -> Arc<persistence::Db> {
        let dsn =
            std::env::var("DATABASE_URL").expect("DATABASE_URL is not set");
        let dsn = format!("{}/{}", dsn, "tachyon_apps_database_manager");
        persistence::Db::new(&dsn).await
    }

    pub async fn migrate(db: Arc<persistence::Db>) -> anyhow::Result<()> {
        migration_preflight::ensure_check_constraints_enforced(
            db.pool().as_ref(),
        )
        .await?;
        sqlx::migrate!("./migrations")
            .run(db.pool().as_ref())
            .await?;
        Ok(())
    }
}

#[derive(Getters, Clone)]
pub struct App {
    // Database CRUD usecases
    create_database: Arc<dyn CreateDatabaseInputPort>,
    add_property: Arc<dyn AddPropertyInputPort>,
    get_database_definition_usecase:
        Arc<dyn GetDatabaseDefinitionInputPort>,
    add_data_usecase: Arc<dyn AddDataInputPort>,
    update_data_usecase: Arc<dyn UpdateDataInputPort>,
    find_database_usecase: Arc<dyn FindDatabasesInputPort>,
    get_data_usecase: Arc<dyn GetDataInputPort>,
    get_database_usecase: Arc<dyn GetDatabaseInputPort>,
    delete_database_usecase: Arc<dyn DeleteDatabaseInputPort>,
    delete_property_usecase: Arc<dyn DeletePropertyInputPort>,
    delete_data_usecase: Arc<dyn DeleteDataInputPort>,
    search_data: Arc<dyn SearchDataInputPort>,
    find_all_properties: Arc<dyn FindAllPropertiesInputPort>,
    update_property: Arc<dyn UpdatePropertyInputPort>,
}

impl Debug for App {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("App").finish_non_exhaustive()
    }
}

impl App {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        create_database_usecase: Arc<dyn CreateDatabaseInputPort>,
        add_property_usecase: Arc<dyn AddPropertyInputPort>,
        get_database_definition_usecase: Arc<
            dyn GetDatabaseDefinitionInputPort,
        >,
        add_data_usecase: Arc<dyn AddDataInputPort>,
        update_data_usecase: Arc<dyn UpdateDataInputPort>,
        find_database_usecase: Arc<dyn FindDatabasesInputPort>,
        get_data_usecase: Arc<dyn GetDataInputPort>,
        get_database_usecase: Arc<dyn GetDatabaseInputPort>,
        delete_database_usecase: Arc<dyn DeleteDatabaseInputPort>,
        delete_property_usecase: Arc<dyn DeletePropertyInputPort>,
        delete_data_usecase: Arc<dyn DeleteDataInputPort>,
        search_data: Arc<dyn SearchDataInputPort>,
        find_all_properties: Arc<dyn FindAllPropertiesInputPort>,
        update_property: Arc<dyn UpdatePropertyInputPort>,
    ) -> Self {
        Self {
            create_database: create_database_usecase,
            add_property: add_property_usecase,
            get_database_definition_usecase,
            add_data_usecase,
            update_data_usecase,
            find_database_usecase,
            get_data_usecase,
            get_database_usecase,
            delete_database_usecase,
            delete_property_usecase,
            delete_data_usecase,
            search_data,
            find_all_properties,
            update_property,
        }
    }
}

pub async fn factory_client(dsn: impl ToString) -> anyhow::Result<App> {
    let dsn = dsn.to_string();
    let db = persistence::Db::new(&dsn).await;

    // sqlx::migrate!("./migrations")
    //     .run(db.pool().as_ref())
    //     .await?;

    let database_repo = DatabaseRepositoryImpl::new(db.clone());
    let property_repo = PropertyRepositoryImpl::new(db.clone());
    let data_repo = DataRepositoryImpl::new(db.clone());
    let relation_repo = RelationRepositoryImpl::new(db.clone());
    let data_query = DataQueryService::new(db.clone());

    let create_database = CreateDatabaseInteractorImpl::new(
        database_repo.clone(),
        property_repo.clone(),
        data_repo.clone(),
    );
    let get_database_usecase = GetDatabaseInteractorImpl::new(
        database_repo.clone(),
        property_repo.clone(),
        data_repo.clone(),
    );
    let add_property_usecase = AddPropertyInteractorImpl::new(
        database_repo.clone(),
        property_repo.clone(),
        relation_repo.clone(),
    );
    let get_database_definition_usecase = GetDatabaseDefinition::new(
        database_repo.clone(),
        property_repo.clone(),
    );
    let add_data_usecase = AddDataInteractorImpl::new(
        database_repo.clone(),
        property_repo.clone(),
        data_repo.clone(),
    );
    let update_data_usecase = UpdateDataInteractorImpl::new(
        database_repo.clone(),
        property_repo.clone(),
        data_repo.clone(),
    );
    let find_database_usecase =
        FindDatabasesInteractorImpl::new(database_repo.clone());
    let get_data_usecase = GetDataInteractorImpl::new(
        database_repo.clone(),
        data_repo.clone(),
    );
    let delete_database_usecase = DeleteDatabaseInteractor::new(
        database_repo.clone(),
        property_repo.clone(),
        data_repo.clone(),
    );
    let delete_property_usecase = DeletePropertyInteractor::new(
        database_repo.clone(),
        property_repo.clone(),
        data_repo.clone(),
    );
    let delete_data_usecase =
        DeleteDataInteractor::new(database_repo.clone(), data_repo.clone());
    let search_data = SearchData::new(
        database_repo.clone(),
        data_repo.clone(),
        data_query,
    );
    let find_all_properties = FindAllProperties::new(
        database_repo.clone(),
        property_repo.clone(),
    );
    let update_property = UpdatePropertyInteractorImpl::new(
        database_repo.clone(),
        property_repo.clone(),
    );
    Ok(App::new(
        create_database,
        add_property_usecase,
        get_database_definition_usecase,
        add_data_usecase,
        update_data_usecase,
        find_database_usecase,
        get_data_usecase,
        get_database_usecase,
        delete_database_usecase,
        delete_property_usecase,
        delete_data_usecase,
        search_data,
        find_all_properties,
        update_property,
    ))
}

#[cfg(feature = "integration_tests")]
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test() {
        let dsn = "postgres://postgres:postgres@localhost:5432/postgres";
        let db = persistence::Db::new(dsn).await;
        let database_repo = DatabaseRepositoryImpl::new(db.clone());
        let property_repo = PropertyRepositoryImpl::new(db.clone());
        let data_repo = DataRepositoryImpl::new(db.clone());

        let create_database_usecase = CreateDatabaseInteractorImpl::new(
            database_repo.clone(),
            property_repo.clone(),
            data_repo.clone(),
        );
        let add_property_usecase = AddPropertyInteractorImpl::new(
            database_repo.clone(),
            property_repo.clone(),
        );
        let get_database_definition_usecase = GetDatabaseDefinition::new(
            database_repo.clone(),
            property_repo.clone(),
        );
        let add_data_usecase = AddDataInteractorImpl::new(
            database_repo.clone(),
            property_repo.clone(),
            data_repo.clone(),
        );
        let update_data_usecase = UpdateDataInteractorImpl::new(
            database_repo.clone(),
            property_repo.clone(),
            data_repo.clone(),
        );
        let client = App::new(
            create_database_usecase,
            add_property_usecase,
            get_database_definition_usecase,
            add_data_usecase,
            update_data_usecase,
        );
    }
}
