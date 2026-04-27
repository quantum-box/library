//! PLT-942: Look up a `global_id_mapping` by `(system, system_code)`.
//! Policy gate: `library:ReadGlobalIdMapping` (bakuure SA Phase 1 read-only).

use std::sync::Arc;

use tachyon_sdk::auth::{AuthApp, CheckPolicyInput};

use crate::domain::{GlobalIdMapping, GlobalIdMappingRepository};

use super::{GetGlobalIdMappingInputData, GetGlobalIdMappingInputPort};

#[derive(Debug, Clone)]
pub struct GetGlobalIdMapping {
    repository: Arc<dyn GlobalIdMappingRepository>,
    auth: Arc<dyn AuthApp>,
}

impl GetGlobalIdMapping {
    pub fn new(
        repository: Arc<dyn GlobalIdMappingRepository>,
        auth: Arc<dyn AuthApp>,
    ) -> Self {
        Self { repository, auth }
    }
}

#[async_trait::async_trait]
impl GetGlobalIdMappingInputPort for GetGlobalIdMapping {
    #[tracing::instrument(name = "GetGlobalIdMapping::execute", skip(self))]
    async fn execute<'a>(
        &self,
        input: GetGlobalIdMappingInputData<'a>,
    ) -> errors::Result<Option<GlobalIdMapping>> {
        self.auth
            .check_policy(&CheckPolicyInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                action: "library:ReadGlobalIdMapping",
            })
            .await?;

        let tenant_id = input.multi_tenancy.get_operator_id()?;

        self.repository
            .find_by_system_code(
                &tenant_id,
                &input.system,
                &input.system_code,
            )
            .await
    }
}
