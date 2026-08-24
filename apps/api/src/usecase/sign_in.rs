use std::sync::Arc;
use tracing::info;

use inbound_sync::sdk::SystemExecutor;
use tachyon_sdk::auth::{AuthApp as AuthAppTrait, User};
use value_object::{PlatformId, TenantId};

use crate::domain::{
    library_org_creator_policy_id, library_user_policy_id, LIBRARY_TENANT,
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
    /// Attach the platform-scope library policies to a user signing
    /// into the Library platform.
    ///
    /// Deliberately platform scope only. The policies a user needs
    /// inside an organization are granted where the membership is
    /// created — `CreateOrganization` for the creator,
    /// `InviteOrgMember` for everyone invited afterwards,
    /// `ChangeOrgMemberRole` when a role changes — and
    /// `seedLibraryTenant` backfills a tenant that predates those. All
    /// of them run as a caller who is an administrator of the target
    /// organization, which is what makes the grant possible at all: the
    /// service account this use case authenticates as belongs to the
    /// Library platform tenant and tachyon rejects it outright in any
    /// per-organization scope.
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

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        should_attach_library_policy, SdkAuthApp, SignIn, SignInInputPort,
        LIBRARY_TENANT,
    };
    use std::sync::{Arc, Mutex};

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

    /// Signs a user in against a stand-in tachyon and reports the
    /// `x-operator-id` of every policy attachment it received, in
    /// order. The user is reported as a member of two organizations, so
    /// a per-organization grant would show up here.
    async fn operator_scopes_of_grants_during_sign_in() -> Vec<String> {
        let scopes = Arc::new(Mutex::new(Vec::new()));

        let sign_in_body = serde_json::json!({
            "user": {
                "id": "us_01testcaller",
                "role": "general",
                "tenants": ["tn_01memberofthis", "tn_01andofthis"],
            }
        });

        let recorder = scopes.clone();
        let app = axum::Router::new()
            .route(
                "/auth/v1beta/sign-in-with-platform",
                axum::routing::post(move || {
                    let body = sign_in_body.clone();
                    async move { axum::Json(body) }
                }),
            )
            .route(
                "/v1/auth/user-policies/attach",
                axum::routing::post(
                    move |headers: axum::http::HeaderMap| {
                        let recorder = recorder.clone();
                        async move {
                            let operator = headers
                                .get("x-operator-id")
                                .and_then(|value| value.to_str().ok())
                                .unwrap_or("<none>")
                                .to_string();
                            recorder.lock().unwrap().push(operator);
                            axum::Json(serde_json::json!({}))
                        }
                    },
                ),
            );

        let listener =
            tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        let sign_in = SignIn::new(Arc::new(SdkAuthApp::new(
            format!("http://{addr}"),
            &LIBRARY_TENANT,
            "service-account-token",
        )));

        sign_in
            .execute(
                LIBRARY_TENANT.as_str().parse().unwrap(),
                "platform-access-token".to_string(),
                None,
            )
            .await
            .unwrap();

        let scopes = scopes.lock().unwrap().clone();
        scopes
    }

    /// The service account this use case authenticates as belongs to
    /// the Library platform tenant, and tachyon rejects it in any
    /// per-organization scope. Granting there aborted sign-in for
    /// everyone once memberships started arriving (#207, reverted in
    /// #210), so no grant may leave the platform scope.
    #[tokio::test]
    async fn signing_in_never_grants_in_an_organization_scope() {
        let scopes = operator_scopes_of_grants_during_sign_in().await;

        assert!(
            !scopes.is_empty(),
            "expected sign-in to grant the platform-scope policy"
        );
        for scope in &scopes {
            assert_eq!(
                scope,
                LIBRARY_TENANT.as_str(),
                "sign-in granted a policy outside the platform scope: \
                 {scopes:?}"
            );
        }
    }
}
