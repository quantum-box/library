use std::sync::Arc;

use tachyon_sdk::auth::ExecutorAction;

use crate::domain::{
    DecideRecordPatchCommand, RecordActor, RecordActorKind, RecordPatch,
    RecordPropertyPatch, VersionedRecordMutationUnitOfWork,
};
use crate::usecase::{PatchRecordInputData, PatchRecordInputPort};

#[derive(Debug, Clone)]
pub struct PatchRecordInteractor {
    mutation: Arc<dyn VersionedRecordMutationUnitOfWork>,
}

impl PatchRecordInteractor {
    pub fn new(
        mutation: Arc<dyn VersionedRecordMutationUnitOfWork>,
    ) -> Arc<Self> {
        Arc::new(Self { mutation })
    }

    fn actor(executor: &dyn ExecutorAction) -> errors::Result<RecordActor> {
        if executor.is_none() || executor.get_id().is_empty() {
            return Err(errors::Error::not_found("resource not found"));
        }
        let kind = if executor.is_user() {
            RecordActorKind::User
        } else if executor.is_service_account() {
            RecordActorKind::ServiceAccount
        } else if executor.is_system_user() {
            RecordActorKind::System
        } else {
            return Err(errors::Error::not_found("resource not found"));
        };
        RecordActor::new(kind, executor.get_id())
    }
}

#[async_trait::async_trait]
impl PatchRecordInputPort for PatchRecordInteractor {
    async fn execute(
        &self,
        input: PatchRecordInputData<'_>,
    ) -> errors::Result<crate::domain::RecordMutationDecision> {
        if input.executor.is_none()
            || input.multi_tenancy.operator_id().as_ref()
                != Some(input.tenant_id)
            || !input.executor.has_tenant_id(input.tenant_id)
        {
            return Err(errors::Error::not_found("resource not found"));
        }

        let actor = Self::actor(input.executor)?;
        let name = input.name.map(str::parse).transpose()?;
        let properties = input
            .properties
            .into_iter()
            .map(|property| {
                RecordPropertyPatch::new(
                    &property.property_id,
                    property.value,
                )
            })
            .collect();
        let command = DecideRecordPatchCommand::new(
            input.tenant_id,
            input.database_id,
            input.data_id,
            input.operation_id,
            input.expected_version,
            actor,
            RecordPatch::new(name, properties),
        )?;
        self.mutation.decide_patch_atomically(&command).await
    }
}
