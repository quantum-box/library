use std::fmt::Debug;
use std::sync::Arc;

use derive_new::new;
use tachyon_sdk::auth::MultiTenancyAction;
use value_object::{Identifier, OperatorId, PlatformId, TenantId};

use crate::domain::{Organization, OrganizationRepository, LIBRARY_TENANT};
use crate::sdk_auth::SdkAuthApp;

/// LibraryOrg is a specialized implementation of MultiTenancyAction for library-api
///
/// Unlike the general MultiTenancy struct, LibraryOrg identifies tenants
/// using organization usernames from URL paths. An organization's scope is
/// built only from the organization's recorded platform
/// ([`LibraryOrg::for_organization`]): Library's own for an organization
/// created in Library, none for one adopted from another platform.
#[derive(Debug, Clone, Default)]
pub struct LibraryOrg {
    /// The organization username from the URL path, stored as operator_id
    operator: Option<OperatorId>,
    /// The original organization username string
    org_username: Option<String>,
    /// The organization's tenant lives on a platform other than Library's.
    other_platform: bool,
}

impl LibraryOrg {
    /// Create a new LibraryOrg with the given organization username
    pub fn with_org(org_username: String) -> Self {
        // Only the username is known here, not the organization's tenant.
        // Without an operator, the use cases authorize the action in the
        // tenant of the organization they resolve from the username.
        Self {
            operator: None,
            org_username: Some(org_username),
            other_platform: false,
        }
    }

    /// Get the organization username
    pub fn org_username(&self) -> Option<&String> {
        self.org_username.as_ref()
    }

    /// The scope of an organization whose platform has been recorded, or
    /// `None` when it has not (see [`OrgScopes::scope_for`]).
    ///
    /// Every scope Library sends to tachyon for an organization comes from
    /// here. A scope that claimed Library's platform for an adopted
    /// organization is rejected by tachyon-api (403 "x-platform-id does not
    /// match the operator's platform").
    pub fn for_organization(org: &Organization) -> Option<Self> {
        let scope = Self::with_org_and_operator(
            org.username().to_string(),
            org.id().clone(),
        );
        match scope_platform(org.home_platform().as_ref(), &LIBRARY_TENANT)
        {
            ScopePlatform::Library => Some(scope),
            ScopePlatform::Other => Some(scope.on_other_platform()),
            ScopePlatform::Unrecorded => None,
        }
    }

    /// The scope of a tenant that only Library's own platform may hold, for
    /// a request that must fail upstream when the tenant is elsewhere.
    pub fn library_native(org_username: String, tenant: TenantId) -> Self {
        Self::with_org_and_operator(org_username, tenant)
    }

    /// A scope for reads that stay inside Library's database and never reach
    /// tachyon, where the platform is not consulted.
    pub fn for_local_read(org_username: String, tenant: TenantId) -> Self {
        Self::with_org_and_operator(org_username, tenant)
    }

    fn with_org_and_operator(
        org_username: String,
        operator_id: OperatorId,
    ) -> Self {
        Self {
            operator: Some(operator_id),
            org_username: Some(org_username),
            other_platform: false,
        }
    }

    fn on_other_platform(mut self) -> Self {
        self.other_platform = true;
        self
    }
}

pub(crate) use verified_org_scope::ScopePlatform;

/// Where an organization's scope sits, from its recorded platform. The rule
/// is proven in `verified_org_scope::scope_platform`.
pub(crate) fn scope_platform(
    home: Option<&PlatformId>,
    library: &PlatformId,
) -> ScopePlatform {
    let home = home.map(|id| id.to_string());
    verified_org_scope::scope_platform(
        home.is_some(),
        home.as_deref().unwrap_or_default().as_bytes(),
        library.to_string().as_bytes(),
    )
}

/// Builds organization scopes, recording the platform of organizations
/// stored before platforms were recorded.
#[derive(Debug, Clone, new)]
pub struct OrgScopes {
    sdk: Arc<SdkAuthApp>,
    organizations: Arc<dyn OrganizationRepository>,
}

