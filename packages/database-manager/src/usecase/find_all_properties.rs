use std::{fmt::Debug, sync::Arc};

use crate::domain::{Database, DatabaseId, Property, PropertyRepository};
use value_object::{RepositoryV1, TenantId};

#[derive(Debug, Clone)]
pub struct FindAllPropertiesInputData {
    pub tenant_id: TenantId,
    pub database_id: DatabaseId,
}

#[async_trait::async_trait]
pub trait FindAllPropertiesInputPort: Debug + Send + Sync {
    async fn execute(
        &self,
        input: FindAllPropertiesInputData,
    ) -> errors::Result<Vec<Property>>;
}

#[derive(Debug, Clone)]
pub struct FindAllProperties<D, P>
where
    D: RepositoryV1<DatabaseId, Database>,
    P: PropertyRepository,
{
    database_repo: Arc<D>,
    property_repo: Arc<P>,
}

impl<D, P> FindAllProperties<D, P>
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
impl<D, P> FindAllPropertiesInputPort for FindAllProperties<D, P>
where
    D: RepositoryV1<DatabaseId, Database>,
    P: PropertyRepository,
{
    async fn execute(
        &self,
        input: FindAllPropertiesInputData,
    ) -> errors::Result<Vec<Property>> {
        let database = self
            .database_repo
            .get_by_id(&input.tenant_id, &input.database_id)
            .await?
            .ok_or(errors::Error::not_found("database"))?;
        Ok(self
            .property_repo
            .find_all(database.id(), database.tenant_id())
            .await?)
    }
}
