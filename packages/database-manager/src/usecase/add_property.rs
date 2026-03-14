use std::sync::Arc;

use crate::usecase::{AddPropertyInputData, AddPropertyInputPort};
use std::fmt::Debug;

use crate::domain::{
    Database, DatabaseId, Property, PropertyId, PropertyRepository,
    PropertyType, Relation, RelationId, RelationRepository,
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
        let database = self
            .database_repo
            .get_by_id(input.tenant_id, input.database_id)
            .await?
            .ok_or(anyhow::anyhow!(
                "Database is not found in add property"
            ))?;
        let properties = self
            .property_repo
            .find_all(database.id(), database.tenant_id())
            .await?;

        if properties
            .iter()
            .any(|p| matches!(p.property_type(), PropertyType::Id(_)))
        {
            return Err(errors::business_logic!(
                "Id property already exists"
            ));
        }

        let new_property = Property::new(
            &PropertyId::default(),
            database.tenant_id(),
            database.id(),
            input.name,
            &input.property_type,
            false,
            properties.len() as u32,
        );

        if let PropertyType::Relation(relation) =
            new_property.property_type()
        {
            let target_database = self
                .database_repo
                .get_by_id(input.tenant_id, &relation.database_id)
                .await?
                .ok_or(anyhow::anyhow!(
                    "Relation is not found in add property"
                ))?;

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