impl OrgScopes {
    /// The SDK the scopes are resolved with, authenticated as Library.
    pub fn service_sdk(&self) -> &SdkAuthApp {
        &self.sdk
    }

    pub async fn scope_for(&self, org: &Organization) -> LibraryOrg {
        if let Some(scope) = LibraryOrg::for_organization(org) {
            return scope;
        }
        match self.record_home_platform(org).await {
            Ok(placed) => LibraryOrg::for_organization(&placed)
                .unwrap_or_else(|| Self::unplaced(org)),
            Err(error) => {
                tracing::warn!(
                    tenant_id = %org.id(),
                    error = ?error,
                    "Failed to record the platform of an organization"
                );
                Self::unplaced(org)
            }
        }
    }

    /// Until every stored organization has its platform recorded, an
    /// organization tachyon-api cannot place keeps Library's platform, as
    /// before platforms were recorded.
    fn unplaced(org: &Organization) -> LibraryOrg {
        LibraryOrg::with_org_and_operator(
            org.username().to_string(),
            org.id().clone(),
        )
    }

    async fn record_home_platform(
        &self,
        org: &Organization,
    ) -> errors::Result<Organization> {
        let operator = self
            .sdk
            .get_operator(org.id().as_str())
            .await?
            .ok_or_else(|| errors::Error::not_found("operator"))?;
        let platform = operator.platform_id.parse()?;
        let placed = org.clone().place_on_home_platform(&platform)?;
        // Only the platform: saving the whole organization would overwrite
        // a profile edit made since `org` was read.
        self.organizations
            .record_home_platform(placed.id(), &platform)
            .await?;
        Ok(placed)
    }
}

impl MultiTenancyAction for LibraryOrg {
    fn platform_id(&self) -> Option<PlatformId> {
        if self.other_platform {
            return None;
        }
        Some(LIBRARY_TENANT.clone())
    }

    fn operator_id(&self) -> Option<OperatorId> {
        // Return the operator ID if we have one
        self.operator.clone()
    }

