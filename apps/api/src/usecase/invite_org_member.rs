//! Invite a user to an organization with library-specific
//! policy setup.
//!
//! This usecase wraps auth's InviteUser and additionally:
//! - Attaches LibraryUserPolicy to the invited user
//! - If the invited user becomes org owner, attaches repo
//!   owner policy

use std::sync::Arc;

use async_trait::async_trait;
use derive_new::new;
use tachyon_sdk::auth::{
    AuthApp, CheckPolicyInput, DefaultRole, ExecutorAction,
    MultiTenancyAction, User,
};
use tracing::info;
use value_object::{OperatorId, PlatformId, TenantId};

use crate::domain::{
    library_repo_owner_policy_id, library_user_policy_id, OrgRole,
    LIBRARY_TENANT,
};
use crate::sdk_auth::{InviteUserReq, SdkAuthApp};

/// Input data for inviting a user to an organization
#[derive(Debug)]
pub struct InviteOrgMemberInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    pub platform_id: Option<&'a PlatformId>,
    pub tenant_id: &'a OperatorId,
    pub invitee: value_object::IdOrEmail<tachyon_sdk::auth::UserId>,
    pub notify_user: Option<bool>,
    /// Role to assign to the invited user
    pub role: Option<OrgRole>,
}

/// Output data for inviting a user to an organization
#[derive(Debug)]
pub struct InviteOrgMemberOutputData {
    pub user: User,
}

#[async_trait]
pub trait InviteOrgMemberInputPort: std::fmt::Debug + Send + Sync {
    async fn execute<'a>(
        &self,
        input: InviteOrgMemberInputData<'a>,
    ) -> errors::Result<InviteOrgMemberOutputData>;
}

#[derive(Debug, Clone, new)]
pub struct InviteOrgMember {
    sdk: Arc<SdkAuthApp>,
}

#[async_trait]
impl InviteOrgMemberInputPort for InviteOrgMember {
    #[tracing::instrument(name = "InviteOrgMember::execute", skip(self))]
    async fn execute<'a>(
        &self,
        input: InviteOrgMemberInputData<'a>,
    ) -> errors::Result<InviteOrgMemberOutputData> {
        // Policy check
        AuthApp::check_policy(
            self.sdk.as_ref(),
            &CheckPolicyInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                action: "library:InviteOrgMember",
            },
        )
        .await?;

        // Build invite request
        let (invitee_id, invitee_email) = match &input.invitee {
            value_object::IdOrEmail::Id(id) => (Some(id.to_string()), None),
            value_object::IdOrEmail::Email(email) => {
                (None, Some(email.to_string()))
            }
        };

        // 1. Call invite_user via REST
        let user = self
            .sdk
            .invite_user_rest(
                input.executor,
                input.multi_tenancy,
                &InviteUserReq {
                    platform_id: input.platform_id.map(|p| p.to_string()),
                    tenant_id: input.tenant_id.to_string(),
                    invitee_id,
                    invitee_email,
                    notify_user: input.notify_user,
                },
            )
            .await?;

        // 2. If role is specified, update user's role
        if let Some(role) = input.role {
            let auth_role: DefaultRole = role.into();
            let auth_role_str = auth_role.to_string();
            let tenant_id = TenantId::new(input.tenant_id.as_ref())?;
            self.sdk
                .update_user_role(
                    input.executor,
                    input.multi_tenancy,
                    &user.id().to_string(),
                    &tenant_id,
                    &auth_role_str,
                )
                .await?;

            info!(
                user = %user.id(),
                tenant = %input.tenant_id,
                role = ?role,
                "updated user role during org member invite"
            );
        }

        // 3. Attach LibraryUserPolicy
        let tenant_id = TenantId::new(input.tenant_id.as_ref())?;
        self.attach_library_policy(
            input.executor,
            input.multi_tenancy,
            &user,
            &tenant_id,
        )
        .await?;

        // 4. If user is org owner, attach repo owner policy
        let final_role = input.role.unwrap_or(OrgRole::General);
        if final_role == OrgRole::Owner {
            self.attach_repo_owner_policy(
                input.executor,
                input.multi_tenancy,
                &user,
                &tenant_id,
            )
            .await?;
        }

        Ok(InviteOrgMemberOutputData { user })
    }
}

impl InviteOrgMember {
    /// Attach LibraryUserPolicy to the user.
    async fn attach_library_policy(
        &self,
        executor: &dyn ExecutorAction,
        multi_tenancy: &dyn MultiTenancyAction,
        user: &User,
        tenant_id: &TenantId,
    ) -> errors::Result<()> {
        let policy_id = library_user_policy_id();

        // Attach to platform scope
        let platform_tenant = LIBRARY_TENANT.clone();
        AuthApp::attach_user_policy(
            self.sdk.as_ref(),
            &tachyon_sdk::auth::AttachUserPolicyInput {
                executor,
                multi_tenancy,
                user_id: user.id(),
                policy_id: &policy_id,
                tenant_id: &platform_tenant,
            },
        )
        .await?;

        // Attach to tenant scope
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
            "attached library policy during org member invite"
        );

        Ok(())
    }

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
            "attached repo owner policy for org owner"
        );

        Ok(())
    }
}
