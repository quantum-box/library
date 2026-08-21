use std::collections::HashSet;
use std::sync::Arc;
use tracing::{info, warn};

use inbound_sync::sdk::SystemExecutor;
use tachyon_sdk::auth::{AuthApp as AuthAppTrait, DefaultRole, User};
use value_object::{PlatformId, TenantId};

use crate::domain::{
    library_org_creator_policy_id, library_repo_owner_policy_id,
    library_user_policy_id, LIBRARY_TENANT,
};
use crate::sdk_auth::SdkAuthApp;

fn should_attach_library_policy(
    platform_id: &str,
    library_tenant_id: &str,
) -> bool {
    platform_id == library_tenant_id
}

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
        if !should_attach_library_policy(
            platform_id.as_ref(),
            LIBRARY_TENANT.as_ref(),
        ) {
            return Ok(());
        }

        let policy_id = library_user_policy_id();
        let platform_tenant = TenantId::new(LIBRARY_TENANT.as_ref())?;
        let executor = &SystemExecutor;
        let platform_scope = tachyon_sdk::auth::MultiTenancy::new(
            Some(platform_tenant.clone()),
            Some(platform_tenant.clone()),
        );

        AuthAppTrait::attach_user_policy(
            self.sdk.as_ref(),
            &tachyon_sdk::auth::AttachUserPolicyInput {
                executor,
                multi_tenancy: &platform_scope,
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

        // Organization creation is authorized as the caller, and the
        // tachyon side of it (`create_operator`) demands
        // `auth:CreateOperator` from that caller. LibraryUserPolicy is
        // a system policy that cannot be amended to carry the action,
        // so it rides in this companion policy.
        if let Some(org_creator_policy_id) = library_org_creator_policy_id()
        {
            AuthAppTrait::attach_user_policy(
                self.sdk.as_ref(),
                &tachyon_sdk::auth::AttachUserPolicyInput {
                    executor,
                    multi_tenancy: &platform_scope,
                    user_id: user.id(),
                    policy_id: &org_creator_policy_id,
                    tenant_id: &platform_tenant,
                },
            )
            .await?;

            info!(
                user = %user.id(),
                tenant = %platform_tenant,
                "attached platform-scope org creator policy"
            );
        }

        // Signing in is authentication; seeding these policies is
        // provisioning that happens to ride along. One organization
        // refusing a grant is not a reason to deny the caller every
        // other organization they belong to — and refusing it fails
        // sign-in outright, locking them out of the product. Record
        // each failure and carry on.
        let mut seen = HashSet::new();
        for tenant in user.tenants() {
            if !seen.insert(tenant.to_string()) {
                continue;
            }

            let tenant_scope = tachyon_sdk::auth::MultiTenancy::new(
                Some(platform_tenant.clone()),
                Some(tenant.clone()),
            );
            if let Err(error) = AuthAppTrait::attach_user_policy(
                self.sdk.as_ref(),
                &tachyon_sdk::auth::AttachUserPolicyInput {
                    executor,
                    multi_tenancy: &tenant_scope,
                    user_id: user.id(),
                    policy_id: &policy_id,
                    tenant_id: tenant,
                },
            )
            .await
            {
                warn!(
                    error = ?error,
                    user = %user.id(),
                    tenant = %tenant,
                    "failed to attach the operator-scope library policy"
                );
                continue;
            }

            info!(
                user = %user.id(),
                tenant = %tenant,
                "attached operator-scope library policy"
            );

            if let Err(error) = self
                .attach_repo_owner_policy_if_org_owner(user, tenant)
                .await
            {
                warn!(
                    error = ?error,
                    user = %user.id(),
                    tenant = %tenant,
                    "failed to attach the repo owner policy"
                );
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
        let multi_tenancy = tachyon_sdk::auth::MultiTenancy::new(
            Some(TenantId::new(LIBRARY_TENANT.as_ref())?),
            Some(tenant_id.clone()),
        );

        AuthAppTrait::attach_user_policy(
            self.sdk.as_ref(),
            &tachyon_sdk::auth::AttachUserPolicyInput {
                executor,
                multi_tenancy: &multi_tenancy,
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

#[cfg(test)]
mod tests {
    use super::{should_attach_library_policy, SignIn};
    use crate::domain::LIBRARY_TENANT;
    use crate::sdk_auth::SdkAuthApp;
    use std::sync::{Arc, Mutex};
    use tachyon_sdk::auth::{DefaultRole, User};
    use value_object::{PlatformId, TenantId};

    #[test]
    fn attaches_policy_for_the_configured_library_tenant() {
        assert!(should_attach_library_policy(
            "tn_01j91h09tpj5ehwbwfwfxpak2b",
            "tn_01j91h09tpj5ehwbwfwfxpak2b",
        ));
    }

    #[test]
    fn skips_policy_for_an_unrelated_platform() {
        assert!(!should_attach_library_policy(
            "tn_01j702qf86pc2j35s0kv0gv3gy",
            "tn_01j91h09tpj5ehwbwfwfxpak2b",
        ));
    }

    fn user_belonging_to(tenants: &[&str]) -> User {
        User {
            id: "us_01testcaller".parse().unwrap(),
            username: "us_01testcaller".to_string(),
            tenants: tenants
                .iter()
                .map(|tenant| TenantId::new(tenant).unwrap())
                .collect(),
            email: None,
            name: None,
            email_verified: None,
            image: None,
            role: DefaultRole::General,
            metadata: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    /// Serves a tachyon that accepts the platform-scope grants and
    /// refuses every per-organization one, which is what production
    /// does. Returns the `tenantId` of each attach it was asked for.
    async fn attach_targets_when_org_grants_are_refused(
        user: &User,
    ) -> Vec<String> {
        let platform_tenant = LIBRARY_TENANT.as_str().to_string();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let captured = seen.clone();
        let refuse_below = platform_tenant.clone();

        let app = axum::Router::new()
            .route(
                "/v1/auth/user-policies/attach",
                axum::routing::post(
                    move |body: axum::Json<serde_json::Value>| {
                        let captured = captured.clone();
                        let refuse_below = refuse_below.clone();
                        async move {
                            let tenant = body
                                .get("tenantId")
                                .and_then(|id| id.as_str())
                                .unwrap_or_default()
                                .to_string();
                            let is_platform = tenant == refuse_below;
                            captured.lock().unwrap().push(tenant);
                            if is_platform {
                                (
                                    axum::http::StatusCode::OK,
                                    axum::Json(serde_json::json!({})),
                                )
                            } else {
                                (
                                    axum::http::StatusCode::FORBIDDEN,
                                    axum::Json(serde_json::json!({})),
                                )
                            }
                        }
                    },
                ),
            )
            .route(
                "/v1/auth/users/:id",
                axum::routing::get(|| async {
                    axum::Json(serde_json::json!({
                        "id": "us_01testcaller",
                        "role": "general",
                        "tenants": [],
                    }))
                }),
            );

        let listener =
            tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        let tenant_id = TenantId::new(&platform_tenant).unwrap();
        let sdk = SdkAuthApp::new(
            format!("http://{addr}"),
            &tenant_id,
            "process-level-token",
        );
        let sign_in = SignIn::new(Arc::new(sdk));
        let platform_id: PlatformId = platform_tenant.parse().unwrap();

        sign_in
            .attach_library_policy(user, &platform_id)
            .await
            .expect(
                "a refused per-organization grant must not fail sign-in",
            );

        let seen = seen.lock().unwrap().clone();
        seen
    }

    /// Signing in is authentication. A refused per-organization grant
    /// used to abort it, locking the caller out of the product entirely
    /// — including the organizations whose grants were fine.
    #[tokio::test]
    async fn a_refused_organization_grant_does_not_fail_sign_in() {
        let user = user_belonging_to(&["tn_01firstorganization"]);

        let seen = attach_targets_when_org_grants_are_refused(&user).await;

        assert!(seen.contains(&"tn_01firstorganization".to_string()));
    }

    /// And it must not stop the ones after it either.
    #[tokio::test]
    async fn a_refused_grant_does_not_skip_the_remaining_organizations() {
        let user = user_belonging_to(&[
            "tn_01firstorganization",
            "tn_01secondorganizatio",
        ]);

        let seen = attach_targets_when_org_grants_are_refused(&user).await;

        assert!(
            seen.contains(&"tn_01secondorganizatio".to_string()),
            "the second organization was never attempted: {seen:?}"
        );
    }
}
