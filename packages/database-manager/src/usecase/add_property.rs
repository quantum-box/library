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

        validate_rollup_target(
            self.database_repo.as_ref(),
            self.property_repo.as_ref(),
            input.tenant_id,
            input.database_id,
            &input.property_type,
        )
        .await?;
        let command = AddPropertyCommand::new_with_display_name(
            database.tenant_id(),
            database.id(),
            input.name,
            input.display_name.unwrap_or(input.name),
            &input.property_type,
        );

        self.property_repo.add_property_atomically(&command).await
    }
}

/// Validate the target schema through the same tenant-scoped repository as reads.
pub(crate) async fn validate_rollup_target(
    database_repo: &dyn RepositoryV1<DatabaseId, Database>,
    property_repo: &dyn PropertyRepository,
    tenant_id: &value_object::TenantId,
    database_id: &DatabaseId,
    property_type: &PropertyType,
) -> errors::Result<()> {
    let PropertyType::Rollup(config) = property_type else {
        return Ok(());
    };
    config.validate()?;
    let source = property_repo.find_all(database_id, tenant_id).await?;
    let relation = source
        .iter()
        .find(|p| p.id().as_str() == config.relation_property_id)
        .ok_or_else(|| errors::Error::invalid("unknown rollup relation"))?;
    let PropertyType::Relation(relation) = relation.property_type() else {
        return Err(errors::Error::invalid("rollup requires a relation"));
    };
    DatabaseScope::new(tenant_id, &relation.database_id)
        .require_database(database_repo)
        .await?;
    let targets = property_repo
        .find_all(&relation.database_id, tenant_id)
        .await?;
    let target = targets
        .iter()
        .find(|p| p.id().as_str() == config.target_property_id)
        .ok_or_else(|| {
            errors::Error::invalid("unknown rollup target property")
        })?;
    if !matches!(
        target.property_type(),
        PropertyType::Decimal(_)
            | PropertyType::Integer
            | PropertyType::Formula(_)
    ) {
        return Err(errors::Error::invalid(
            "rollup target must be decimal, integer, or formula",
        ));
    }
    Ok(())
}
