use std::fmt::Debug;
use std::sync::Arc;

use crate::domain::{
    DataRepository, Database, DatabaseId, PropertyRepository,
};
use crate::usecase::{CreateDatabaseInputData, CreateDatabaseInputPort};
use value_object::RepositoryV1;

#[derive(Debug, Clone)]
pub struct CreateDatabaseInteractorImpl<D, P, DT>
where
    D: RepositoryV1<DatabaseId, Database>,
    P: PropertyRepository,
    DT: DataRepository,
{
    database_repo: Arc<D>,
    #[allow(dead_code)]
    property_repo: Arc<P>,
    #[allow(dead_code)]
    data_repo: Arc<DT>,
}

impl<D, P, DT> CreateDatabaseInteractorImpl<D, P, DT>
where
    D: RepositoryV1<DatabaseId, Database> + Debug + Send + Sync + 'static,
    P: PropertyRepository + Debug + Send + Sync + 'static,
    DT: DataRepository + Debug + Send + Sync + 'static,
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
impl<D, P, DT> CreateDatabaseInputPort
    for CreateDatabaseInteractorImpl<D, P, DT>
where
    D: RepositoryV1<DatabaseId, Database> + Debug + Send + Sync + 'static,
    P: PropertyRepository + Debug + Send + Sync + 'static,
    DT: DataRepository + Debug + Send + Sync + 'static,
{
    async fn execute(
        &self,
        input: CreateDatabaseInputData<'_>,
    ) -> errors::Result<Database> {
        let database_id = if let Some(database_id) = input.database_id {
            database_id.clone()
        } else {
            DatabaseId::default()
        };
        let database =
            Database::new(&database_id, input.tenant_id, input.name);
        self.database_repo.save(&database).await?;
        Ok(database)
    }
}
