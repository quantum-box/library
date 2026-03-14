use std::sync::Arc;

use crate::domain::{
    DataRepository, Database, DatabaseId, PropertyRepository,
};
use crate::usecase::{DeleteDatabaseInputData, DeleteDatabaseInputPort};
use std::str::FromStr;
use value_object::RepositoryV1;

#[derive(Debug, Clone)]
pub struct DeleteDatabaseInteractor {
    database_repository: Arc<dyn RepositoryV1<DatabaseId, Database>>,
    property_repository: Arc<dyn PropertyRepository>,
    data_repository: Arc<dyn DataRepository>,
}

impl DeleteDatabaseInteractor {
    pub fn new(
        database_repository: Arc<dyn RepositoryV1<DatabaseId, Database>>,
        property_repository: Arc<dyn PropertyRepository>,
        data_repository: Arc<dyn DataRepository>,
    ) -> Arc<Self> {
        Arc::new(Self {
            database_repository,
            property_repository,
            data_repository,
        })
    }
}

#[async_trait::async_trait]
impl DeleteDatabaseInputPort for DeleteDatabaseInteractor {
    async fn execute(
        &self,
        input: &DeleteDatabaseInputData<'_>,
    ) -> errors::Result<Database> {
        let database_id = DatabaseId::from_str(input.database_id)?;
        let database = self
            .database_repository
            .get_by_id(&input.tenant_id.parse()?, &database_id)
            .await?
            .ok_or(errors::not_found!(
                "database is not found in delete database"
            ))?;

        self.data_repository
            .delete_all(database.tenant_id(), database.id())
            .await?;
        self.property_repository
            .delete_all(database.tenant_id(), database.id())
            .await?;
        self.database_repository
            .delete(database.tenant_id(), database.id())
            .await?;
        Ok(database)
    }
}
