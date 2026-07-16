use std::sync::Arc;

use crate::domain::{
    Database, DatabaseId, IndexDefinition, IndexDefinitionId,
    IndexDefinitionRepository, IndexGeneration, IndexTarget,
    PropertyDefinition, PropertyDefinitionRepository, RelationDefinition,
    RelationDefinitionRepository,
};
use crate::usecase::{
    database_scope::DatabaseScope, DeclareIndexDefinitionInputData,
    FindIndexDefinitionByIdInputData, FindIndexDefinitionByTargetInputData,
    FindIndexDefinitionsInputData, IndexDefinitionInputPort,
    ReconfigureIndexDefinitionInputData,
    TransitionIndexProjectionInputData,
};
use tachyon_sdk::auth::{ExecutorAction, MultiTenancyAction};
use value_object::{RepositoryV1, TenantId};

#[derive(Debug, Clone)]
pub struct IndexDefinitionInteractor {
    database_repository: Arc<dyn RepositoryV1<DatabaseId, Database>>,
    property_repository: Arc<dyn PropertyDefinitionRepository>,
    relation_repository: Arc<dyn RelationDefinitionRepository>,
    index_repository: Arc<dyn IndexDefinitionRepository>,
}

impl IndexDefinitionInteractor {
    pub fn new(
        database_repository: Arc<dyn RepositoryV1<DatabaseId, Database>>,
        property_repository: Arc<dyn PropertyDefinitionRepository>,
        relation_repository: Arc<dyn RelationDefinitionRepository>,
        index_repository: Arc<dyn IndexDefinitionRepository>,
    ) -> Arc<Self> {
        Arc::new(Self {
            database_repository,
            property_repository,
            relation_repository,
            index_repository,
        })
    }

    async fn require_scope(
        &self,
        executor: &dyn ExecutorAction,
        multi_tenancy: &dyn MultiTenancyAction,
        tenant_id: &TenantId,
        database_id: &DatabaseId,
    ) -> errors::Result<()> {
        if executor.is_none()
            || multi_tenancy.operator_id().as_ref() != Some(tenant_id)
            || !executor.has_tenant_id(tenant_id)
        {
            return Err(Self::not_found());
        }
        DatabaseScope::new(tenant_id, database_id)
            .require_database(self.database_repository.as_ref())
            .await?;
        Ok(())
    }

    fn require_projection_worker(
        executor: &dyn ExecutorAction,
    ) -> errors::Result<()> {
        if !executor.is_system_user() && !executor.is_service_account() {
            return Err(Self::not_found());
        }
        Ok(())
    }

    async fn property(
        &self,
        tenant_id: &TenantId,
        database_id: &DatabaseId,
        property_id: &crate::domain::PropertyId,
    ) -> errors::Result<PropertyDefinition> {
        self.property_repository
            .find_canonical_definition_by_id(
                property_id,
                database_id,
                tenant_id,
            )
            .await?
            .ok_or_else(Self::not_found)
    }

    async fn relation(
        &self,
        tenant_id: &TenantId,
        database_id: &DatabaseId,
        relation_id: &crate::domain::RelationId,
    ) -> errors::Result<RelationDefinition> {
        self.relation_repository
            .find_by_id(tenant_id, database_id, relation_id)
            .await?
            .ok_or_else(Self::not_found)
    }

    async fn require_definition(
        &self,
        tenant_id: &TenantId,
        database_id: &DatabaseId,
        index_definition_id: &IndexDefinitionId,
    ) -> errors::Result<IndexDefinition> {
        self.index_repository
            .find_by_id(tenant_id, database_id, index_definition_id)
            .await?
            .ok_or_else(Self::not_found)
    }

    fn require_generation(
        current: &IndexDefinition,
        expected: IndexGeneration,
    ) -> errors::Result<()> {
        if current.generation() != &expected {
            return Err(errors::Error::conflict(
                "IndexDefinition generation does not match",
            ));
        }
        Ok(())
    }

    fn not_found() -> errors::Error {
        errors::Error::not_found("resource not found")
    }
}

