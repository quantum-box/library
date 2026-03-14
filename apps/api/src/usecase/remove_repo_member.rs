//! Remove a user's access from a repository.
//!
//! This usecase detaches all role-based policies from a user
//! for a specific repository.
//!
//! Safety checks:
//! - Cannot remove yourself
//! - Cannot remove the last owner

use std::sync::Arc;

use async_trait::async_trait;
use tachyon_sdk::auth::{
    AuthApp, CheckPolicyForResourceInput, DetachUserPolicyWithScopeInput,
};

use crate::usecase::{
    GetRepoMembersInputData, GetRepoMembersInputPort,
    RemoveRepoMemberInputData, RemoveRepoMemberInputPort,
};

/// Usecase for removing a user from a repository.
#[derive(Debug)]
pub struct RemoveRepoMember {
    auth_app: Arc<dyn AuthApp>,
    get_repo_members: Arc<dyn GetRepoMembersInputPort>,
}

impl RemoveRepoMember {
    /// Create a new RemoveRepoMember usecase instance.
    pub fn new(
        auth_app: Arc<dyn AuthApp>,
        get_repo_members: Arc<dyn GetRepoMembersInputPort>,
    ) -> Self {
        Self {
            auth_app,
            get_repo_members,
        }
    }
}

/// Policy IDs for library repo roles
const REPO_POLICY_IDS: [&str; 3] = [
    "pol_01libraryrepoowner",
    "pol_01libraryrepowriter",
    "pol_01libraryreporeader",
];

const OWNER_POLICY_ID: &str = "pol_01libraryrepoowner";
/// Org owners also have full access to all repos in the org
const ORG_OWNER_POLICY_ID: &str = "org_owner";

#[async_trait]
impl RemoveRepoMemberInputPort for RemoveRepoMember {
    async fn execute<'a>(
        &self,
        input: RemoveRepoMemberInputData<'a>,
    ) -> errors::Result<()> {
        // Generate TRN format resource identifier
        let resource_trn = format!("trn:library:repo:{}", input.repo_id);

        // Permission check: Only repo owners can remove members
        self.auth_app
            .check_policy_for_resource(&CheckPolicyForResourceInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                action: "library:RemoveRepoMember",
                resource_trn: &resource_trn,
            })
            .await?;

        let tenant_id = input.multi_tenancy.get_operator_id()?;
        let user_id: tachyon_sdk::auth::UserId = input.user_id.parse()?;

        // Safety check: Cannot remove yourself
        match input.executor.get_user_id() {
            Ok(executor_user_id) => {
                if executor_user_id == user_id {
                    return Err(errors::Error::bad_request(
                        "Cannot remove yourself from the repository",
                    ));
                }
            }
            Err(_) => {
                // If executor is not a user, skip this check
            }
        }

        // Safety check: Cannot remove the last owner
        let members = self
            .get_repo_members
            .execute(GetRepoMembersInputData {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                resource_trn: &resource_trn,
                tenant_id: &tenant_id,
            })
            .await?;

        // Count owners: both repo owners and org owners have owner privileges
        let owner_count = members
            .iter()
            .filter(|m| {
                m.policy_id == OWNER_POLICY_ID
                    || m.policy_id == ORG_OWNER_POLICY_ID
            })
            .count();

        // Check if target user is a repo owner (not org owner - org owners
        // cannot be removed via repo member removal)
        let is_target_owner = members.iter().any(|m| {
            m.user_id == user_id && m.policy_id == OWNER_POLICY_ID
        });

        if is_target_owner && owner_count <= 1 {
            return Err(errors::Error::bad_request(
                "Cannot remove the last owner. Transfer ownership first.",
            ));
        }

        // Detach all repo policies from the user for this repository
        for policy_id in REPO_POLICY_IDS {
            // Ignore errors for policies the user doesn't have
            let _ = self
                .auth_app
                .detach_user_policy_with_scope(
                    &DetachUserPolicyWithScopeInput {
                        executor: input.executor,
                        multi_tenancy: input.multi_tenancy,
                        user_id: &user_id,
                        policy_id: &tachyon_sdk::auth::PolicyId::new(
                            policy_id,
                        ),
                        tenant_id: &tenant_id,
                        resource_scope: &resource_trn,
                    },
                )
                .await;
        }

        Ok(())
    }
}
