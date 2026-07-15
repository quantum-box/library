use std::sync::Arc;

use crate::domain::{Data, DataRepository, Database, DatabaseId};
use crate::usecase::database_scope::DatabaseScope;
use crate::usecase::{DeleteDataInputData, DeleteDataInputPort};
use value_object::RepositoryV1;

#[derive(Debug, Clone)]
pub struct DeleteDataInteractor {
    database_repository: Arc<dyn RepositoryV1<DatabaseId, Database>>,
    data_repository: Arc<dyn DataRepository>,
}

impl DeleteDataInteractor {
    pub fn new(
        database_repository: Arc<dyn RepositoryV1<DatabaseId, Database>>,
        data_repository: Arc<dyn DataRepository>,
    ) -> Arc<Self> {
        Arc::new(Self {
            database_repository,
            data_repository,
        })
    }
}

#[async_trait::async_trait]
impl DeleteDataInputPort for DeleteDataInteractor {
    async fn execute(
        &self,
        input: &DeleteDataInputData<'_>,
    ) -> errors::Result<Data> {
        let tenant_id = input.tenant_id.parse()?;
        let database_id = input.database_id.parse()?;
        let data_id = input.data_id.parse()?;
        let scope = DatabaseScope::new(&tenant_id, &database_id);
        let database = scope
            .require_database(self.database_repository.as_ref())
            .await?;
        let data = scope
            .require_data(self.data_repository.as_ref(), &data_id)
            .await?;

        self.data_repository
            .delete(database.tenant_id(), database.id(), data.id())
            .await?;
        Ok(data)
    }
}
