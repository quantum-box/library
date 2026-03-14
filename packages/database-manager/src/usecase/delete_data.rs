use std::sync::Arc;

use crate::domain::{Data, DataRepository, Database, DatabaseId};
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
        let database = self
            .database_repository
            .get_by_id(
                &input.tenant_id.parse()?,
                &input.database_id.parse()?,
            )
            .await?
            .ok_or(errors::not_found!(
                "database is not found in delete data"
            ))?;
        let data = self
            .data_repository
            .find_by_id(
                &input.data_id.parse()?,
                database.id(),
                &input.tenant_id.parse()?,
            )
            .await?
            .ok_or(errors::not_found!(
                "data is not found in delete data"
            ))?;

        self.data_repository
            .delete(data.tenant_id(), database.id(), data.id())
            .await?;
        Ok(data)
    }
}
