use std::collections::HashSet;
use std::sync::Arc;
use tracing::info;

use inbound_sync::sdk::SystemExecutor;
use tachyon_sdk::auth::{AuthApp as AuthAppTrait, DefaultRole, User};
use value_object::{PlatformId, TenantId};

use crate::domain::{library_repo_owner_policy_id, library_user_policy_id};
use crate::sdk_auth::SdkAuthApp;

const LIBRARY_PLATFORM_ID: &str = "tn_01j702qf86pc2j35s0kv0gv3gy";

#[derive(Debug, Clone)]
pub struct SignIn {
    sdk: Arc<SdkAuthApp>,
}

impl SignIn {
    pub fn new(sdk: Arc<SdkAuthApp>) -> Self {
        Self { sdk }
    }
}

#[async_trait::async_trait]
pub trait SignInInputPort: Send + Sync + std::fmt::Debug {
    async fn execute(
        &self,
        platform_id: PlatformId,
        access_token: String,
        allow_sign_up: Option<bool>,
    ) -> errors::Result<User>;
}

#[async_trait::async_trait]
impl SignInInputPort for SignIn {
    async fn execute(
        &self,
        platform_id: PlatformId,
        access_token: String,
        allow_sign_up: Option<bool>,
    ) -> errors::Result<User> {
        let user = self
            .sdk
            .sign_in_with_platform(
                platform_id.as_ref(),
                &access_token,
                allow_sign_up,
                None,
                None,
            )
            .await?;

        self.attach_library_policy(&user, &platform_id).await?;
        Ok(user)
    }
}

impl SignIn {
    /// Attach library policy to user for the given
    /// platform.
    async fn attach_library_policy(
        &self,
        user: &User,
        platform_id: &PlatformId,
    ) -> errors::Result<()> {
        if platform_id.as_ref() != LIBRARY_PLATFORM_ID {
            return Ok(());
        }

        let policy_id = library_user_policy_id();
        let platform_tenant = TenantId::new(LIBRARY_PLATFORM_ID)?;
        let executor = &SystemExecutor;
        let multi_tenancy = &tachyon_sdk::auth::MultiTenancy::default();

        AuthAppTrait::attach_user_policy(
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

        info!(
            user = %user.id(),
            tenant = %platform_tenant,
            "attached platform-scope library policy"
        );

        let mut seen = HashSet::new();
        for tenant in user.tenants() {
            if seen.insert(tenant.to_string()) {
                AuthAppTrait::attach_user_policy(
                    self.sdk.as_ref(),
                    &tachyon_sdk::auth::AttachUserPolicyInput {
                        executor,
                        multi_tenancy,
                        user_id: user.id(),
                        policy_id: &policy_id,
                        tenant_id: tenant,
                    },
                )
                .await?;

                info!(
                    user = %user.id(),
                    tenant = %tenant,
                    "attached operator-scope library policy"
                );

                self.attach_repo_owner_policy_if_org_owner(user, tenant)
                    .await?;
            }
        }

        Ok(())
    }

    /// Check if user is org owner and attach repo
    /// owner policy.
    async fn attach_repo_owner_policy_if_org_owner(
        &self,
        user: &User,
        tenant_id: &TenantId,
    ) -> errors::Result<()> {
        let user_in_tenant = self
            .sdk
            .get_user_by_id_full(tenant_id, user.id().as_str())
            .await?
            .ok_or_else(|| {
                errors::not_found!("User not found in tenant")
            })?;

        if *user_in_tenant.role() != DefaultRole::Owner {
            return Ok(());
        }

        let policy_id = library_repo_owner_policy_id();
        let executor = &SystemExecutor;
        let multi_tenancy = &tachyon_sdk::auth::MultiTenancy::default();

        AuthAppTrait::attach_user_policy(
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
