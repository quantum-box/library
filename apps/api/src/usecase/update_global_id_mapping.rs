//! PLT-942: Update a `global_id_mapping`. Only `name` is mutable.
//! Policy gate: `library:UpdateGlobalIdMapping`.

use std::sync::Arc;

use tachyon_sdk::auth::{AuthApp, CheckPolicyInput};

use crate::domain::{GlobalIdMapping, GlobalIdMappingRepository};

use super::{
    UpdateGlobalIdMappingInputData, UpdateGlobalIdMappingInputPort,
};

#[derive(Debug, Clone)]
pub struct UpdateGlobalIdMapping {
    repository: Arc<dyn GlobalIdMappingRepository>,
    auth: Arc<dyn AuthApp>,
}

impl UpdateGlobalIdMapping {
    pub fn new(
        repository: Arc<dyn GlobalIdMappingRepository>,
        auth: Arc<dyn AuthApp>,
    ) -> Self {
        Self { repository, auth }
    }
}

#[async_trait::async_trait]
impl UpdateGlobalIdMappingInputPort for UpdateGlobalIdMapping {
    #[tracing::instrument(
        name = "UpdateGlobalIdMapping::execute",
        skip(self)
    )]
    async fn execute<'a>(
        &self,
        input: UpdateGlobalIdMappingInputData<'a>,
    ) -> errors::Result<GlobalIdMapping> {
        self.auth
            .check_policy(&CheckPolicyInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                action: "library:UpdateGlobalIdMapping",
            })
            .await?;

        let tenant_id = input.multi_tenancy.get_operator_id()?;

        self.repository
            .update_name(&tenant_id, &input.id, &input.name)
            .await?;

        self.repository
            .get_by_id(&tenant_id, &input.id)
            .await?
            .ok_or_else(|| {
                errors::Error::not_found("global_id_mapping not found")
            })
    }
}
