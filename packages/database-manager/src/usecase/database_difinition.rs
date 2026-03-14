use std::sync::Arc;

use crate::domain::{Database, DatabaseId, Property, PropertyRepository};
use crate::usecase::{
    GetDatabaseDefinitionInputData, GetDatabaseDefinitionInputPort,
};
use value_object::RepositoryV1;

#[derive(Debug, Clone)]
pub struct GetDatabaseDefinition<D, P>
where
    D: RepositoryV1<DatabaseId, Database>,
    P: PropertyRepository,
{
    database_repo: Arc<D>,
    property_repo: Arc<P>,
}

impl<D, P> GetDatabaseDefinition<D, P>
where
    D: RepositoryV1<DatabaseId, Database>,
    P: PropertyRepository,
{
    pub fn new(database_repo: Arc<D>, property_repo: Arc<P>) -> Arc<Self> {
        Arc::new(Self {
            database_repo,
            property_repo,
        })
    }
}

#[async_trait::async_trait]
impl<D, P> GetDatabaseDefinitionInputPort for GetDatabaseDefinition<D, P>
where
    D: RepositoryV1<DatabaseId, Database> + Sync + Send + 'static,
    P: PropertyRepository + Sync + Send + 'static,
{
    #[tracing::instrument(
        name = "GetDatabaseDefinition::execute",
        skip(self)
    )]
    async fn execute(
        &self,
        input: GetDatabaseDefinitionInputData<'_>,
    ) -> errors::Result<(Database, Vec<Property>)> {
        let database = self
            .database_repo
            .get_by_id(input.tenant_id, input.database_id)
            .await?
            .ok_or(errors::not_found!(
                "database is not found in get database definition"
            ))?;
        let properties = self
            .property_repo
            .find_all(database.id(), database.tenant_id())
            .await?;
        Ok((database, properties))
    }
}
