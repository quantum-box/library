use std::sync::Arc;

use crate::domain::{
    DataRepository, Database, DatabaseId, Property, PropertyId,
    PropertyRepository,
};
use crate::usecase::{DeletePropertyInputData, DeletePropertyInputPort};
use value_object::RepositoryV1;

#[derive(Debug, Clone)]
pub struct DeletePropertyInteractor {
    database_repository: Arc<dyn RepositoryV1<DatabaseId, Database>>,
    property_repository: Arc<dyn PropertyRepository>,
    data_repository: Arc<dyn DataRepository>,
}

impl DeletePropertyInteractor {
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
        let property = self
            .property_repository
            .find_by_id(&property_id, database.id(), database.tenant_id())
            .await?
            .ok_or(errors::not_found!(
                "property is not found in delete property"
            ))?;
        let all_data = self
            .data_repository
            .find_all(database.id(), database.tenant_id())
            .await?;

        let all_data = all_data.delete_property_data(property.id());

        self.data_repository.update_all(&all_data).await?;
        self.property_repository
            .delete(database.tenant_id(), property.id())
            .await?;
        Ok(property)
    }
}
