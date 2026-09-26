use std::{fmt::Debug, sync::Arc};

use crate::usecase::database_scope::DatabaseScope;
use crate::usecase::{AddPropertyInputData, AddPropertyInputPort};

use crate::domain::{
    AddPropertyCommand, Database, DatabaseId, Property, PropertyRepository,
    PropertyType, RelationRepository,
};
use value_object::RepositoryV1;

#[derive(Debug, Clone)]
pub struct AddPropertyInteractorImpl {
    database_repo: Arc<dyn RepositoryV1<DatabaseId, Database>>,
    property_repo: Arc<dyn PropertyRepository>,
}

impl AddPropertyInteractorImpl {
    pub fn new(
        database_repo: Arc<dyn RepositoryV1<DatabaseId, Database>>,
        property_repo: Arc<dyn PropertyRepository>,
        // Retained for source compatibility. Definition writes are owned by
        // the Property-schema unit of work.
        _relation_repo: Arc<dyn RelationRepository>,
    ) -> Arc<Self> {
        Arc::new(Self {
            database_repo,
            property_repo,
        })
    }
}

#[async_trait::async_trait]
impl AddPropertyInputPort for AddPropertyInteractorImpl {
    #[tracing::instrument(skip(self))]
    async fn execute(
        &self,
        input: AddPropertyInputData<'_>,
    ) -> errors::Result<Property> {
        let database =
            DatabaseScope::new(input.tenant_id, input.database_id)
                .require_database(self.database_repo.as_ref())
                .await?;

        if let PropertyType::Relation(relation) = &input.property_type {
            DatabaseScope::new(input.tenant_id, &relation.database_id)
                .require_database(self.database_repo.as_ref())
                .await?;
        }

        let command = AddPropertyCommand::new(
            database.tenant_id(),
            database.id(),
            input.name,
            &input.property_type,
        );

        self.property_repo.add_property_atomically(&command).await
    }
}
