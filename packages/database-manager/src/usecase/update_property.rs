//! update property
//!
//! TODO: add English documentation
//! TODO: add English documentation
//!
//! TODO: add English documentation

use std::sync::Arc;

use crate::usecase::{UpdatePropertyInputData, UpdatePropertyInputPort};
use std::fmt::Debug;

use crate::domain::{Database, DatabaseId, Property, PropertyRepository};
use value_object::RepositoryV1;

#[derive(Debug, Clone)]
pub struct UpdatePropertyInteractorImpl {
    database_repo: Arc<dyn RepositoryV1<DatabaseId, Database>>,
    property_repo: Arc<dyn PropertyRepository>,
}

impl UpdatePropertyInteractorImpl {
    pub fn new(
        database_repo: Arc<dyn RepositoryV1<DatabaseId, Database>>,
        property_repo: Arc<dyn PropertyRepository>,
    ) -> Arc<Self> {
        Arc::new(Self {
            database_repo,
            property_repo,
        })
    }
}

#[async_trait::async_trait]
impl UpdatePropertyInputPort for UpdatePropertyInteractorImpl {
    #[tracing::instrument(skip(self))]
    async fn execute(
        &self,
        input: UpdatePropertyInputData<'_>,
    ) -> errors::Result<Property> {
        let database = self
            .database_repo
            .get_by_id(input.tenant_id, input.database_id)
            .await?
            .ok_or(errors::not_found!("Database not found"))?;

        let property = self
            .property_repo
            .find_by_id(input.property_id, database.id(), input.tenant_id)
            .await?
            .ok_or(errors::not_found!("Property not found"))?;

        let updated_property = property.update_with_meta_json(
            input.name,
            input.property_type,
            input.meta_json,
        )?;

        self.property_repo.update(&updated_property).await?;

        Ok(updated_property)
    }
}
