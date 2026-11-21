//! Change a member's role in an organization.
//!
//! tachyon no longer has a way to set the `role` label on a user
//! (`PUT /v1/auth/users/:user_id/role` was removed upstream: roles were
//! folded into policies, and the label is one column shared by every
//! tenant the user belongs to). What makes someone an owner of an
//! organization in Library is the policies attached to them in its
//! tenant, so that is what this usecase changes:
//!
//! - Owner: the unscoped repo owner policy is attached.
//! - Manager / General: the repo owner policy and the API key policies
//!   an owner picks up on use are detached, whatever the member was
//!   before. Detaching what is not attached is a no-op upstream, so the
//!   revocation covers every way the grant can have arrived (a seed of
//!   a tenant administrator, an earlier promotion, an invite as Owner),
//!   not only the ones the member's label still remembers.
//!
//! The member's label is left as tachyon has it. The returned user
//! carries the role that was applied, which is what the caller asked
//! for and what the policies now say.

use std::sync::Arc;

use async_trait::async_trait;
use derive_new::new;
use tachyon_sdk::auth::{
    AuthApp, CheckPolicyInput, DefaultRole, DetachUserPolicyInput,
    ExecutorAction, GetUserByIdInput, MultiTenancyAction, PolicyId, User,
};
use tracing::info;
use value_object::{OperatorId, TenantId, UserId};

use crate::domain::{
    library_api_key_accounts_policy_id, library_api_key_issuer_policy_id,
    library_repo_owner_policy_id, OrgRole,
};

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
    auth_app: Arc<dyn AuthApp>,
}

