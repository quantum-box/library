use std::sync::Arc;

use crate::domain::{
    Data, DataRepository, Database, DatabaseId, Property,
    PropertyRepository,
};
use crate::usecase::{GetDatabaseInputData, GetDatabaseInputPort};
use value_object::{OffsetPaginator, RepositoryV1};

#[derive(Debug, Clone)]
pub struct GetDatabaseInteractorImpl<D, P, DT>
where
    D: RepositoryV1<DatabaseId, Database>,
    P: PropertyRepository,
    DT: DataRepository,
{
    database_repo: Arc<D>,
    property_repo: Arc<P>,
    data_repo: Arc<DT>,
}

impl<D, P, DT> GetDatabaseInteractorImpl<D, P, DT>
where
    D: RepositoryV1<DatabaseId, Database>,
    P: PropertyRepository,
    DT: DataRepository,
{
    pub fn new(
        database_repo: Arc<D>,
        property_repo: Arc<P>,
        data_repo: Arc<DT>,
    ) -> Arc<Self> {
        Arc::new(Self {
            database_repo,
            property_repo,
            data_repo,
        })
    }
}

#[async_trait::async_trait]
impl<D, P, DT> GetDatabaseInputPort for GetDatabaseInteractorImpl<D, P, DT>
where
    D: RepositoryV1<DatabaseId, Database> + Sync + Send + 'static,
    P: PropertyRepository + Sync + Send + 'static,
    DT: DataRepository + Sync + Send + 'static,
{
    async fn execute(
        &self,
        input: GetDatabaseInputData<'_>,
    ) -> errors::Result<(Database, Vec<Property>, Vec<Data>, OffsetPaginator)>
    {
        let page = input.page.unwrap_or(1);
        let page_size = input.page_size.unwrap_or(10);
        let database = self
            .database_repo
            .get_by_id(input.tenant_id, input.database_id)
            .await?
            .ok_or(errors::not_found!(
                "database is not found in get database"
            ))?;
        let properties = self
            .property_repo
            .find_all(database.id(), database.tenant_id())
            .await?;
        let (data, paginator) = self
            .data_repo
            .find_all_with_paging(
                database.tenant_id(),
                database.id(),
                page,
                page_size,
            )
            .await?;
        Ok((database, properties, data.value().to_vec(), paginator))
    }
}
