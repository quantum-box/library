use std::sync::Arc;

use crate::domain::{
    Database, DatabaseId, Property, PropertyId, PropertySchemaMutationPort,
};
use crate::usecase::{DeletePropertyInputData, DeletePropertyInputPort};
use value_object::RepositoryV1;

#[derive(Debug, Clone)]
pub struct DeletePropertyInteractor {
    database_repository: Arc<dyn RepositoryV1<DatabaseId, Database>>,
    schema_mutation: Arc<dyn PropertySchemaMutationPort>,
}

impl DeletePropertyInteractor {
    pub fn new(
        database_repository: Arc<dyn RepositoryV1<DatabaseId, Database>>,
        schema_mutation: Arc<dyn PropertySchemaMutationPort>,
    ) -> Arc<Self> {
        Arc::new(Self {
            database_repository,
            schema_mutation,
        })
    }
}

#[async_trait::async_trait]
impl DeletePropertyInputPort for DeletePropertyInteractor {
    async fn execute(
        &self,
        input: &DeletePropertyInputData<'_>,
    ) -> errors::Result<Property> {
        let property_id = PropertyId::new(input.property_id)?;
        let database = self
            .database_repository
            .get_by_id(
                &input.tenant_id.parse()?,
                &input.database_id.parse()?,
            )
            .await?
            .ok_or(errors::not_found!(
                "database is not found in delete property"
            ))?;
        self.schema_mutation
            .delete_property_atomically(
                database.tenant_id(),
                database.id(),
                &property_id,
            )
            .await
    }
}