/// The policies an owner of the organization holds in its tenant and a
/// member who is not one must not.
///
/// The repo owner policy comes first: it is the grant that matters, so a
/// failure on a later one still leaves the member without owner rights
/// over the repositories, and repeating the request finishes the rest.
fn owner_only_policies() -> [PolicyId; 3] {
    [
        library_repo_owner_policy_id(),
        library_api_key_issuer_policy_id(),
        library_api_key_accounts_policy_id(),
    ]
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
        // Everything below runs in the organization's own tenant. tachyon
        // attaches and detaches in the tenant named by `x-operator-id`,
        // not the one in the request body, so a header naming the
        // platform tenant would otherwise grant an unscoped repo owner
        // policy there and leave the organization's own grant in place.
        let scope = crate::usecase::authorization_scope_for_org(
            input.multi_tenancy,
            input.tenant_id,
        )?;
        self.auth_app
            .check_policy(&CheckPolicyInput {
                executor: input.executor,
                multi_tenancy: &scope,
                action: "library:ChangeOrgMemberRole",
            })
            .await?;

        let tenant = TenantId::new(input.tenant_id.as_ref())?;
        let mut user = self
            .auth_app
            .get_user_by_id(&GetUserByIdInput {
                executor: input.executor,
                multi_tenancy: &scope,
                user_id: input.target_user_id,
            })
            .await?
            .ok_or_else(|| {
                errors::not_found!("User not found in tenant")
            })?;
        // A policy attached to someone outside the tenant still acts in
        // it upstream, so promoting a non-member would make them an
        // owner of the organization without ever joining it.
        if !user.tenants.contains(&tenant) {
            return Err(errors::not_found!("User not found in tenant"));
        }

        let new_role = input.new_role;
        if new_role == OrgRole::Owner {
            // Attaching is idempotent upstream.
            self.auth_app
                .attach_user_policy(
                    &tachyon_sdk::auth::AttachUserPolicyInput {
                        executor: input.executor,
                        multi_tenancy: &scope,
                        user_id: user.id(),
                        policy_id: &library_repo_owner_policy_id(),
                        tenant_id: &tenant,
                    },
                )
                .await?;
            info!(
                user = %user.id(),
                tenant = %tenant,
                "attached repo owner policy for organization owner"
            );
        } else {
            for policy_id in owner_only_policies() {
                self.auth_app
                    .detach_user_policy(&DetachUserPolicyInput {
                        executor: input.executor,
                        multi_tenancy: &scope,
                        user_id: user.id(),
                        policy_id: &policy_id,
                        tenant_id: &tenant,
                    })
                    .await?;
                info!(
                    user = %user.id(),
                    tenant = %tenant,
                    policy = %policy_id,
                    "detached an owner policy from a member who is not an owner"
                );
            }
        }

        info!(
            user = %input.target_user_id,
            tenant = %input.tenant_id,
            new_role = ?new_role,
            "changed organization member role"
        );

        user.role = DefaultRole::from(new_role);
        Ok(ChangeOrgMemberRoleOutputData { user })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::LIBRARY_TENANT;
    use chrono::Utc;
    use std::sync::Mutex;
    use tachyon_sdk::auth::{MockAuthApp, MultiTenancy};

    const ORG: &str = "tn_01hjryxysgey07h5jz5wagqj0m";
    const OTHER_ORG: &str = "tn_01hjryxysgey07h5jz5wagqj0n";
    const TARGET: &str = "us_01ke1h5471vxsbscp8jd3bramn";

    #[derive(Debug, Clone, PartialEq)]
    enum Call {
        Check {
            operator: Option<String>,
        },
        Attach {
            policy: String,
            operator: Option<String>,
        },
        Detach {
            policy: String,
            operator: Option<String>,
        },
    }

    fn member(role: DefaultRole, tenants: &[&str]) -> User {
        let now = Utc::now();
        User {
            id: UserId::new(TARGET).unwrap(),
            username: "target".to_string(),
            tenants: tenants
                .iter()
                .map(|t| TenantId::new(t).unwrap())
                .collect(),
            email: None,
            name: None,
            email_verified: None,
            image: None,
            role,
            metadata: None,
            created_at: now,
            updated_at: now,
        }
    }

    fn auth(
        calls: Arc<Mutex<Vec<Call>>>,
        target: Option<User>,
        fail_detach_of: Option<PolicyId>,
    ) -> MockAuthApp {
        let mut auth = MockAuthApp::new();
        auth.expect_check_policy().returning({
            let calls = calls.clone();
            move |input| {
                calls.lock().unwrap().push(Call::Check {
                    operator: input
                        .multi_tenancy
                        .operator_id()
                        .map(|t| t.to_string()),
                });
                Box::pin(async { Ok(()) })
            }
        });
        auth.expect_get_user_by_id().returning(move |_| {
            let target = target.clone();
            Box::pin(async move { Ok(target) })
        });
        auth.expect_attach_user_policy().returning({
            let calls = calls.clone();
            move |input| {
                calls.lock().unwrap().push(Call::Attach {
                    policy: input.policy_id.to_string(),
                    operator: input
                        .multi_tenancy
                        .operator_id()
                        .map(|t| t.to_string()),
                });
                Box::pin(async { Ok(()) })
            }
        });
        auth.expect_detach_user_policy().returning({
            let calls = calls.clone();
            move |input| {
                calls.lock().unwrap().push(Call::Detach {
                    policy: input.policy_id.to_string(),
                    operator: input
                        .multi_tenancy
                        .operator_id()
                        .map(|t| t.to_string()),
                });
                let refuse =
                    fail_detach_of.as_ref() == Some(input.policy_id);
                Box::pin(async move {
                    if refuse {
                        Err(errors::Error::internal_server_error(
                            "detach refused",
                        ))
                    } else {
                        Ok(())
                    }
                })
            }
        });
        auth
    }

    async fn change(
        auth: MockAuthApp,
        header: MultiTenancy,
        new_role: OrgRole,
    ) -> errors::Result<ChangeOrgMemberRoleOutputData> {
        let usecase = ChangeOrgMemberRole::new(Arc::new(auth));
        let executor = tachyon_sdk::auth::Executor::SystemUser;
        let tenant: OperatorId = ORG.parse().unwrap();
        let target = UserId::new(TARGET).unwrap();
        usecase
            .execute(ChangeOrgMemberRoleInputData {
                executor: &executor,
                multi_tenancy: &header,
                tenant_id: &tenant,
                target_user_id: &target,
                new_role,
            })
            .await
    }

    fn platform_header() -> MultiTenancy {
        MultiTenancy::new(
            Some(LIBRARY_TENANT.clone()),
            Some(LIBRARY_TENANT.clone()),
        )
    }

    fn org() -> Option<String> {
        Some(ORG.to_string())
    }

    /// The finding this guards: a member whose label never said Owner
    /// (a tenant administrator seeded as Manager, say) still carries the
    /// repo owner policy, and demoting them must take it away.
    #[tokio::test]
    async fn demotion_detaches_owner_policies_whatever_the_label_says() {
        for label in [
            DefaultRole::Manager,
            DefaultRole::General,
            DefaultRole::Owner,
        ] {
            let calls = Arc::new(Mutex::new(Vec::new()));
            let output = change(
                auth(calls.clone(), Some(member(label, &[ORG])), None),
                platform_header(),
                OrgRole::General,
            )
            .await
            .unwrap();

            assert_eq!(output.user.role, DefaultRole::General);
            let calls = calls.lock().unwrap().clone();
            assert_eq!(
                calls,
                vec![
                    Call::Check { operator: org() },
                    Call::Detach {
                        policy: library_repo_owner_policy_id().to_string(),
                        operator: org(),
                    },
                    Call::Detach {
                        policy: library_api_key_issuer_policy_id()
                            .to_string(),
                        operator: org(),
                    },
                    Call::Detach {
                        policy: library_api_key_accounts_policy_id()
                            .to_string(),
                        operator: org(),
                    },
                ],
                "label {label:?}"
            );
        }
    }

    #[tokio::test]
    async fn promotion_attaches_repo_owner_policy_in_the_org_tenant() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let output = change(
            auth(
                calls.clone(),
                Some(member(DefaultRole::General, &[ORG])),
                None,
            ),
            platform_header(),
            OrgRole::Owner,
        )
        .await
        .unwrap();

        assert_eq!(output.user.role, DefaultRole::Owner);
        assert_eq!(
            calls.lock().unwrap().clone(),
            vec![
                Call::Check { operator: org() },
                Call::Attach {
                    policy: library_repo_owner_policy_id().to_string(),
                    operator: org(),
                },
            ]
        );
    }

    /// The repo owner policy goes first, so a refusal on a later detach
    /// leaves the member without owner rights over the repositories, and
    /// the error reaches the caller to repeat the request.
    #[tokio::test]
    async fn a_failed_api_key_detach_still_leaves_repo_owner_policy_removed(
    ) {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let result = change(
            auth(
                calls.clone(),
                Some(member(DefaultRole::Owner, &[ORG])),
                Some(library_api_key_issuer_policy_id()),
            ),
            platform_header(),
            OrgRole::Manager,
        )
        .await;

        assert!(result.is_err());
        let calls = calls.lock().unwrap().clone();
        assert_eq!(
            calls[1],
            Call::Detach {
                policy: library_repo_owner_policy_id().to_string(),
                operator: org(),
            }
        );
        assert!(!calls.iter().any(|c| matches!(c, Call::Attach { .. })));
    }

    #[tokio::test]
    async fn a_user_outside_the_tenant_is_not_promoted() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let result = change(
            auth(
                calls.clone(),
                Some(member(DefaultRole::General, &[OTHER_ORG])),
                None,
            ),
            platform_header(),
            OrgRole::Owner,
        )
        .await;

        assert!(
            matches!(result, Err(errors::Error::NotFound { .. })),
            "{result:?}"
        );
        assert!(!calls
            .lock()
            .unwrap()
            .iter()
            .any(|c| matches!(c, Call::Attach { .. })));
    }

    #[tokio::test]
    async fn a_header_naming_another_organization_is_rejected() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let other = TenantId::new(OTHER_ORG).unwrap();
        let result = change(
            auth(
                calls.clone(),
                Some(member(DefaultRole::General, &[ORG])),
                None,
            ),
            MultiTenancy::new(None, Some(other)),
            OrgRole::Owner,
        )
        .await;

        assert!(
            matches!(result, Err(errors::Error::Forbidden { .. })),
            "{result:?}"
        );
        assert!(calls.lock().unwrap().is_empty());
    }
}
