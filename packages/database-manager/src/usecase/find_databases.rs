use std::sync::Arc;

use crate::domain::{Database, DatabaseId};
use crate::usecase::{FindDatabasesInputData, FindDatabasesInputPort};
use value_object::RepositoryV1;

#[derive(Debug, Clone)]
pub struct FindDatabasesInteractorImpl<D>
where
    D: RepositoryV1<DatabaseId, Database>,
{
    database_repository: Arc<D>,
}

impl<D> FindDatabasesInteractorImpl<D>
where
    D: RepositoryV1<DatabaseId, Database>,
{
    pub fn new(database_repository: Arc<D>) -> Arc<Self> {
        Arc::new(Self {
            database_repository,
        })
    }
}

#[async_trait::async_trait]
impl<D> FindDatabasesInputPort for FindDatabasesInteractorImpl<D>
where
    D: RepositoryV1<DatabaseId, Database> + Sync + Send + 'static,
{
    async fn execute(
        &self,
        input: &FindDatabasesInputData<'_>,
    ) -> errors::Result<Vec<Database>> {
        let databases =
            self.database_repository.find_all(input.tenant_id).await?;
        Ok(databases)
    }
}
