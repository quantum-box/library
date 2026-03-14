use std::sync::Arc;

use async_trait::async_trait;
use derive_new::new;
use errors::Result;
use tachyon_sdk::auth::{
    AuthApp, CheckPolicyForResourceInput, CheckPolicyInput,
};

use crate::domain::VisibilityService;

use super::{
    GetOrganizationByUsernameQuery, GetRepoByUsernameQuery,
    ViewRepoInputData, ViewRepoInputPort, ViewRepoOutputData,
};

#[derive(Debug, Clone, new)]
pub struct ViewRepo {
    auth_app: Arc<dyn AuthApp>,
    get_org_by_username: Arc<dyn GetOrganizationByUsernameQuery>,
    get_repo_by_username: Arc<dyn GetRepoByUsernameQuery>,
    visibility_service: Arc<VisibilityService>,
}

#[async_trait]
impl ViewRepoInputPort for ViewRepo {
    #[tracing::instrument(name = "ViewRepo::execute", skip(self))]
    async fn execute<'a>(
        &self,
        input: &ViewRepoInputData<'a>,
    ) -> Result<ViewRepoOutputData> {
        let org = self
            .get_org_by_username
            .execute(&input.organization_username.parse()?)
            .await?
            .ok_or(errors::not_found!("organization not found"))?;
        let repo = self
            .get_repo_by_username
            .execute(org.username(), &input.repo_username.parse()?)
            .await?
            .ok_or(errors::not_found!("repo not found"))?;

        // Check visibility: public repos are always accessible
        // For private repos, need login + tenant membership
        let need_policy_check = self
            .visibility_service
            .check_access(&repo, input.executor)?;

        if input.executor.is_service_account()
            && !input.executor.has_tenant_id(org.id())
        {
            return Err(errors::permission_denied!(
                "API key can only access repositories in the same organization"
            ));
        }

        // For private repos, check access with resource-based policy
        if need_policy_check && !input.executor.is_none() {
            let resource_trn = format!("trn:library:repo:{}", repo.id());

            // Try resource-based access first (for invited members)
            let resource_access = self
                .auth_app
                .check_policy_for_resource(&CheckPolicyForResourceInput {
                    executor: input.executor,
                    multi_tenancy: input.multi_tenancy,
                    action: "library:ViewRepo",
                    resource_trn: &resource_trn,
                })
                .await;

            if resource_access.is_err() {
                // Fallback to standard policy (for org-wide access)
                self.auth_app
                    .check_policy(&CheckPolicyInput {
                        executor: input.executor,
                        multi_tenancy: input.multi_tenancy,
                        action: "library:ViewPrivateRepo",
                    })
                    .await?;
            }
        }

        Ok(ViewRepoOutputData { repo })
    }
}
