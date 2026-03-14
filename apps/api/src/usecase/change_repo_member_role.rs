//! Change a user's role in a repository.
//!
//! This usecase changes the role-based policy for a user
//! in a specific repository (Owner/Writer/Reader).
//!
//! Safety checks:
//! - Cannot demote the last owner

use std::sync::Arc;

use async_trait::async_trait;
use tachyon_sdk::auth::{
    AttachUserPolicyWithScopeInput, AuthApp, CheckPolicyForResourceInput,
    DetachUserPolicyWithScopeInput,
};

use crate::usecase::{
    ChangeRepoMemberRoleInputData, ChangeRepoMemberRoleInputPort,
    GetRepoMembersInputData, GetRepoMembersInputPort,
};

/// Usecase for changing a user's role in a repository.
#[derive(Debug)]
pub struct ChangeRepoMemberRole {
    auth_app: Arc<dyn AuthApp>,
    get_repo_members: Arc<dyn GetRepoMembersInputPort>,
}

impl ChangeRepoMemberRole {
    /// Create a new ChangeRepoMemberRole usecase instance.
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

#[async_trait]
impl ChangeRepoMemberRoleInputPort for ChangeRepoMemberRole {
    async fn execute<'a>(
        &self,
        input: ChangeRepoMemberRoleInputData<'a>,
    ) -> errors::Result<()> {
        // Generate TRN format resource identifier
        let resource_trn = format!("trn:library:repo:{}", input.repo_id);

        // Permission check: Only repo owners can change member roles
        self.auth_app
            .check_policy_for_resource(&CheckPolicyForResourceInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                action: "library:ChangeRepoMemberRole",
                resource_trn: &resource_trn,
            })
            .await?;

        // Map new role to policy ID
        let new_policy_id = match input.new_role.as_str() {
            "owner" => "pol_01libraryrepoowner",
            "writer" => "pol_01libraryrepowriter",
            "reader" => "pol_01libraryreporeader",
            other => {
                return Err(errors::Error::bad_request(format!(
                    "Unsupported role: {other}. Valid roles: owner, writer, reader"
                )))
            }
        };

        let tenant_id = input.multi_tenancy.get_operator_id()?;
        let user_id: tachyon_sdk::auth::UserId = input.user_id.parse()?;

        // Get current members to check constraints
        let members = self
            .get_repo_members
            .execute(GetRepoMembersInputData {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                resource_trn: &resource_trn,
                tenant_id: &tenant_id,
            })
            .await?;

        // Check if there is already an owner
        let has_owner =
            members.iter().any(|m| m.policy_id == OWNER_POLICY_ID);

        // Safety check: Cannot change owner's role
        // To transfer ownership, add another owner first, then remove the original owner
        let is_target_owner = members.iter().any(|m| {
            m.user_id == user_id && m.policy_id == OWNER_POLICY_ID
        });

        if is_target_owner {
            return Err(errors::Error::bad_request(
                "Cannot change owner's role. To transfer ownership, add another owner first.",
            ));
        }

        // Safety check: Cannot add a new owner if one already exists
        // Repository must have exactly one owner
        if new_policy_id == OWNER_POLICY_ID && has_owner {
            return Err(errors::Error::bad_request(
                "Repository already has an owner. Only one owner is allowed per repository.",
            ));
        }

        // Detach all existing repo policies from the user for this repository
        for policy_id in REPO_POLICY_IDS {
            let _ = self
                .auth_app
                .detach_user_policy_with_scope(
                    &DetachUserPolicyWithScopeInput {
                        executor: input.executor,
                        multi_tenancy: input.multi_tenancy,
                        user_id: &user_id,
                        policy_id: &tachyon_sdk::auth::PolicyId::new(policy_id),
                        tenant_id: &tenant_id,
                        resource_scope: &resource_trn,
                    },
                )
                .await;
        }

        // Attach the new role policy
        self.auth_app
            .attach_user_policy_with_scope(
                &AttachUserPolicyWithScopeInput {
                    executor: input.executor,
                    multi_tenancy: input.multi_tenancy,
                    user_id: &user_id,
                    policy_id: &tachyon_sdk::auth::PolicyId::new(new_policy_id),
                    tenant_id: &tenant_id,
                    resource_scope: &resource_trn,
                },
            )
            .await?;

        Ok(())
    }
}
