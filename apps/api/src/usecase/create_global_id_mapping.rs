//! PLT-942: Create a Library global_id mapping for an external system code.
//!
//! Tenant-scoped: the operator id (`x-operator-id` header) determines which
//! tenant owns the new row. Policy gate: `library:CreateGlobalIdMapping`.

use std::sync::Arc;

use tachyon_sdk::auth::{AuthApp, CheckPolicyInput};

use crate::domain::{GlobalIdMapping, GlobalIdMappingRepository};

use super::{
    CreateGlobalIdMappingInputData, CreateGlobalIdMappingInputPort,
};

#[derive(Debug, Clone)]
pub struct CreateGlobalIdMapping {
    repository: Arc<dyn GlobalIdMappingRepository>,
    auth: Arc<dyn AuthApp>,
}

impl CreateGlobalIdMapping {
    pub fn new(
        repository: Arc<dyn GlobalIdMappingRepository>,
        auth: Arc<dyn AuthApp>,
    ) -> Self {
        Self { repository, auth }
    }
}

#[async_trait::async_trait]
impl CreateGlobalIdMappingInputPort for CreateGlobalIdMapping {
    #[tracing::instrument(
        name = "CreateGlobalIdMapping::execute",
        skip(self)
    )]
    async fn execute<'a>(
        &self,
        input: CreateGlobalIdMappingInputData<'a>,
    ) -> errors::Result<GlobalIdMapping> {
        self.auth
            .check_policy(&CheckPolicyInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                action: "library:CreateGlobalIdMapping",
            })
            .await?;

        let tenant_id = input.multi_tenancy.get_operator_id()?;

        let mapping = GlobalIdMapping::create(
            &tenant_id,
            input.global_id,
            &input.system,
            &input.system_code,
            &input.name,
        );

        self.repository.insert(&mapping).await?;

        Ok(mapping)
    }
}