    /// Get the operator ID for the current request
    ///
    /// For library-api, we return the operator ID if available,
    /// otherwise we return the LIBRARY_TENANT as a fallback
    fn get_operator_id(&self) -> errors::Result<OperatorId> {
        if let Some(operator) = &self.operator {
            Ok(operator.clone())
        } else {
            // Fallback to LIBRARY_TENANT if no operator is available
            Ok(LIBRARY_TENANT.clone())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_library_org_platform_id() {
        let library_org = LibraryOrg::default();
        assert_eq!(library_org.platform_id(), Some(LIBRARY_TENANT.clone()));
    }

    #[test]
    fn test_library_org_operator_id() {
        // Default has no operator
        let library_org = LibraryOrg::default();
        assert_eq!(library_org.operator_id(), None);

        // A username alone names no tenant, so there is no operator to
        // invent: the use case resolves the organization's own tenant.
        let org_username = "test-org".to_string();
        let library_org = LibraryOrg::with_org(org_username);
        assert_eq!(library_org.operator_id(), None);
    }

    #[test]
    fn test_library_org_get_operator_id() {
        // Default falls back to LIBRARY_TENANT
        let library_org = LibraryOrg::default();
        assert_eq!(
            library_org.get_operator_id().unwrap(),
            LIBRARY_TENANT.clone()
        );

        // With only a username it falls back the same way
        let org_username = "test-org".to_string();
        let library_org = LibraryOrg::with_org(org_username);
        assert_eq!(
            library_org.get_operator_id().unwrap(),
            LIBRARY_TENANT.clone()
        );
    }

    #[derive(Debug, Default)]
    struct Organizations(std::sync::Mutex<Vec<Organization>>);

    #[async_trait::async_trait]
    impl OrganizationRepository for Organizations {
        async fn insert(&self, org: &Organization) -> errors::Result<()> {
            self.0.lock().unwrap().push(org.clone());
            Ok(())
        }
        async fn update(&self, _: &Organization) -> errors::Result<()> {
            panic!("recording a platform must not save the organization")
        }
        async fn record_home_platform(
            &self,
            org_id: &TenantId,
            platform: &PlatformId,
        ) -> errors::Result<()> {
            self.0.lock().unwrap().push(Organization::reconstruct(
                org_id,
                &value_object::Text::new("recorded").unwrap(),
                &"recorded".parse().unwrap(),
                None,
                None,
                None,
                Some(platform),
            ));
            Ok(())
        }
        async fn get_by_id(
            &self,
            _: &TenantId,
        ) -> errors::Result<Option<Organization>> {
            Ok(None)
        }
        async fn get_by_username(
            &self,
            _: &Identifier,
        ) -> errors::Result<Option<Organization>> {
            Ok(None)
        }
        async fn find_all(&self) -> errors::Result<Vec<Organization>> {
            Ok(vec![])
        }
        async fn delete(&self, _: &TenantId) -> errors::Result<()> {
            Ok(())
        }
    }

    #[derive(Debug)]
    struct Caller;

    impl tachyon_sdk::auth::ExecutorAction for Caller {
        fn get_id(&self) -> &str {
            "us_01testcaller"
        }
        fn has_tenant_id(&self, _: &TenantId) -> bool {
            true
        }
        fn is_system_user(&self) -> bool {
            false
        }
        fn is_user(&self) -> bool {
            true
        }
        fn is_service_account(&self) -> bool {
            false
        }
        fn is_none(&self) -> bool {
            false
        }
    }

    const ADOPTED: &str = "tn_01kshr165k3wwr8xyk0e6vy36c";
    const OTHER_PLATFORM: &str = "tn_01hjryxysgey07h5jz5wagqj0m";

    fn organization(home: Option<&str>) -> Organization {
        Organization::reconstruct(
            &ADOPTED.parse().unwrap(),
            &value_object::Text::new("Bernard Square").unwrap(),
            &"bernard-square".parse().unwrap(),
            None,
            None,
            None,
            home.map(|id| id.parse::<TenantId>().unwrap()).as_ref(),
        )
    }

    /// Resolve the organization's scope and send one policy check with it
    /// through the real SDK adapter. Returns the platform header the check
    /// carried, the paths requested, and the organizations recorded.
    async fn check_with_scope(
        home: Option<&str>,
        operator_status: axum::http::StatusCode,
    ) -> (Option<String>, Vec<String>, Vec<Organization>) {
        let seen = Arc::new(std::sync::Mutex::new((None, Vec::new())));
        let captured = seen.clone();
        let app = axum::Router::new().fallback(axum::routing::any(
            move |uri: axum::http::Uri, headers: axum::http::HeaderMap| {
                let captured = captured.clone();
                async move {
                    let mut seen = captured.lock().unwrap();
                    seen.1.push(uri.path().to_string());
                    if uri.path().starts_with("/v1/auth/operators/") {
                        return (
                            operator_status,
                            axum::Json(serde_json::json!({
                                "id": ADOPTED,
                                "name": "Bernard Square",
                                "operatorName": "bernard-square",
                                "platformId": OTHER_PLATFORM,
                            })),
                        );
                    }
                    seen.0 = headers
                        .get("x-platform-id")
                        .map(|v| v.to_str().unwrap().to_string());
                    (
                        axum::http::StatusCode::OK,
                        axum::Json(serde_json::json!({
                            "results": [{"action": "library:CreateRepo", "allowed": true}]
                        })),
                    )
                }
            },
        ));
        let listener =
            tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap()
        });
        let sdk = Arc::new(
            SdkAuthApp::new(
                format!("http://{address}"),
                &LIBRARY_TENANT,
                "service-token",
            )
            .with_caller_token("caller-token"),
        );
        let organizations = Arc::new(Organizations::default());
        let scopes = OrgScopes::new(sdk.clone(), organizations.clone());
        let scope = scopes.scope_for(&organization(home)).await;
        crate::sdk_auth::caller_token_scope(
            Some("caller-token".to_string()),
            tachyon_sdk::auth::AuthApp::check_policy(
                sdk.as_ref(),
                &tachyon_sdk::auth::CheckPolicyInput {
                    executor: &Caller,
                    multi_tenancy: &scope,
                    action: "library:CreateRepo",
                },
            ),
        )
        .await
        .unwrap();
        server.abort();
        let (platform, paths) = seen.lock().unwrap().clone();
        let recorded = organizations.0.lock().unwrap().clone();
        (platform, paths, recorded)
    }

