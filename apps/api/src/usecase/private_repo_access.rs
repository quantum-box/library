use tachyon_sdk::auth::{
    AuthApp, CheckPolicyForResourceInput, CheckPolicyInput, ExecutorAction,
    MultiTenancyAction,
};

/// Authorize reading a private repository of `organization_id`.
///
/// The checks run in the organization's own tenant (see
/// [`super::authorization_scope_for_org`]): a repository grant there, or
/// the organization-wide `library:ViewPrivateRepo`. Repository grants
/// made through the v1 client were attached in Library's platform tenant
/// with the repository as resource scope, so a caller that names the
/// platform tenant may also pass with `library:ViewRepo` on this
/// repository there. `LibraryUserPolicy`, which everyone holds in the
/// platform tenant, does not carry `library:ViewRepo`, so only such a
/// repository grant satisfies it.
pub async fn authorize_private_repo_read(
    auth_app: &dyn AuthApp,
    executor: &dyn ExecutorAction,
    multi_tenancy: &dyn MultiTenancyAction,
    organization_id: &value_object::TenantId,
    repo_id: &str,
) -> errors::Result<()> {
    let org_scope =
        super::authorization_scope_for_org(multi_tenancy, organization_id)?;
    let resource_trn = format!("trn:library:repo:{repo_id}");

    let resource_access = auth_app
        .check_policy_for_resource(&CheckPolicyForResourceInput {
            executor,
            multi_tenancy: &org_scope,
            action: "library:ViewRepo",
            resource_trn: &resource_trn,
        })
        .await;
    if resource_access.is_ok() {
        return Ok(());
    }

    let org_wide = auth_app
        .check_policy(&CheckPolicyInput {
            executor,
            multi_tenancy: &org_scope,
            action: "library:ViewPrivateRepo",
        })
        .await;
    match org_wide {
        Ok(()) => Ok(()),
        Err(error)
            if super::header_names_platform_tenant(multi_tenancy) =>
        {
            auth_app
                .check_policy_for_resource(&CheckPolicyForResourceInput {
                    executor,
                    multi_tenancy,
                    action: "library:ViewRepo",
                    resource_trn: &resource_trn,
                })
                .await
                .map_err(|_| error)
        }
        Err(error) => Err(error),
    }
}

/// Authorize listing the members of a repository (`Repo.members`).
///
/// The listing is the repository settings' view of who holds which
/// grant, with when it was given, so it is for the people inside: a
/// member of the organization or a holder of a grant on the repository,
/// which is exactly who may read it as if it were private. Being public
/// lets anyone read the repository's content, not who administers it.
pub async fn authorize_repo_member_listing(
    auth_app: &dyn AuthApp,
    executor: &dyn ExecutorAction,
    multi_tenancy: &dyn MultiTenancyAction,
    organization_id: &value_object::TenantId,
    repo_id: &str,
) -> errors::Result<()> {
    if executor.is_none() {
        return Err(errors::Error::forbidden(
            "Listing repository members needs a signed-in caller",
        ));
    }
    authorize_private_repo_read(
        auth_app,
        executor,
        multi_tenancy,
        organization_id,
        repo_id,
    )
    .await
}

/// Authorize listing the policies held on a repository
/// (`Repo.policies`).
///
/// A public repository shows who works on it (the repository page
/// lists them, as contributors), so its policies stay readable by
/// anyone. A private repository's are readable by whoever may read the
/// repository itself.
pub async fn authorize_repo_policy_listing(
    auth_app: &dyn AuthApp,
    executor: &dyn ExecutorAction,
    multi_tenancy: &dyn MultiTenancyAction,
    organization_id: &value_object::TenantId,
    repo_id: &str,
    is_public: bool,
) -> errors::Result<()> {
    if is_public {
        return Ok(());
    }
    if executor.is_none() {
        return Err(errors::Error::forbidden(
            "A private repository's policies need a signed-in caller",
        ));
    }
    authorize_private_repo_read(
        auth_app,
        executor,
        multi_tenancy,
        organization_id,
        repo_id,
    )
    .await
}

