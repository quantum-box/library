//! Get repository members.
//!
//! This usecase retrieves all users with access to a repository,
//! including those with resource-scoped policies and org owners.

use std::collections::HashSet;
use std::sync::Arc;

use async_trait::async_trait;
use tachyon_sdk::auth::{AuthApp, DefaultRole};

use crate::usecase::{
    GetRepoMembersInputData, GetRepoMembersInputPort, PermissionSource,
    RepoMemberInfo,
};

/// Usecase for getting repository members.
#[derive(Debug)]
pub struct GetRepoMembers {
    user_policy_mapping_repo:
        Arc<dyn tachyon_sdk::auth::UserPolicyMappingRepository>,
    auth_app: Arc<dyn AuthApp>,
}

impl GetRepoMembers {
    /// Create a new GetRepoMembers usecase instance.
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
impl GetRepoMembersInputPort for GetRepoMembers {
    async fn execute<'a>(
        &self,
        input: GetRepoMembersInputData<'a>,
    ) -> errors::Result<Vec<RepoMemberInfo>> {
        let user_policies = self
            .user_policy_mapping_repo
            .find_by_resource_scope(input.tenant_id, input.resource_trn)
            .await?;

        let mut seen_user_ids = HashSet::new();
        let mut members = Vec::new();

        // Get members with resource-scoped policies
        for up in user_policies {
            let Ok(user_id) = value_object::UserId::new(up.user_id())
            else {
                continue;
            };

            // Get policy info via SDK
            let policy_id =
                tachyon_sdk::auth::PolicyId::new(up.policy_id().as_ref());
            let policy = self
                .auth_app
                .get_policy_by_id(&tachyon_sdk::auth::GetPolicyByIdInput {
                    executor: input.executor,
                    multi_tenancy: input.multi_tenancy,
                    policy_id: &policy_id,
                })
                .await?;

            // Get user info via SDK
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

            seen_user_ids.insert(up.user_id().to_string());

            members.push(RepoMemberInfo {
                user_id,
                policy_id: up.policy_id().to_string(),
                policy_name: policy.map(|p| p.name().to_string()),
                resource_scope: up.resource_scope().map(|s| s.to_string()),
                assigned_at: *up.assigned_at(),
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
            if *user.role() == DefaultRole::Owner
                && !seen_user_ids.contains(&user.id().to_string())
            {
                members.push(RepoMemberInfo {
                    user_id: user.id().clone(),
                    policy_id: "org_owner".to_string(),
                    policy_name: Some("Organization Owner".to_string()),
                    resource_scope: None,
                    assigned_at: *user.created_at(),
                    user: Some(user),
                    permission_source: PermissionSource::Org,
                });
            }
        }

        Ok(members)
    }
}
