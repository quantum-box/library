//! PLT-942: Update a `global_id_mapping`. Only `name` is mutable.
//!
//! NOTE (M2): policy enforcement (`library:UpdateGlobalIdMapping`) deferred
//! to M3 (see PLT-942 / leader-plt942-action).

use std::sync::Arc;

use tachyon_sdk::auth::AuthApp;

use crate::domain::{GlobalIdMapping, GlobalIdMappingRepository};

use super::{
    UpdateGlobalIdMappingInputData, UpdateGlobalIdMappingInputPort,
};

#[derive(Debug, Clone)]
pub struct UpdateGlobalIdMapping {
    repository: Arc<dyn GlobalIdMappingRepository>,
    #[allow(dead_code)]
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
        // TODO PLT-942 M3: self.auth.check_policy("library:UpdateGlobalIdMapping")
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
