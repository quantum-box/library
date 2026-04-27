//! PLT-942: List `global_id_mapping` rows for the caller's tenant.
//!
//! NOTE (M2): policy enforcement (`library:ReadGlobalIdMapping`) deferred to
//! M3 (see PLT-942 / leader-plt942-action).

use std::sync::Arc;

use tachyon_sdk::auth::AuthApp;

use crate::domain::{GlobalIdMapping, GlobalIdMappingRepository};

use super::{
    FindGlobalIdMappingsInputData, FindGlobalIdMappingsInputPort,
};

#[derive(Debug, Clone)]
pub struct FindGlobalIdMappings {
    repository: Arc<dyn GlobalIdMappingRepository>,
    #[allow(dead_code)]
    auth: Arc<dyn AuthApp>,
}

impl FindGlobalIdMappings {
    pub fn new(
        repository: Arc<dyn GlobalIdMappingRepository>,
        auth: Arc<dyn AuthApp>,
    ) -> Self {
        Self { repository, auth }
    }
}

#[async_trait::async_trait]
impl FindGlobalIdMappingsInputPort for FindGlobalIdMappings {
    #[tracing::instrument(
        name = "FindGlobalIdMappings::execute",
        skip(self)
    )]
    async fn execute<'a>(
        &self,
        input: FindGlobalIdMappingsInputData<'a>,
    ) -> errors::Result<Vec<GlobalIdMapping>> {
        // TODO PLT-942 M3: self.auth.check_policy("library:ReadGlobalIdMapping")
        let tenant_id = input.multi_tenancy.get_operator_id()?;

        self.repository
            .find_all(&tenant_id, input.system.as_deref())
            .await
    }
}
