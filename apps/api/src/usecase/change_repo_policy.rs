use std::sync::Arc;

use async_trait::async_trait;
use tachyon_sdk::auth::{
    AttachUserPolicyWithScopeInput, AuthApp, CheckPolicyForResourceInput,
};

use crate::usecase::{
    ChangeRepoPolicyInputData, ChangeRepoPolicyInputPort,
};

#[derive(Debug)]
pub struct ChangeRepoPolicy {
    auth_app: Arc<dyn AuthApp>,
}

impl ChangeRepoPolicy {
    pub fn new(auth_app: Arc<dyn AuthApp>) -> Self {
        Self { auth_app }
    }
}

#[async_trait]
impl ChangeRepoPolicyInputPort for ChangeRepoPolicy {
    async fn execute<'a>(
        &self,
        input: ChangeRepoPolicyInputData<'a>,
    ) -> errors::Result<()> {
        // Generate TRN format resource identifier
        let resource_trn = format!("trn:library:repo:{}", input.repo_id);

        // Permission check: Repo policy operation (with resource scope)
        self.auth_app
            .check_policy_for_resource(&CheckPolicyForResourceInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                action: "library:ManageRepoPolicy",
                resource_trn: &resource_trn,
            })
            .await?;

        // TODO: add English comment
        let policy_id = match input.role.as_str() {
            "owner" => "pol_01libraryrepoowner",
            "writer" => "pol_01libraryrepowriter",
            "reader" => "pol_01libraryreporeader",
            other => {
                return Err(errors::Error::business_logic(format!(
                    "unsupported role: {other}"
                )))
            }
        };

        let tenant_id = input.multi_tenancy.get_operator_id()?;

        // Attach policy with resource scope
        self.auth_app
            .attach_user_policy_with_scope(
                &AttachUserPolicyWithScopeInput {
                    executor: input.executor,
                    multi_tenancy: input.multi_tenancy,
                    user_id: &input.target_user_id.parse()?,
                    policy_id: &tachyon_sdk::auth::PolicyId::new(
                        policy_id,
                    ),
                    tenant_id: &tenant_id,
                    resource_scope: &resource_trn,
                },
            )
            .await?;

        Ok(())
    }
}
