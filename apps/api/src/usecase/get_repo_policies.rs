//! Get repository policies for GraphQL resolver.
//!
//! This usecase retrieves all policies with user information
//! for a specific repository, extracting roles from policy names.

use std::sync::Arc;

use async_trait::async_trait;
use tachyon_sdk::auth::{AuthApp, DefaultRole};

use crate::usecase::{
    GetRepoPoliciesInputData, GetRepoPoliciesInputPort, PermissionSource,
    RepoPolicyInfo,
};

/// Usecase for getting repository policies.
#[derive(Debug)]
pub struct GetRepoPolicies {
    user_policy_mapping_repo:
        Arc<dyn tachyon_sdk::auth::UserPolicyMappingRepository>,
    auth_app: Arc<dyn AuthApp>,
}

impl GetRepoPolicies {
    /// Create a new GetRepoPolicies usecase instance.
    pub fn new(
        user_policy_mapping_repo: Arc<
            dyn tachyon_sdk::auth::UserPolicyMappingRepository,
        >,
        auth_app: Arc<dyn AuthApp>,
    ) -> Self {
        Self {
            user_policy_mapping_repo,
            auth_app,
        }
    }
}

#[async_trait]
impl GetRepoPoliciesInputPort for GetRepoPolicies {
    async fn execute<'a>(
        &self,
        input: GetRepoPoliciesInputData<'a>,
    ) -> errors::Result<Vec<RepoPolicyInfo>> {
        use std::collections::HashSet;

        // Get user policies scoped to this resource.
        //
        // Best effort like the enrichment lookups below: this is served by
        // the auth service, which rejects a caller that cannot read policy
        // mappings. A repo Owner is such a caller, and letting that denial
        // escape made `policies` fail for the very people who own the repo.
        let user_policies = self
            .user_policy_mapping_repo
            .find_by_resource_scope(input.tenant_id, input.resource_trn)
            .await
            .unwrap_or_else(|error| {
                tracing::warn!(
                    "[GetRepoPolicies] Could not read policy mappings,                      returning no repo-level policies: {error:?}"
                );
                Vec::new()
            });

        tracing::info!(
            "[GetRepoPolicies] Found {} user policies for resource: {}",
            user_policies.len(),
            input.resource_trn
        );

        // For unauthenticated users, return basic policy info
        // without calling auth-protected usecases
        if input.executor.is_none() {
            let policies = user_policies
                .into_iter()
                .map(|up| {
                    let role = extract_role_from_policy_id(up.policy_id());
                    RepoPolicyInfo {
                        user_id: up.user_id().to_string(),
                        role,
                        user: None,
                        permission_source: PermissionSource::Repo,
                    }
                })
                .collect();
            return Ok(policies);
        }

        // Track seen user IDs to avoid duplicates
        let mut seen_user_ids = HashSet::new();

        // Get policy details and user info for repo-level policies
        let mut policies = Vec::new();
        for user_policy in user_policies {
            // Get policy info via SDK
            let policy_id = tachyon_sdk::auth::PolicyId::new(
                user_policy.policy_id().as_ref(),
            );
            // Best effort, like get_user_by_id below. Reading a policy is an
            // auth-service permission that a Library repo Owner does not
            // hold, so propagating this error failed the whole `policies`
            // field -- and with it the repository settings query, leaving a
            // repo's own creator unable to open its settings. The policy id
            // already carries the role, so losing this lookup costs nothing.
            let policy = self
                .auth_app
                .get_policy_by_id(&tachyon_sdk::auth::GetPolicyByIdInput {
                    executor: input.executor,
                    multi_tenancy: input.multi_tenancy,
                    policy_id: &policy_id,
                })
                .await
                .ok()
                .flatten();

            // Extract role from policy name (e.g., "LibraryRepoOwnerPolicy" -> "owner")
            let role = policy
                .as_ref()
                .and_then(|p| {
                    let name = p.name().to_lowercase();
                    if name.contains("owner") {
                        Some("owner".to_string())
                    } else if name.contains("writer") {
                        Some("writer".to_string())
                    } else if name.contains("reader") {
                        Some("reader".to_string())
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| {
                    extract_role_from_policy_id(user_policy.policy_id())
                });

            // Get user info via SDK
            let user_id: value_object::UserId =
                value_object::UserId::new(user_policy.user_id())?;
            let user = self
                .auth_app
                .get_user_by_id(&tachyon_sdk::auth::GetUserByIdInput {
                    executor: input.executor,
                    multi_tenancy: input.multi_tenancy,
                    user_id: &user_id,
                })
                .await
                .ok()
                .flatten();

            seen_user_ids.insert(user_policy.user_id().to_string());

            policies.push(RepoPolicyInfo {
                user_id: user_policy.user_id().to_string(),
                role,
                user,
                permission_source: PermissionSource::Repo,
            });
        }

        // Add org owners who are not already in the list via SDK
        // Also best effort: listing a tenant's users is an org-level
        // permission. A repo Owner who cannot enumerate the organization
        // should still see the repo-level policies gathered above rather
        // than lose the whole field.
        let org_users = self
            .auth_app
            .find_users_by_tenant(
                &tachyon_sdk::auth::FindUsersByTenantInput {
                    executor: input.executor,
                    multi_tenancy: input.multi_tenancy,
                    tenant_id: input.tenant_id,
                },
            )
            .await
            .unwrap_or_else(|error| {
                tracing::warn!(
                    "[GetRepoPolicies] Could not list organization users,                      returning repo-level policies only: {error:?}"
                );
                Vec::new()
            });

        for user in org_users {
            // Only add org owners who don't already have a repo-level policy
            if *user.role() == DefaultRole::Owner
                && !seen_user_ids.contains(&user.id().to_string())
            {
                tracing::info!(
                    "[GetRepoPolicies] Adding org owner {} as repo owner",
                    user.id()
                );
                policies.push(RepoPolicyInfo {
                    user_id: user.id().to_string(),
                    role: "owner".to_string(),
                    user: Some(user),
                    permission_source: PermissionSource::Org,
                });
            }
        }

        Ok(policies)
    }
}

/// Extract role from policy ID by matching known patterns.
///
/// Policy IDs follow the pattern `pol_01libraryrepo{role}`,
/// e.g., `pol_01libraryrepoowner`, `pol_01libraryrepowriter`,
/// `pol_01libraryreporeader`.
fn extract_role_from_policy_id(
    policy_id: &tachyon_sdk::auth::PolicyId,
) -> String {
    let id = policy_id.as_ref().to_lowercase();
    if id.contains("owner") {
        "owner".to_string()
    } else if id.contains("writer") {
        "writer".to_string()
    } else if id.contains("reader") {
        "reader".to_string()
    } else {
        tracing::warn!(
            "[GetRepoPolicies] Could not extract role from \
             policy_id: {}, defaulting to 'reader'",
            policy_id
        );
        "reader".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use tachyon_sdk::auth::{
        test_helper::{create_test_executor, create_test_multi_tenancy},
        MockAuthApp, PolicyId, UserPolicy,
    };
    use value_object::{TenantId, UserId};

    const TENANT: &str = "tn_01j702qf86pc2j35s0kv0gv3gy";
    const USER: &str = "us_01j702qf86pc2j35s0kv0gv3gy";
    const REPO_TRN: &str = "trn:library:repo:repo_01j702qf86pc2j35s0kv0";

    #[derive(Debug)]
    struct StubMappings(Vec<UserPolicy>);

    #[async_trait]
    impl tachyon_sdk::auth::UserPolicyMappingRepository for StubMappings {
        async fn create_mapping(
            &self,
            _user_id: &UserId,
            _policy_id: &PolicyId,
            _tenant_id: &TenantId,
        ) -> errors::Result<()> {
            unimplemented!("not used by these tests")
        }

        async fn delete_mapping(
            &self,
            _user_id: &UserId,
            _policy_id: &PolicyId,
            _tenant_id: &TenantId,
        ) -> errors::Result<()> {
            unimplemented!("not used by these tests")
        }

        async fn create_mapping_with_scope(
            &self,
            _user_id: &UserId,
            _policy_id: &PolicyId,
            _tenant_id: &TenantId,
            _resource_scope: &str,
        ) -> errors::Result<()> {
            unimplemented!("not used by these tests")
        }

        async fn delete_mapping_with_scope(
            &self,
            _user_id: &UserId,
            _policy_id: &PolicyId,
            _tenant_id: &TenantId,
            _resource_scope: &str,
        ) -> errors::Result<()> {
            unimplemented!("not used by these tests")
        }

        async fn find_policies_by_user(
            &self,
            _user_id: &UserId,
            _tenant_id: &TenantId,
        ) -> errors::Result<Vec<PolicyId>> {
            unimplemented!("not used by these tests")
        }

        async fn find_users_by_policy(
            &self,
            _policy_id: &PolicyId,
            _tenant_id: &TenantId,
        ) -> errors::Result<Vec<UserId>> {
            unimplemented!("not used by these tests")
        }

        async fn exists_mapping(
            &self,
            _user_id: &UserId,
            _policy_id: &PolicyId,
            _tenant_id: &TenantId,
        ) -> errors::Result<bool> {
            unimplemented!("not used by these tests")
        }

        async fn find_by_resource_scope(
            &self,
            _tenant_id: &TenantId,
            _resource_scope: &str,
        ) -> errors::Result<Vec<UserPolicy>> {
            Ok(self.0.clone())
        }
    }

    fn owner_mapping() -> UserPolicy {
        UserPolicy {
            user_id: USER.parse().expect("valid UserId"),
            policy_id: PolicyId::new("pol_01libraryrepoowner"),
            tenant_id: TENANT.parse().expect("valid TenantId"),
            resource_scope: Some(REPO_TRN.to_string()),
            assigned_at: Utc::now(),
        }
    }

    /// A repo Owner holds Library permissions, not auth-service ones, so
    /// both enrichment lookups are denied for them. Neither may take the
    /// field down: that is what left a repo's creator unable to open its
    /// own settings, since the settings query selects `policies`.
    #[tokio::test]
    async fn denied_enrichment_lookups_still_return_the_repo_policies() {
        let mut auth = MockAuthApp::new();
        auth.expect_get_policy_by_id().returning(|_| {
            Box::pin(async { Err(errors::Error::forbidden("denied")) })
        });
        auth.expect_get_user_by_id().returning(|_| {
            Box::pin(async { Err(errors::Error::forbidden("denied")) })
        });
        auth.expect_find_users_by_tenant().returning(|_| {
            Box::pin(async { Err(errors::Error::forbidden("denied")) })
        });

        let usecase = GetRepoPolicies::new(
            Arc::new(StubMappings(vec![owner_mapping()])),
            Arc::new(auth),
        );

        let executor = create_test_executor();
        let multi_tenancy = create_test_multi_tenancy();
        let tenant_id: TenantId = TENANT.parse().expect("valid TenantId");
        let policies = usecase
            .execute(GetRepoPoliciesInputData {
                executor: &executor,
                multi_tenancy: &multi_tenancy,
                resource_trn: REPO_TRN,
                tenant_id: &tenant_id,
            })
            .await
            .expect("a denied enrichment lookup must not fail the field");

        assert_eq!(policies.len(), 1);
        // The role comes from the policy id, so it survives the denial
        // rather than silently degrading to "reader".
        assert_eq!(policies[0].role, "owner");
        assert_eq!(policies[0].user_id, USER);
        assert!(policies[0].user.is_none());
    }
}
