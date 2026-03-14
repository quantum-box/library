use std::sync::Arc;

use crate::domain::{Data, DataRepository, Database, DatabaseId};
use crate::usecase::{GetDataInputData, GetDataInputPort};
use value_object::RepositoryV1;

#[derive(Debug, Clone)]
pub struct GetDataInteractorImpl<D, DD>
where
    D: RepositoryV1<DatabaseId, Database>,
    DD: DataRepository,
{
    database_repository: Arc<D>,
    data_repository: Arc<DD>,
}

impl<D, DD> GetDataInteractorImpl<D, DD>
where
    D: RepositoryV1<DatabaseId, Database>,
    DD: DataRepository,
{
    pub fn new(
        database_repository: Arc<D>,
        data_repository: Arc<DD>,
    ) -> Arc<Self> {
        Arc::new(Self {
            database_repository,
            data_repository,
        })
    }
}

#[async_trait::async_trait]
impl<D, DD> GetDataInputPort for GetDataInteractorImpl<D, DD>
where
    D: RepositoryV1<DatabaseId, Database> + Sync + Send + 'static,
    DD: DataRepository + Sync + Send + 'static,
{
    async fn execute(
        &self,
        input: &GetDataInputData<'_>,
    ) -> errors::Result<Data> {
        self.database_repository
            .get_by_id(input.tenant_id, input.database_id)
            .await?
            .ok_or(errors::not_found!(
                "Database is not found in get data"
            ))?;
        let data = self
            .data_repository
            .find_by_id(input.data_id, input.database_id, input.tenant_id)
            .await?
            .ok_or(errors::not_found!("Data is not found in get data"))?;
        Ok(data)
    }
}
