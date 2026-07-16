use std::sync::Arc;

use tachyon_sdk::auth::{ExecutorAction, MultiTenancyAction};
use value_object::RepositoryV1;

use crate::domain::{
    Database, DatabaseId, DeleteRelationDefinitionCommand,
    ReconfigureRelationDefinitionCommand, RelationDefinition,
    RelationSchemaMutationPort,
};
use crate::usecase::{
    database_scope::DatabaseScope, DeleteRelationDefinitionInputData,
    ReconfigureRelationDefinitionInputData,
    RelationDefinitionMutationInputPort,
};

#[derive(Debug, Clone)]
pub struct RelationDefinitionMutationInteractor {
    database_repository: Arc<dyn RepositoryV1<DatabaseId, Database>>,
    schema_mutation: Arc<dyn RelationSchemaMutationPort>,
}

impl RelationDefinitionMutationInteractor {
    pub fn new(
        database_repository: Arc<dyn RepositoryV1<DatabaseId, Database>>,
        schema_mutation: Arc<dyn RelationSchemaMutationPort>,
    ) -> Arc<Self> {
        Arc::new(Self {
            database_repository,
            schema_mutation,
        })
    }

    async fn require_scope(
        &self,
        executor: &dyn ExecutorAction,
        multi_tenancy: &dyn MultiTenancyAction,
        tenant_id: &value_object::TenantId,
        source_database_id: &DatabaseId,
    ) -> errors::Result<()> {
        if executor.is_none()
            || multi_tenancy.operator_id().as_ref() != Some(tenant_id)
            || !executor.has_tenant_id(tenant_id)
        {
            return Err(errors::Error::not_found("resource not found"));
        }
        DatabaseScope::new(tenant_id, source_database_id)
            .require_database(self.database_repository.as_ref())
            .await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl RelationDefinitionMutationInputPort
    for RelationDefinitionMutationInteractor
{
    async fn reconfigure(
        &self,
        input: ReconfigureRelationDefinitionInputData<'_>,
    ) -> errors::Result<RelationDefinition> {
        self.require_scope(
            input.executor,
            input.multi_tenancy,
            input.tenant_id,
            input.source_database_id,
        )
        .await?;
        self.schema_mutation
            .reconfigure_relation_atomically(
                &ReconfigureRelationDefinitionCommand::new(
                    input.tenant_id,
                    input.source_database_id,
                    input.source_property_id,
                    input.expected_generation,
                    input.forward_cardinality,
                    input.reverse_cardinality,
                    input.inverse,
                    input.on_target_delete,
                ),
            )
            .await
    }

    async fn delete(
        &self,
        input: DeleteRelationDefinitionInputData<'_>,
    ) -> errors::Result<RelationDefinition> {
        self.require_scope(
            input.executor,
            input.multi_tenancy,
            input.tenant_id,
            input.source_database_id,
        )
        .await?;
        self.schema_mutation
            .delete_relation_atomically(
                &DeleteRelationDefinitionCommand::new(
                    input.tenant_id,
                    input.source_database_id,
                    input.source_property_id,
                    input.expected_generation,
                ),
            )
            .await
    }
}
