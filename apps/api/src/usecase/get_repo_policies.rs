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

        // Get user policies scoped to this resource
        let user_policies = self
            .user_policy_mapping_repo
            .find_by_resource_scope(
                input.tenant_id,
                input.resource_trn,
            )
            .await?;

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
            let policy = self
                .auth_app
                .get_policy_by_id(&tachyon_sdk::auth::GetPolicyByIdInput {
                    executor: input.executor,
                    multi_tenancy: input.multi_tenancy,
                    policy_id: &policy_id,
                })
                .await?;

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
                    tracing::warn!(
                        "[GetRepoPolicies] Policy not found or role extraction failed, defaulting to 'reader', user_id: {}",
                        user_policy.user_id()
                    );
                    "reader".to_string()
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
        let org_users = self
            .auth_app
            .find_users_by_tenant(
                &tachyon_sdk::auth::FindUsersByTenantInput {
                    executor: input.executor,
                    multi_tenancy: input.multi_tenancy,
                    tenant_id: input.tenant_id,
                },
            )
            .await?;

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