    #[tokio::test]
    async fn scopes_carry_the_organizations_platform_upstream() {
        let ok = axum::http::StatusCode::OK;
        let library = LIBRARY_TENANT.to_string();
        // (recorded platform, operator lookup, x-platform-id sent,
        //  platform recorded by the lookup)
        let cases: [(Option<&str>, _, Option<&str>, Option<&str>); 4] = [
            // Adopted, recorded: no platform, no lookup.
            (Some(OTHER_PLATFORM), ok, None, None),
            // Native, recorded: Library's platform, no lookup.
            (Some(library.as_str()), ok, Some(library.as_str()), None),
            // Unrecorded: tachyon places it, and the platform is recorded.
            (None, ok, None, Some(OTHER_PLATFORM)),
            // Unrecorded and tachyon cannot say: Library, as before (B1).
            (
                None,
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                Some(library.as_str()),
                None,
            ),
        ];
        for (home, lookup, sent, recorded) in cases {
            let (platform, paths, stored) =
                check_with_scope(home, lookup).await;
            assert_eq!(platform.as_deref(), sent, "home={home:?}");
            let looked_up =
                paths.iter().any(|p| p.starts_with("/v1/auth/operators/"));
            assert_eq!(looked_up, home.is_none(), "home={home:?}");
            assert_eq!(
                stored
                    .iter()
                    .map(|org| org
                        .home_platform()
                        .clone()
                        .unwrap()
                        .to_string())
                    .collect::<Vec<_>>(),
                recorded
                    .map(str::to_string)
                    .into_iter()
                    .collect::<Vec<_>>(),
                "home={home:?}"
            );
        }
    }

    #[test]
    fn scope_platform_follows_the_recorded_platform() {
        let library = LIBRARY_TENANT.clone();
        let other: PlatformId =
            "tn_01hjryxysgey07h5jz5wagqj0m".parse().unwrap();
        assert_eq!(
            scope_platform(None, &library),
            ScopePlatform::Unrecorded
        );
        assert_eq!(
            scope_platform(Some(&library), &library),
            ScopePlatform::Library
        );
        assert_eq!(
            scope_platform(Some(&other), &library),
            ScopePlatform::Other
        );
    }

    #[test]
    fn an_adopted_organization_claims_no_platform() {
        let tenant: TenantId =
            "tn_01kshr165k3wwr8xyk0e6vy36c".parse().unwrap();
        let other: PlatformId =
            "tn_01hjryxysgey07h5jz5wagqj0m".parse().unwrap();
        let adopted = Organization::adopt(
            &tenant,
            &value_object::Text::new("bernard-square").unwrap(),
            &"bernard-square".parse().unwrap(),
            &other,
        );
        let scope = LibraryOrg::for_organization(&adopted).unwrap();
        assert_eq!(scope.platform_id(), None);
        assert_eq!(scope.get_operator_id().unwrap(), tenant);

        let native = Organization::new(
            &tenant,
            &value_object::Text::new("native").unwrap(),
            &"native".parse().unwrap(),
            None,
            None,
        );
        assert_eq!(
            LibraryOrg::for_organization(&native).unwrap().platform_id(),
            Some(LIBRARY_TENANT.clone())
        );
    }

    #[test]
    fn an_unrecorded_organization_has_no_scope() {
        let unrecorded = Organization::reconstruct(
            &"tn_01kshr165k3wwr8xyk0e6vy36c".parse().unwrap(),
            &value_object::Text::new("old").unwrap(),
            &"old".parse().unwrap(),
            None,
            None,
            None,
            None,
        );
        assert!(LibraryOrg::for_organization(&unrecorded).is_none());
    }

    #[test]
    fn test_library_org_with_org() {
        let org_username = "test-org".to_string();
        let library_org = LibraryOrg::with_org(org_username.clone());
        assert_eq!(library_org.org_username(), Some(&org_username));
        assert!(library_org.operator.is_none());
    }
}
