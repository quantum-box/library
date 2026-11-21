//! Invite a user to an organization with library-specific
//! policy setup.
//!
//! This usecase wraps auth's InviteUser. The Library policies the
//! invitee needs in the organization's tenant ride along with the
//! invitation (`policyIds`), so tachyon grants them itself: at once for
//! an existing account, and on acceptance for an email that has no
//! account yet, which Library has no user to attach anything to.
//!
//! - LibraryUserPolicy, always.
//! - The repo owner policy, when the invitee is to be an owner.
//!
//! An existing account additionally gets LibraryUserPolicy in Library's
//! platform tenant, as sign-in does.

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
use crate::sdk_auth::{InviteUserOutcome, InviteUserReq, SdkAuthApp};

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
    /// The member, or `None` when the email had no account and an
    /// invitation was sent instead.
    pub user: Option<User>,
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

/// The policies granted in the organization's tenant with an invitation
/// as `role`.
fn invitation_policy_ids(role: OrgRole) -> Vec<String> {
    let mut policy_ids = vec![library_user_policy_id().to_string()];
    if role == OrgRole::Owner {
        policy_ids.push(library_repo_owner_policy_id().to_string());
    }
    policy_ids
}

#[async_trait]
impl InviteOrgMemberInputPort for InviteOrgMember {
    #[tracing::instrument(name = "InviteOrgMember::execute", skip(self))]
    async fn execute<'a>(
        &self,
        input: InviteOrgMemberInputData<'a>,
    ) -> errors::Result<InviteOrgMemberOutputData> {
        // Policy check, in the organization's own tenant. The invitation
        // is sent from there too: tachyon grants `policyIds` in the
        // tenant the request is scoped to.
        let scope = crate::usecase::authorization_scope_for_org(
            input.multi_tenancy,
            input.tenant_id,
        )?;
        AuthApp::check_policy(
            self.sdk.as_ref(),
            &CheckPolicyInput {
                executor: input.executor,
                multi_tenancy: &scope,
                action: "library:InviteOrgMember",
            },
        )
        .await?;

        let (invitee_id, invitee_email) = match &input.invitee {
            value_object::IdOrEmail::Id(id) => (Some(id.to_string()), None),
            value_object::IdOrEmail::Email(email) => {
                (None, Some(email.to_string()))
            }
        };
        let role = input.role.unwrap_or(OrgRole::General);

        let outcome = self
            .sdk
            .invite_user_rest(
                input.executor,
                &scope,
                &InviteUserReq {
                    platform_id: input.platform_id.map(|p| p.to_string()),
                    tenant_id: input.tenant_id.to_string(),
                    invitee_id,
                    invitee_email,
                    notify_user: input.notify_user,
                    policy_ids: invitation_policy_ids(role),
                },
            )
            .await?;

        let mut user = match outcome {
            InviteUserOutcome::TenantAccessGranted(user) => *user,
            InviteUserOutcome::InvitationSent { email } => {
                info!(
                    tenant = %input.tenant_id,
                    email = ?email,
                    role = ?role,
                    "sent an organization invitation to an email without an account"
                );
                return Ok(InviteOrgMemberOutputData { user: None });
            }
        };

        self.attach_platform_library_policy(input.executor, &user)
            .await?;

        info!(
            user = %user.id(),
            tenant = %input.tenant_id,
            role = ?role,
            "invited organization member"
        );

        // The label is tachyon's and is not changed; the role that was
        // applied is what the policies now say.
        if input.role.is_some() {
            user.role = DefaultRole::from(role);
        }
        Ok(InviteOrgMemberOutputData { user: Some(user) })
    }
}

impl InviteOrgMember {
    /// Attach LibraryUserPolicy in Library's platform tenant, where
    /// sign-in attaches it too. tachyon grants in the tenant the request
    /// is scoped to, so the scope names the platform tenant.
    async fn attach_platform_library_policy(
        &self,
        executor: &dyn ExecutorAction,
        user: &User,
    ) -> errors::Result<()> {
        let platform_tenant = TenantId::new(LIBRARY_TENANT.as_ref())?;
        let platform_scope = tachyon_sdk::auth::MultiTenancy::new(
            Some(platform_tenant.clone()),
            Some(platform_tenant.clone()),
        );
        AuthApp::attach_user_policy(
            self.sdk.as_ref(),
            &tachyon_sdk::auth::AttachUserPolicyInput {
                executor,
                multi_tenancy: &platform_scope,
                user_id: user.id(),
                policy_id: &library_user_policy_id(),
                tenant_id: &platform_tenant,
            },
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_owner_invitation_carries_the_repo_owner_policy() {
        assert_eq!(
            invitation_policy_ids(OrgRole::Owner),
            vec![
                library_user_policy_id().to_string(),
                library_repo_owner_policy_id().to_string(),
            ]
        );
    }

    #[test]
    fn other_invitations_carry_only_the_library_user_policy() {
        for role in [OrgRole::Manager, OrgRole::General] {
            assert_eq!(
                invitation_policy_ids(role),
                vec![library_user_policy_id().to_string()]
            );
        }
    }
}
