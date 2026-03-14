pub extern crate database_domain as domain;

pub mod database_app;
pub mod interface_adapter;
pub mod usecase;

pub mod sdk;

// Re-export sync engines
pub use inbound_sync;
pub use outbound_sync;

use derive_getters::Getters;
use gateway::{
    DataQueryService, DataRepositoryImpl, DatabaseRepositoryImpl,
    PropertyRepositoryImpl, RelationRepositoryImpl,
};
pub use usecase::boundary::*;

use interface_adapter::*;
use std::fmt::Debug;
use std::sync::Arc;
use usecase::*;

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

    // Inbound Sync usecases (optional for backward compatibility)
    #[getter(skip)]
    list_integrations:
        Option<Arc<dyn inbound_sync::usecase::ListIntegrationsInputPort>>,
    #[getter(skip)]
    list_connections:
        Option<Arc<dyn inbound_sync::usecase::ListConnectionsInputPort>>,
    #[getter(skip)]
    register_webhook_endpoint: Option<
        Arc<dyn inbound_sync::usecase::RegisterWebhookEndpointInputPort>,
    >,
    #[getter(skip)]
    update_webhook_endpoint: Option<
        Arc<dyn inbound_sync::usecase::UpdateWebhookEndpointInputPort>,
    >,
    #[getter(skip)]
    delete_webhook_endpoint: Option<
        Arc<dyn inbound_sync::usecase::DeleteWebhookEndpointInputPort>,
    >,

    // Outbound Sync usecases (optional for backward compatibility)
    #[getter(skip)]
    sync_data: Option<Arc<dyn outbound_sync::usecase::SyncDataInputPort>>,
}

impl Debug for App {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("App")
            .field("has_inbound_sync", &self.list_integrations.is_some())
            .field("has_outbound_sync", &self.sync_data.is_some())
            .finish_non_exhaustive()
    }
}

impl App {
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
            // Sync usecases default to None
            list_integrations: None,
            list_connections: None,
            register_webhook_endpoint: None,
            update_webhook_endpoint: None,
            delete_webhook_endpoint: None,
            sync_data: None,
        }
    }

    // Inbound Sync usecase getters
    pub fn list_integrations(
        &self,
    ) -> &Arc<dyn inbound_sync::usecase::ListIntegrationsInputPort> {
        self.list_integrations
            .as_ref()
            .expect("list_integrations not configured")
    }

    pub fn list_connections(
        &self,
    ) -> &Arc<dyn inbound_sync::usecase::ListConnectionsInputPort> {
        self.list_connections
            .as_ref()
            .expect("list_connections not configured")
    }

    pub fn register_webhook_endpoint(
        &self,
    ) -> &Arc<dyn inbound_sync::usecase::RegisterWebhookEndpointInputPort>
    {
        self.register_webhook_endpoint
            .as_ref()
            .expect("register_webhook_endpoint not configured")
    }

    pub fn update_webhook_endpoint(
        &self,
    ) -> &Arc<dyn inbound_sync::usecase::UpdateWebhookEndpointInputPort>
    {
        self.update_webhook_endpoint
            .as_ref()
            .expect("update_webhook_endpoint not configured")
    }

    pub fn delete_webhook_endpoint(
        &self,
    ) -> &Arc<dyn inbound_sync::usecase::DeleteWebhookEndpointInputPort>
    {
        self.delete_webhook_endpoint
            .as_ref()
            .expect("delete_webhook_endpoint not configured")
    }

    // Outbound Sync usecase getter
    pub fn sync_data(
        &self,
    ) -> &Arc<dyn outbound_sync::usecase::SyncDataInputPort> {
        self.sync_data.as_ref().expect("sync_data not configured")
    }

    // Builder methods for sync usecases
    pub fn with_inbound_sync(
        mut self,
        list_integrations: Arc<
            dyn inbound_sync::usecase::ListIntegrationsInputPort,
        >,
        list_connections: Arc<
            dyn inbound_sync::usecase::ListConnectionsInputPort,
        >,
        register_webhook_endpoint: Arc<
            dyn inbound_sync::usecase::RegisterWebhookEndpointInputPort,
        >,
        update_webhook_endpoint: Arc<
            dyn inbound_sync::usecase::UpdateWebhookEndpointInputPort,
        >,
        delete_webhook_endpoint: Arc<
            dyn inbound_sync::usecase::DeleteWebhookEndpointInputPort,
        >,
    ) -> Self {
        self.list_integrations = Some(list_integrations);
        self.list_connections = Some(list_connections);
        self.register_webhook_endpoint = Some(register_webhook_endpoint);
        self.update_webhook_endpoint = Some(update_webhook_endpoint);
        self.delete_webhook_endpoint = Some(delete_webhook_endpoint);
        self
    }

    pub fn with_outbound_sync(
        mut self,
        sync_data: Arc<dyn outbound_sync::usecase::SyncDataInputPort>,
    ) -> Self {
        self.sync_data = Some(sync_data);
        self
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
    let search_data = SearchData::new(data_repo.clone(), data_query);
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