#[cfg(test)]
mod listing_tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use tachyon_sdk::auth::{
        test_helper::{create_test_executor, create_test_multi_tenancy},
        Executor, MockAuthApp,
    };

    fn denying_auth(calls: Arc<Mutex<Vec<String>>>) -> MockAuthApp {
        let mut auth = MockAuthApp::new();
        auth.expect_check_policy_for_resource().returning({
            let calls = calls.clone();
            move |input| {
                calls.lock().unwrap().push(format!(
                    "resource:{}:{}",
                    input.action,
                    input
                        .multi_tenancy
                        .operator_id()
                        .map(|t| t.to_string())
                        .unwrap_or_default()
                ));
                Box::pin(async { Err(errors::Error::forbidden("denied")) })
            }
        });
        auth.expect_check_policy().returning({
            let calls = calls.clone();
            move |input| {
                calls
                    .lock()
                    .unwrap()
                    .push(format!("policy:{}", input.action));
                Box::pin(async { Err(errors::Error::forbidden("denied")) })
            }
        });
        auth
    }

    fn org() -> value_object::TenantId {
        create_test_multi_tenancy().get_operator_id().unwrap()
    }

    #[tokio::test]
    async fn members_of_a_repo_are_not_listed_to_an_outsider() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let auth = denying_auth(calls.clone());
        let multi_tenancy = create_test_multi_tenancy();

        let error = authorize_repo_member_listing(
            &auth,
            &create_test_executor(),
            &multi_tenancy,
            &org(),
            "rp_01test",
        )
        .await
        .unwrap_err();

        assert!(matches!(error, errors::Error::Forbidden { .. }));
        assert_eq!(
            calls.lock().unwrap()[0],
            format!("resource:library:ViewRepo:{}", org())
        );
    }

    #[tokio::test]
    async fn members_are_not_listed_anonymously() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let auth = denying_auth(calls.clone());

        let error = authorize_repo_member_listing(
            &auth,
            &Executor::None,
            &create_test_multi_tenancy(),
            &org(),
            "rp_01test",
        )
        .await
        .unwrap_err();

        assert!(matches!(error, errors::Error::Forbidden { .. }));
        assert!(calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_public_repos_policies_stay_readable() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let auth = denying_auth(calls.clone());

        authorize_repo_policy_listing(
            &auth,
            &Executor::None,
            &create_test_multi_tenancy(),
            &org(),
            "rp_01test",
            true,
        )
        .await
        .unwrap();
        assert!(calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_private_repos_policies_need_read_access() {
        for executor in [Executor::None, create_test_executor()] {
            let calls = Arc::new(Mutex::new(Vec::new()));
            let auth = denying_auth(calls.clone());

            let error = authorize_repo_policy_listing(
                &auth,
                &executor,
                &create_test_multi_tenancy(),
                &org(),
                "rp_01test",
                false,
            )
            .await
            .unwrap_err();

            assert!(matches!(error, errors::Error::Forbidden { .. }));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::LIBRARY_TENANT;
    use std::sync::{Arc, Mutex};
    use tachyon_sdk::auth::{
        test_helper::{create_test_executor, create_test_multi_tenancy},
        MockAuthApp, MultiTenancy,
    };

    fn mock_auth(
        calls: Arc<Mutex<Vec<String>>>,
        deny_resource: bool,
        deny_org_wide: bool,
    ) -> MockAuthApp {
        mock_auth_with_platform_grant(
            calls,
            deny_resource,
            deny_org_wide,
            false,
        )
    }

    /// `platform_grant`: a repository grant exists in the platform tenant.
    fn mock_auth_with_platform_grant(
        calls: Arc<Mutex<Vec<String>>>,
        deny_resource: bool,
        deny_org_wide: bool,
        platform_grant: bool,
    ) -> MockAuthApp {
        let mut auth = MockAuthApp::new();
        auth.expect_check_policy_for_resource().returning({
            let calls = calls.clone();
            move |input| {
                let operator = input.multi_tenancy.operator_id();
                calls.lock().unwrap().push(format!(
                    "resource:{}:{}",
                    input.action, input.resource_trn
                ));
                let platform_grant = platform_grant
                    && operator.as_ref() == Some(&*LIBRARY_TENANT);
                Box::pin(async move {
                    if deny_resource && !platform_grant {
                        Err(errors::Error::forbidden("denied"))
                    } else {
                        Ok(())
                    }
                })
            }
        });
        auth.expect_check_policy().returning({
            let calls = calls.clone();
            move |input| {
                calls
                    .lock()
                    .unwrap()
                    .push(format!("policy:{}", input.action));
                Box::pin(async move {
                    if deny_org_wide {
                        Err(errors::Error::forbidden("denied"))
                    } else {
                        Ok(())
                    }
                })
            }
        });
        auth
    }

    #[tokio::test]
    async fn denies_private_repo_without_resource_or_org_wide_permission() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let auth = mock_auth(calls.clone(), true, true);
        let executor = create_test_executor();
        let multi_tenancy = create_test_multi_tenancy();

        let error = authorize_private_repo_read(
            &auth,
            &executor,
            &multi_tenancy,
            &multi_tenancy.get_operator_id().unwrap(),
            "rp_01test",
        )
        .await
        .unwrap_err();

        assert!(matches!(error, errors::Error::Forbidden { .. }));
        assert_eq!(
            calls.lock().unwrap().as_slice(),
            &[
                "resource:library:ViewRepo:trn:library:repo:rp_01test",
                "policy:library:ViewPrivateRepo"
            ]
        );
    }

    #[tokio::test]
    async fn resource_access_allows_without_org_wide_fallback() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let auth = mock_auth(calls.clone(), false, false);
        let executor = create_test_executor();
        let multi_tenancy = create_test_multi_tenancy();

        authorize_private_repo_read(
            &auth,
            &executor,
            &multi_tenancy,
            &multi_tenancy.get_operator_id().unwrap(),
            "rp_01test",
        )
        .await
        .unwrap();

        assert_eq!(
            calls.lock().unwrap().as_slice(),
            &["resource:library:ViewRepo:trn:library:repo:rp_01test"]
        );
    }

    #[tokio::test]
    async fn falls_back_to_org_wide_private_repo_policy() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let auth = mock_auth(calls.clone(), true, false);
        let executor = create_test_executor();
        let multi_tenancy = create_test_multi_tenancy();

        authorize_private_repo_read(
            &auth,
            &executor,
            &multi_tenancy,
            &multi_tenancy.get_operator_id().unwrap(),
            "rp_01test",
        )
        .await
        .unwrap();

        assert_eq!(
            calls.lock().unwrap().as_slice(),
            &[
                "resource:library:ViewRepo:trn:library:repo:rp_01test",
                "policy:library:ViewPrivateRepo"
            ]
        );
    }

    fn platform_header() -> MultiTenancy {
        MultiTenancy::new(
            Some(LIBRARY_TENANT.clone()),
            Some(LIBRARY_TENANT.clone()),
        )
    }

    fn org() -> value_object::TenantId {
        "tn_01hjryxysgey07h5jz5wagqj0b".parse().unwrap()
    }

    #[tokio::test]
    async fn platform_header_is_checked_in_the_organization_tenant() {
        let operators = Arc::new(Mutex::new(Vec::new()));
        let mut auth = MockAuthApp::new();
        auth.expect_check_policy_for_resource().returning({
            let operators = operators.clone();
            move |input| {
                operators
                    .lock()
                    .unwrap()
                    .push(input.multi_tenancy.operator_id());
                Box::pin(async { Err(errors::Error::forbidden("denied")) })
            }
        });
        auth.expect_check_policy().returning({
            let operators = operators.clone();
            move |input| {
                operators
                    .lock()
                    .unwrap()
                    .push(input.multi_tenancy.operator_id());
                // The platform tenant would allow anyone.
                let allowed = input.multi_tenancy.operator_id().as_ref()
                    == Some(&*LIBRARY_TENANT);
                Box::pin(async move {
                    if allowed {
                        Ok(())
                    } else {
                        Err(errors::Error::forbidden("denied"))
                    }
                })
            }
        });

        let error = authorize_private_repo_read(
            &auth,
            &create_test_executor(),
            &platform_header(),
            &org(),
            "rp_01test",
        )
        .await
        .unwrap_err();

        assert!(matches!(error, errors::Error::Forbidden { .. }));
        assert_eq!(
            operators.lock().unwrap().as_slice(),
            &[Some(org()), Some(org()), Some(LIBRARY_TENANT.clone()),],
            "ViewRepo and ViewPrivateRepo in the organization, then only \
             the repository grant in the platform tenant"
        );
    }

    #[tokio::test]
    async fn platform_header_passes_with_a_repository_grant_there() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let auth =
            mock_auth_with_platform_grant(calls.clone(), true, true, true);

        authorize_private_repo_read(
            &auth,
            &create_test_executor(),
            &platform_header(),
            &org(),
            "rp_01test",
        )
        .await
        .unwrap();

        assert_eq!(
            calls.lock().unwrap().as_slice(),
            &[
                "resource:library:ViewRepo:trn:library:repo:rp_01test",
                "policy:library:ViewPrivateRepo",
                "resource:library:ViewRepo:trn:library:repo:rp_01test",
            ]
        );
    }

    #[tokio::test]
    async fn organization_header_has_no_platform_fallback() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let auth =
            mock_auth_with_platform_grant(calls.clone(), true, true, true);

        authorize_private_repo_read(
            &auth,
            &create_test_executor(),
            &MultiTenancy::new(None, Some(org())),
            &org(),
            "rp_01test",
        )
        .await
        .unwrap_err();

        assert_eq!(calls.lock().unwrap().len(), 2);
    }
}
