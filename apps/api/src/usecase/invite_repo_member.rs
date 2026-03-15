//! Invite a user to a repository with a specific role.
//!
//! This usecase assigns a role-based policy (Owner/Writer/Reader) to a user
//! with resource scope limited to the specific repository TRN.
//!
//! The invited user is added to the tenant (operator) if not already a member,
//! but they only get access to the specific repository they were invited to.

use std::sync::Arc;

use async_trait::async_trait;
use tachyon_sdk::auth::UserQuery;
use tachyon_sdk::auth::{
    AddUserToTenantInput, AttachUserPolicyWithScopeInput, AuthApp,
    CheckPolicyForResourceInput,
};

use crate::usecase::{
    GetRepoMembersInputData, GetRepoMembersInputPort,
    InviteRepoMemberInputData, InviteRepoMemberInputPort,
};

const OWNER_POLICY_ID: &str = "pol_01libraryrepoowner";

/// Usecase for inviting a user to a repository.
#[derive(Debug)]
pub struct InviteRepoMember {
    auth_app: Arc<dyn AuthApp>,
    user_query: Arc<dyn UserQuery>,
    get_repo_members: Arc<dyn GetRepoMembersInputPort>,
}

impl InviteRepoMember {
    /// Create a new InviteRepoMember usecase instance.
    pub fn new(
        auth_app: Arc<dyn AuthApp>,
        user_query: Arc<dyn UserQuery>,
        get_repo_members: Arc<dyn GetRepoMembersInputPort>,
    ) -> Self {
        Self {
            auth_app,
            user_query,
            get_repo_members,
        }
    }
}

#[async_trait]
impl InviteRepoMemberInputPort for InviteRepoMember {
    async fn execute<'a>(
        &self,
        input: InviteRepoMemberInputData<'a>,
    ) -> errors::Result<()> {
        // Generate TRN format resource identifier
        let resource_trn = format!("trn:library:repo:{}", input.repo_id);

        // Permission check: Only repo owners can invite members
        self.auth_app
            .check_policy_for_resource(&CheckPolicyForResourceInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                action: "library:InviteRepoMember",
                resource_trn: &resource_trn,
            })
            .await?;

        // Find user by username or email
        let user = self.find_user(&input.username_or_email).await?;
        let user = user.ok_or_else(|| {
            errors::Error::not_found(format!(
                "User not found: {}",
                input.username_or_email
            ))
        })?;

        // Map role to policy ID
        let policy_id = match input.role.as_str() {
            "owner" => "pol_01libraryrepoowner",
            "writer" => "pol_01libraryrepowriter",
            "reader" => "pol_01libraryreporeader",
            other => {
                return Err(errors::Error::bad_request(format!(
                    "Unsupported role: {other}. Valid roles are: owner, writer, reader"
                )))
            }
        };

        let tenant_id = input.multi_tenancy.get_operator_id()?;

        // Safety check: Cannot add a new owner if one already exists
        // Repository must have exactly one owner
        if policy_id == OWNER_POLICY_ID {
            let members = self
                .get_repo_members
                .execute(GetRepoMembersInputData {
                    executor: input.executor,
                    multi_tenancy: input.multi_tenancy,
                    resource_trn: &resource_trn,
                    tenant_id: &tenant_id,
                })
                .await?;

            let has_owner =
                members.iter().any(|m| m.policy_id == OWNER_POLICY_ID);

            if has_owner {
                return Err(errors::Error::bad_request(
                    "Repository already has an owner. Only one owner is allowed per repository.",
                ));
            }
        }

        // Add user to tenant first (if not already a member)
        // This ensures the user can access the tenant, but without
        // any default policies - only the repository-specific policy below
        self.auth_app
            .add_user_to_tenant(&AddUserToTenantInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                user_id: user.id(),
                tenant_id: &tenant_id,
            })
            .await?;

        // Attach policy with resource scope (only for this repository)
        self.auth_app
            .attach_user_policy_with_scope(
                &AttachUserPolicyWithScopeInput {
                    executor: input.executor,
                    multi_tenancy: input.multi_tenancy,
                    user_id: user.id(),
                    policy_id: &tachyon_sdk::auth::PolicyId::new(policy_id),
                    tenant_id: &tenant_id,
                    resource_scope: &resource_trn,
                },
            )
            .await?;

        Ok(())
    }
}

impl InviteRepoMember {
    /// Find a user by username or email.
    ///
    /// First tries to find by username, then by email if not found.
    async fn find_user(
        &self,
        username_or_email: &str,
    ) -> errors::Result<Option<tachyon_sdk::auth::User>> {
        // Try to find by username first
        if let Ok(username) = username_or_email.parse() {
            if let Some(user) =
                self.user_query.get_by_username(&username).await?
            {
                return Ok(Some(user));
            }
        }

        // If input contains '@', try to find by email
        if username_or_email.contains('@') {
            // Note: get_by_email requires tenant_id, but we want to search globally
            // For now, we only support username-based search for cross-tenant invites
            // Email search would need tenant context
            tracing::debug!(
                "Email-based search not supported for cross-tenant invites"
            );
        }

        Ok(None)
    }
}