#[async_trait::async_trait]
impl IndexDefinitionInputPort for IndexDefinitionInteractor {
    async fn declare(
        &self,
        input: DeclareIndexDefinitionInputData<'_>,
    ) -> errors::Result<IndexDefinition> {
        self.require_scope(
            input.executor,
            input.multi_tenancy,
            input.tenant_id,
            input.database_id,
        )
        .await?;
        let id = IndexDefinitionId::default();
        let definition = match &input.target {
            IndexTarget::Property(property_id) => {
                let property = self
                    .property(
                        input.tenant_id,
                        input.database_id,
                        property_id,
                    )
                    .await?;
                IndexDefinition::declare_for_property(
                    &id,
                    input.tenant_id,
                    input.database_id,
                    &property,
                    input.policy,
                    input.unique,
                )?
            }
            IndexTarget::Relation(relation_id) => {
                let relation = self
                    .relation(
                        input.tenant_id,
                        input.database_id,
                        relation_id,
                    )
                    .await?;
                IndexDefinition::declare_for_relation(
                    &id,
                    input.tenant_id,
                    input.database_id,
                    &relation,
                    input.policy,
                    input.unique,
                )?
            }
        };
        self.index_repository.insert(&definition).await?;
        Ok(definition)
    }

    async fn reconfigure(
        &self,
        input: ReconfigureIndexDefinitionInputData<'_>,
    ) -> errors::Result<IndexDefinition> {
        self.require_scope(
            input.executor,
            input.multi_tenancy,
            input.tenant_id,
            input.database_id,
        )
        .await?;
        let current = self
            .require_definition(
                input.tenant_id,
                input.database_id,
                input.index_definition_id,
            )
            .await?;
        Self::require_generation(&current, input.expected_generation)?;

        let updated = match current.target() {
            IndexTarget::Property(property_id) => {
                let property = self
                    .property(
                        input.tenant_id,
                        input.database_id,
                        property_id,
                    )
                    .await?;
                current.reconfigure_for_property(
                    &property,
                    input.policy,
                    input.unique,
                )?
            }
            IndexTarget::Relation(relation_id) => {
                let relation = self
                    .relation(
                        input.tenant_id,
                        input.database_id,
                        relation_id,
                    )
                    .await?;
                current.reconfigure_for_relation(
                    &relation,
                    input.policy,
                    input.unique,
                )?
            }
        };
        self.index_repository
            .replace_if_generation(&updated, input.expected_generation)
            .await?;
        Ok(updated)
    }

    async fn transition_projection(
        &self,
        input: TransitionIndexProjectionInputData<'_>,
    ) -> errors::Result<IndexDefinition> {
        Self::require_projection_worker(input.executor)?;
        self.require_scope(
            input.executor,
            input.multi_tenancy,
            input.tenant_id,
            input.database_id,
        )
        .await?;
        let current = self
            .require_definition(
                input.tenant_id,
                input.database_id,
                input.index_definition_id,
            )
            .await?;
        Self::require_generation(&current, input.expected_generation)?;
        match current.target() {
            IndexTarget::Property(property_id) => {
                let property = self
                    .property(
                        input.tenant_id,
                        input.database_id,
                        property_id,
                    )
                    .await?;
                current.validate_current_property_target(&property)?;
            }
            IndexTarget::Relation(relation_id) => {
                let relation = self
                    .relation(
                        input.tenant_id,
                        input.database_id,
                        relation_id,
                    )
                    .await?;
                current.validate_current_relation_target(&relation)?;
            }
        }
        let previous_state = *current.projection_state();
        let updated = current.transition_projection(input.next_state)?;
        self.index_repository
            .transition_projection_if_generation(
                &updated,
                input.expected_generation,
                previous_state,
            )
            .await?;
        Ok(updated)
    }

    async fn find_by_id(
        &self,
        input: FindIndexDefinitionByIdInputData<'_>,
    ) -> errors::Result<Option<IndexDefinition>> {
        self.require_scope(
            input.executor,
            input.multi_tenancy,
            input.tenant_id,
            input.database_id,
        )
        .await?;
        self.index_repository
            .find_by_id(
                input.tenant_id,
                input.database_id,
                input.index_definition_id,
            )
            .await
    }

    async fn find_by_target(
        &self,
        input: FindIndexDefinitionByTargetInputData<'_>,
    ) -> errors::Result<Option<IndexDefinition>> {
        self.require_scope(
            input.executor,
            input.multi_tenancy,
            input.tenant_id,
            input.database_id,
        )
        .await?;
        self.index_repository
            .find_by_target(
                input.tenant_id,
                input.database_id,
                input.target,
            )
            .await
    }

    async fn find_all_by_database(
        &self,
        input: FindIndexDefinitionsInputData<'_>,
    ) -> errors::Result<Vec<IndexDefinition>> {
        self.require_scope(
            input.executor,
            input.multi_tenancy,
            input.tenant_id,
            input.database_id,
        )
        .await?;
        self.index_repository
            .find_all_by_database(input.tenant_id, input.database_id)
            .await
    }
}
