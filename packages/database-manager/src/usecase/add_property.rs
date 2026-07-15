use std::{fmt::Debug, sync::Arc};

use crate::usecase::database_scope::DatabaseScope;
use crate::usecase::{AddPropertyInputData, AddPropertyInputPort};

use crate::domain::{
    next_property_num, validate_property_type_addition, Database,
    DatabaseId, Property, PropertyId, PropertyRepository, PropertyType,
    Relation, RelationId, RelationRepository,
};
use value_object::RepositoryV1;

#[derive(Debug, Clone)]
pub struct AddPropertyInteractorImpl {
    database_repo: Arc<dyn RepositoryV1<DatabaseId, Database>>,
    property_repo: Arc<dyn PropertyRepository>,
    relation_repo: Arc<dyn RelationRepository>,
}

impl AddPropertyInteractorImpl {
    pub fn new(
        database_repo: Arc<dyn RepositoryV1<DatabaseId, Database>>,
        property_repo: Arc<dyn PropertyRepository>,
        relation_repo: Arc<dyn RelationRepository>,
    ) -> Arc<Self> {
        Arc::new(Self {
            database_repo,
            property_repo,
            relation_repo,
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
        let database = DatabaseScope::new(
            input.tenant_id,
            input.database_id,
        )
        .require_database(self.database_repo.as_ref())
        .await?;
        let properties = self
            .property_repo
            .find_all(database.id(), database.tenant_id())
            .await?;

        validate_property_type_addition(&properties, &input.property_type)?;
        let property_num = next_property_num(&properties)?;

        let new_property = Property::new(
            &PropertyId::default(),
            database.tenant_id(),
            database.id(),
            input.name,
            &input.property_type,
            false,
            property_num,
        );

        if let PropertyType::Relation(relation) =
            new_property.property_type()
        {
            let target_database = DatabaseScope::new(
                input.tenant_id,
                &relation.database_id,
            )
            .require_database(self.database_repo.as_ref())
            .await?;

            let relation = Relation::new(
                &RelationId::default(),
                database.tenant_id(),
                database.id(),
                new_property.id(),
                0,
                target_database.id(),
            );
            self.property_repo.create(&new_property).await?;

            self.relation_repo.insert(&relation).await?;
        } else {
            self.property_repo.create(&new_property).await?;
        };

        Ok(new_property)
    }
}
