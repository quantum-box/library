//! PLT-942: Create a Library global_id mapping for an external system code.
//!
//! Tenant-scoped: the operator id (`x-operator-id` header) determines which
//! tenant owns the new row.
//!
//! NOTE (M2 / PLT-942): policy enforcement (`library:CreateGlobalIdMapping`)
//! is intentionally deferred to M3 — the action is being added by
//! leader-plt942-action in tachyon IaC and will be wired in once that PR
//! merges (see Linear PLT-942).

use std::sync::Arc;

use tachyon_sdk::auth::AuthApp;

use crate::domain::{GlobalIdMapping, GlobalIdMappingRepository};

use super::{
    CreateGlobalIdMappingInputData, CreateGlobalIdMappingInputPort,
};

#[derive(Debug, Clone)]
pub struct CreateGlobalIdMapping {
    repository: Arc<dyn GlobalIdMappingRepository>,
    #[allow(dead_code)]
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
        // TODO PLT-942 M3: self.auth.check_policy("library:CreateGlobalIdMapping")
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
