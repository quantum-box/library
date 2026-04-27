//! PLT-942: Look up a `global_id_mapping` by `(system, system_code)`.
//!
//! NOTE (M2): policy enforcement (`library:ReadGlobalIdMapping`) deferred to
//! M3 (see PLT-942 / leader-plt942-action).

use std::sync::Arc;

use tachyon_sdk::auth::AuthApp;

use crate::domain::{GlobalIdMapping, GlobalIdMappingRepository};

use super::{GetGlobalIdMappingInputData, GetGlobalIdMappingInputPort};

#[derive(Debug, Clone)]
pub struct GetGlobalIdMapping {
    repository: Arc<dyn GlobalIdMappingRepository>,
    #[allow(dead_code)]
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
    #[tracing::instrument(
        name = "GetGlobalIdMapping::execute",
        skip(self)
    )]
    async fn execute<'a>(
        &self,
        input: GetGlobalIdMappingInputData<'a>,
    ) -> errors::Result<Option<GlobalIdMapping>> {
        // TODO PLT-942 M3: self.auth.check_policy("library:ReadGlobalIdMapping")
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
