//! Change a user's role in an organization with
//! library-specific policy updates.
//!
//! This usecase updates the user's DefaultRole and:
//! - If the new role is Owner, attaches repo owner policy
//! - If downgrading from Owner, detaches repo owner policy

use std::sync::Arc;

use async_trait::async_trait;
use derive_new::new;
use tachyon_sdk::auth::{
    AuthApp, CheckPolicyInput, DefaultRole, DetachUserPolicyInput,
    ExecutorAction, MultiTenancyAction, User,
};
use tracing::info;
use value_object::{OperatorId, TenantId, UserId};

use crate::domain::{
    library_api_key_issuer_policy_id, library_repo_owner_policy_id, OrgRole,
};
use crate::sdk_auth::SdkAuthApp;

/// Input data for changing a user's role in an organization
#[derive(Debug)]
pub struct ChangeOrgMemberRoleInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    pub tenant_id: &'a OperatorId,
    pub target_user_id: &'a UserId,
    /// New role to assign (Owner, Manager, General)
    pub new_role: OrgRole,
}

/// Output data for changing a user's role
#[derive(Debug)]
pub struct ChangeOrgMemberRoleOutputData {
    pub user: User,
}

#[async_trait]
pub trait ChangeOrgMemberRoleInputPort:
    std::fmt::Debug + Send + Sync
{
    async fn execute<'a>(
        &self,
        input: ChangeOrgMemberRoleInputData<'a>,
    ) -> errors::Result<ChangeOrgMemberRoleOutputData>;
}

#[derive(Debug, Clone, new)]
pub struct ChangeOrgMemberRole {
    sdk: Arc<SdkAuthApp>,
}

#[async_trait]
impl ChangeOrgMemberRoleInputPort for ChangeOrgMemberRole {
    #[tracing::instrument(
        name = "ChangeOrgMemberRole::execute",
        skip(self)
    )]
    async fn execute<'a>(
        &self,
        input: ChangeOrgMemberRoleInputData<'a>,
    ) -> errors::Result<ChangeOrgMemberRoleOutputData> {
        // Policy check
        AuthApp::check_policy(
            self.sdk.as_ref(),
            &CheckPolicyInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                action: "library:ChangeOrgMemberRole",
            },
        )
        .await?;

        // 1. Get the target user via REST
        let tenant = TenantId::new(input.tenant_id.as_ref())?;
        let user = self
            .sdk
            .get_user_by_id_full(&tenant, input.target_user_id.as_ref())
            .await?
            .ok_or_else(|| {
                errors::not_found!("User not found in tenant")
            })?;

        // Convert auth role to library role for comparison
        let old_role: OrgRole = (*user.role()).into();
        let new_role = input.new_role;

        // 2. Take back what the old role carried, before the role
        //    itself changes. A refusal here leaves the member an owner,
        //    which the same request repeats; taking the role first and
        //    failing here would leave a member holding an owner's
        //    tachyon grants, with no downgrade left to retry.
        let downgraded_from_owner =
            old_role == OrgRole::Owner && new_role != OrgRole::Owner;
        if downgraded_from_owner {
            self.detach_repo_owner_policy(
                input.executor,
                input.multi_tenancy,
                &user,
                &tenant,
            )
            .await?;
            self.detach_api_key_issuer_policy(
                input.executor,
                input.multi_tenancy,
                &user,
                &tenant,
            )
            .await?;
        }

        // 3. Update user's role via REST
        let auth_role: DefaultRole = new_role.into();
        let updated_user = self
            .sdk
            .update_user_role(
                input.executor,
                input.multi_tenancy,
                input.target_user_id.as_ref(),
                &tenant,
                &auth_role.to_string(),
            )
            .await?;

        info!(
            user = %input.target_user_id,
            tenant = %input.tenant_id,
            old_role = ?old_role,
            new_role = ?new_role,
            "updated user role in organization"
        );

        // 4. An owner holds the repo owner policy, whether or not this
        //    request is what made them one. The attachment is idempotent
        //    upstream, and asking for it every time is what repairs a
        //    downgrade that stopped halfway: setting the role back to
        //    Owner returns the grant the failed attempt took. The other
        //    direction was handled before the role changed, and repeats
        //    the same way.
        if new_role == OrgRole::Owner {
            self.attach_repo_owner_policy(
                input.executor,
                input.multi_tenancy,
                &updated_user,
                &tenant,
            )
            .await?;
        }

        Ok(ChangeOrgMemberRoleOutputData { user: updated_user })
    }
}

impl ChangeOrgMemberRole {
    /// Attach repo owner policy for full repository access.
    async fn attach_repo_owner_policy(
        &self,
        executor: &dyn ExecutorAction,
        multi_tenancy: &dyn MultiTenancyAction,
        user: &User,
        tenant_id: &TenantId,
    ) -> errors::Result<()> {
        let policy_id = library_repo_owner_policy_id();

        AuthApp::attach_user_policy(
            self.sdk.as_ref(),
            &tachyon_sdk::auth::AttachUserPolicyInput {
                executor,
                multi_tenancy,
                user_id: user.id(),
                policy_id: &policy_id,
                tenant_id,
            },
        )
        .await?;

        info!(
            user = %user.id(),
            tenant = %tenant_id,
            "attached repo owner policy during role upgrade"
        );

        Ok(())
    }

    /// Detach the API key issuer policy when downgrading from Owner.
    ///
    /// Issuing a key with repository access grants it on use (see
    /// `grant_api_key_policy`), and it outlives the role that earned it:
    /// left attached, a former owner could still attach policies to the
    /// organization's service accounts and delete them, straight through
    /// tachyon, which Library's own checks would now refuse.
    async fn detach_api_key_issuer_policy(
        &self,
        executor: &dyn ExecutorAction,
        multi_tenancy: &dyn MultiTenancyAction,
        user: &User,
        tenant_id: &TenantId,
    ) -> errors::Result<()> {
        let Some(policy_id) = library_api_key_issuer_policy_id() else {
            return Ok(());
        };

        AuthApp::detach_user_policy(
            self.sdk.as_ref(),
            &DetachUserPolicyInput {
                executor,
                multi_tenancy,
                user_id: user.id(),
                policy_id: &policy_id,
                tenant_id,
            },
        )
        .await?;

        info!(
            user = %user.id(),
            tenant = %tenant_id,
            "detached api key issuer policy during role downgrade"
        );

        Ok(())
    }

    /// Detach repo owner policy when downgrading from Owner.
    async fn detach_repo_owner_policy(
        &self,
        executor: &dyn ExecutorAction,
        multi_tenancy: &dyn MultiTenancyAction,
        user: &User,
        tenant_id: &TenantId,
    ) -> errors::Result<()> {
        let policy_id = library_repo_owner_policy_id();

        AuthApp::detach_user_policy(
            self.sdk.as_ref(),
            &DetachUserPolicyInput {
                executor,
                multi_tenancy,
                user_id: user.id(),
                policy_id: &policy_id,
                tenant_id,
            },
        )
        .await?;

        info!(
            user = %user.id(),
            tenant = %tenant_id,
            "detached repo owner policy during role downgrade"
        );

        Ok(())
    }
}
