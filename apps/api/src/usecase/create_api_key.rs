use std::sync::Arc;

use derive_new::new;
use tachyon_sdk::auth::PublicApiKey;
use tachyon_sdk::auth::{
    AttachSaPolicyInput, AuthApp, CheckPolicyInput,
    CreatePublicApiKeyInput, CreateServiceAccountInput,
    DeleteServiceAccountInput, GetServiceAccountByNameInput,
    ServiceAccount,
};
use value_object::{Identifier, TenantId};

use tachyon_sdk::auth::MultiTenancy;

use super::api_key_issuer::grant_api_key_policy;
use super::GetOrganizationByUsernameQuery;
use crate::domain::{
    library_api_key_accounts_policy_id, library_api_key_issuer_policy_id,
    ApiKeyRole, ApiKeyServiceAccount, LEGACY_API_KEY_SERVICE_ACCOUNT_NAME,
    LIBRARY_TENANT,
};

#[derive(Debug, Clone)]
pub struct CreateApiKeyInputData<'a> {
    pub executor: &'a dyn tachyon_sdk::auth::ExecutorAction,
    pub multi_tenancy: &'a dyn tachyon_sdk::auth::MultiTenancyAction,

    pub org_name: &'a Identifier,
    pub name: &'a str,
    /// Repository access the key gets. `None` issues a key that reaches
    /// public repositories only.
    pub role: Option<ApiKeyRole>,
}

#[derive(Debug, Clone)]
pub struct CreateApiKeyOutputData {
    pub api_key: PublicApiKey,
    /// The key's own account. Internal: Library does not surface it.
    pub service_account: ServiceAccount,
    pub role: Option<ApiKeyRole>,
}

#[derive(Debug, Clone, new)]
pub struct CreateApiKey {
    auth_app: Arc<dyn AuthApp>,
    get_org_by_name: Arc<dyn GetOrganizationByUsernameQuery>,
}

#[async_trait::async_trait]
pub trait CreateApiKeyInputPort: std::fmt::Debug + Send + Sync {
    async fn execute<'a>(
        &self,
        input: &CreateApiKeyInputData<'a>,
    ) -> errors::Result<CreateApiKeyOutputData>;
}

#[async_trait::async_trait]
impl CreateApiKeyInputPort for CreateApiKey {
    #[tracing::instrument(name = "CreateApiKey::execute", skip(self))]
    async fn execute<'a>(
        &self,
        input: &CreateApiKeyInputData<'a>,
    ) -> errors::Result<CreateApiKeyOutputData> {
        let organization = self
            .get_org_by_name
            .execute(&input.org_name.to_string().parse()?)
            .await?
            .ok_or(errors::not_found!("Organization not found"))?;
        let tenant_id: TenantId = organization.id().to_string().parse()?;

        // Everything about the role is decided in the organization's own
        // tenant, whatever operator the request named: that is where an
        // owner's repository policy is attached and the only scope a
        // check reads, and the key's account lives there too, which
        // tachyon insists on for a grant. The v1 web client acts as the
        // Library platform, so reading the caller's scope would refuse
        // every owner.
        let org_scope = MultiTenancy::new(
            Some(LIBRARY_TENANT.clone()),
            Some(tenant_id.clone()),
        );

        // A key issuing a key is a key acting for whoever holds it, and
        // `library:CreateApiKey` is a person's permission: an owner key
        // is recognised by the repository policy it carries instead. It
        // may issue any role, being the widest itself. Checked before
        // anything is created so a refusal leaves nothing behind.
        if input.executor.is_service_account() {
            self.auth_app
                .check_policy(&CheckPolicyInput {
                    executor: input.executor,
                    multi_tenancy: &org_scope,
                    action: "library:ManageRepoPolicy",
                })
                .await?;
        } else {
            self.auth_app
                .check_policy(&CheckPolicyInput {
                    executor: input.executor,
                    multi_tenancy: input.multi_tenancy,
                    action: "library:CreateApiKey",
                })
                .await?;
        }

        // Handing a key repository access is granting repository
        // permissions, which only those who manage repository policy may
        // do.
        if input.role.is_some() {
            if !input.executor.is_service_account() {
                self.auth_app
                    .check_policy(&CheckPolicyInput {
                        executor: input.executor,
                        multi_tenancy: &org_scope,
                        action: "library:ManageRepoPolicy",
                    })
                    .await?;
            }

            // Granting the key's account its role is authorized on the
            // tachyon side as the caller, who needs the issuer grant for
            // it. The owner check above has passed by now. Best effort:
            // see `grant_api_key_policy`.
            grant_api_key_policy(
                self.auth_app.as_ref(),
                &library_api_key_issuer_policy_id(),
                input.executor,
                &org_scope,
                &tenant_id,
            )
            .await?;
        }

        // Making the account is part of issuing any key, including one
        // with no repository access, whose holder need not be an owner.
        grant_api_key_policy(
            self.auth_app.as_ref(),
            &library_api_key_accounts_policy_id(),
            input.executor,
            &org_scope,
            &tenant_id,
        )
        .await?;

        // Every key gets an account of its own, so what is granted here
        // reaches this key and no other (see `ApiKeyServiceAccount`).
        let service_account = self
            .service_account_for_key(input, &org_scope, &tenant_id)
            .await?;

        match self
            .grant_and_issue(
                input,
                &org_scope,
                &organization,
                &service_account,
            )
            .await
        {
            Ok(api_key) => Ok(CreateApiKeyOutputData {
                api_key,
                service_account,
                role: input.role,
            }),
            Err(error) => {
                self.discard_service_account(
                    input,
                    &org_scope,
                    &service_account,
                )
                .await;
                Err(error)
            }
        }
    }
}

impl CreateApiKey {
    /// The account the key is issued on: its own, or, for a key that
    /// carries no access and whose holder may not make one, the shared
    /// account every key used to be issued on.
    ///
    /// Creating an account is an organization owner's to do, and a key
    /// without a role is not. Such a key grants nothing, so where it
    /// lives decides nothing either, and the shared account -- which
    /// carries no policy -- is where keys like it have always gone. A key
    /// with a role never falls back: its access has to reach that key
    /// alone.
    async fn service_account_for_key<'a>(
        &self,
        input: &CreateApiKeyInputData<'a>,
        org_scope: &MultiTenancy,
        tenant_id: &TenantId,
    ) -> errors::Result<ServiceAccount> {
        let own = self
            .auth_app
            .create_service_account(&CreateServiceAccountInput {
                executor: input.executor,
                multi_tenancy: org_scope,
                tenant_id,
                name: &ApiKeyServiceAccount::new_name(input.role),
            })
            .await;

        let error = match own {
            Ok(service_account) => return Ok(service_account),
            Err(error) if input.role.is_some() => return Err(error),
            Err(error) => error,
        };

        tracing::info!(
            error = %error,
            "no account of its own for a key without a role; using the shared one"
        );

        self.auth_app
            .get_service_account_by_name(&GetServiceAccountByNameInput {
                executor: input.executor,
                multi_tenancy: org_scope,
                tenant_id,
                name: LEGACY_API_KEY_SERVICE_ACCOUNT_NAME,
            })
            .await?
            .ok_or(error)
    }

    /// Grant before issuing, so a refused grant never hands out a key
    /// without the access it promised.
    async fn grant_and_issue<'a>(
        &self,
        input: &CreateApiKeyInputData<'a>,
        org_scope: &MultiTenancy,
        organization: &crate::domain::Organization,
        service_account: &ServiceAccount,
    ) -> errors::Result<PublicApiKey> {
        if let Some(role) = input.role {
            self.auth_app
                .attach_sa_policy(&AttachSaPolicyInput {
                    executor: input.executor,
                    multi_tenancy: org_scope,
                    service_account_id: service_account.id(),
                    policy_id: &role.policy_id(),
                })
                .await?;

            // An owner key issues keys of its own (see the check in
            // `execute`), which upstream authorizes as the key: the
            // account it acts as needs what that takes, the way a person
            // is granted it on use.
            if role == ApiKeyRole::Owner {
                for policy_id in [
                    library_api_key_accounts_policy_id(),
                    library_api_key_issuer_policy_id(),
                ] {
                    self.auth_app
                        .attach_sa_policy(&AttachSaPolicyInput {
                            executor: input.executor,
                            multi_tenancy: org_scope,
                            service_account_id: service_account.id(),
                            policy_id: &policy_id,
                        })
                        .await?;
                }
            }
        }

        self.auth_app
            .create_public_api_key(&CreatePublicApiKeyInput {
                executor: input.executor,
                multi_tenancy: org_scope,
                operator_id: organization.id(),
                service_account_id: service_account.id(),
                name: input.name,
            })
            .await
    }

    /// Best effort: an account left behind holds no key, so it is neither
    /// listed nor usable, and the caller's error is the one worth
    /// reporting.
    ///
    /// Removing an account is an owner's to do, and only a key with a
    /// role has established that the caller is one. Without that, asking
    /// upstream would only be refused, so the empty account is left
    /// behind and said so.
    async fn discard_service_account<'a>(
        &self,
        input: &CreateApiKeyInputData<'a>,
        org_scope: &MultiTenancy,
        service_account: &ServiceAccount,
    ) {
        if input.role.is_none() {
            // It may be the shared account, which is not this key's to
            // remove, and an account of its own is empty and inert.
            tracing::info!(
                service_account = %service_account.id(),
                "left the service account of a key that was not issued"
            );
            return;
        }

        // The account may already carry the role, and one that does is
        // one a key can be minted on, so it goes first and on its own
        // terms.
        if let Some(role) = input.role {
            if let Err(error) = self
                .auth_app
                .detach_sa_policy(&AttachSaPolicyInput {
                    executor: input.executor,
                    multi_tenancy: org_scope,
                    service_account_id: service_account.id(),
                    policy_id: &role.policy_id(),
                })
                .await
            {
                tracing::warn!(
                    service_account = %service_account.id(),
                    error = %error,
                    "could not take the role off the account of a key that was not issued"
                );
            }
        }

        if let Err(error) = self
            .auth_app
            .delete_service_account(&DeleteServiceAccountInput {
                executor: input.executor,
                multi_tenancy: org_scope,
                service_account_id: service_account.id(),
            })
            .await
        {
            tracing::warn!(
                service_account = %service_account.id(),
                error = %error,
                "could not remove the service account of a key that was not issued"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        Organization, LIBRARY_API_KEY_ACCOUNTS_POLICY_ID,
        LIBRARY_API_KEY_ISSUER_POLICY_ID,
    };
    use async_trait::async_trait;
    use mockall::mock;
    use std::str::FromStr;
    use std::sync::Mutex;
    use tachyon_sdk::auth::{
        test_helper::create_test_multi_tenancy, MockAuthApp,
        PublicApiKeyId, PublicApiKeyValue,
    };
    use value_object::{TenantId, Text};

    mock! {
        #[derive(Debug)]
        GetOrgByUsername {}
        #[async_trait]
        impl GetOrganizationByUsernameQuery for GetOrgByUsername {
            async fn execute(&self, username: &Identifier) -> errors::Result<Option<Organization>>;
        }
    }

    /// A signed-in person: the issuer grant is given to users only.
    fn user_executor() -> tachyon_sdk::auth::Executor {
        let now = chrono::Utc::now();
        tachyon_sdk::auth::Executor::User(Box::new(
            tachyon_sdk::auth::User {
                id: tachyon_sdk::auth::UserId::new(
                    "us_01hs2yepy5hw4rz8pdq2wywnwt",
                )
                .unwrap(),
                username: "owner".to_string(),
                tenants: vec![TenantId::default()],
                email: None,
                name: None,
                email_verified: None,
                image: None,
                role: tachyon_sdk::auth::DefaultRole::Owner,
                metadata: None,
                created_at: now,
                updated_at: now,
            },
        ))
    }

    type Calls = Arc<Mutex<Vec<String>>>;

    /// An upstream that records every call and refuses `deny_action`.
    fn auth(calls: &Calls, deny: Option<&'static str>) -> MockAuthApp {
        let tenant_id = TenantId::default();
        let mut auth = MockAuthApp::new();
        auth.expect_check_policy().returning({
            let calls = calls.clone();
            move |input| {
                calls.lock().unwrap().push(format!(
                    "policy:{}@{}",
                    input.action,
                    input
                        .multi_tenancy
                        .get_operator_id()
                        .map(|id| id.to_string())
                        .unwrap_or_default()
                ));
                let denied = deny == Some(input.action);
                Box::pin(async move {
                    if denied {
                        Err(errors::Error::forbidden("denied"))
                    } else {
                        Ok(())
                    }
                })
            }
        });
        auth.expect_attach_user_policy().returning({
            let calls = calls.clone();
            move |input| {
                let denied = deny == Some("grant");
                calls.lock().unwrap().push(format!(
                    "grant:{}@{}",
                    input.policy_id,
                    input
                        .multi_tenancy
                        .get_operator_id()
                        .map(|id| id.to_string())
                        .unwrap_or_default()
                ));
                Box::pin(async move {
                    if denied {
                        Err(errors::Error::forbidden("denied"))
                    } else {
                        Ok(())
                    }
                })
            }
        });
        auth.expect_get_service_account_by_name().returning({
            let calls = calls.clone();
            let tenant_id = tenant_id.clone();
            move |input| {
                calls
                    .lock()
                    .unwrap()
                    .push(format!("find-sa:{}", input.name));
                let sa = ServiceAccount {
                    id: "sa_01default".to_string().into(),
                    tenant_id: tenant_id.clone(),
                    name: input.name.to_string(),
                    created_at: chrono::Utc::now(),
                };
                Box::pin(async move { Ok(Some(sa)) })
            }
        });
        auth.expect_create_service_account().returning({
            let calls = calls.clone();
            let tenant_id = tenant_id.clone();
            move |input| {
                // The suffix is random; the role segment is what matters.
                let role = ApiKeyServiceAccount::from_name(input.name)
                    .and_then(ApiKeyServiceAccount::role);
                calls.lock().unwrap().push(format!("sa:{role:?}"));
                if deny == Some("sa") {
                    return Box::pin(async {
                        Err(errors::Error::forbidden("denied"))
                    });
                }
                let sa = ServiceAccount {
                    id: "sa_01key".to_string().into(),
                    tenant_id: tenant_id.clone(),
                    name: input.name.to_string(),
                    created_at: chrono::Utc::now(),
                };
                Box::pin(async move { Ok(sa) })
            }
        });
        auth.expect_delete_service_account().returning({
            let calls = calls.clone();
            move |input| {
                calls.lock().unwrap().push(format!(
                    "delete-sa:{}",
                    input.service_account_id.as_str()
                ));
                Box::pin(async { Ok(()) })
            }
        });
        auth.expect_attach_sa_policy().returning({
            let calls = calls.clone();
            move |input| {
                calls.lock().unwrap().push(format!(
                    "attach:{}:{}@{}",
                    input.service_account_id.as_str(),
                    input.policy_id,
                    input
                        .multi_tenancy
                        .get_operator_id()
                        .map(|id| id.to_string())
                        .unwrap_or_default()
                ));
                let denied = deny == Some("attach");
                Box::pin(async move {
                    if denied {
                        Err(errors::Error::forbidden("denied"))
                    } else {
                        Ok(())
                    }
                })
            }
        });
        auth.expect_detach_sa_policy().returning({
            let calls = calls.clone();
            move |input| {
                calls.lock().unwrap().push(format!(
                    "detach:{}:{}",
                    input.service_account_id.as_str(),
                    input.policy_id
                ));
                Box::pin(async { Ok(()) })
            }
        });
        auth.expect_create_public_api_key().returning({
            let calls = calls.clone();
            move |input| {
                calls.lock().unwrap().push(format!(
                    "key:{}",
                    input.service_account_id.as_str()
                ));
                let key = PublicApiKey {
                    id: PublicApiKeyId::new("pak_01test"),
                    tenant_id: input.operator_id.clone(),
                    service_account_id: input.service_account_id.clone(),
                    name: input.name.to_string(),
                    value: PublicApiKeyValue::new("pk_secret"),
                    created_at: chrono::Utc::now(),
                };
                Box::pin(async move { Ok(key) })
            }
        });
        auth
    }

    /// A key acting for whoever holds it.
    fn key_executor() -> tachyon_sdk::auth::Executor {
        tachyon_sdk::auth::Executor::ServiceAccount(Box::new(
            ServiceAccount {
                id: "sa_01owner".to_string().into(),
                tenant_id: org_tenant(),
                name:
                    "library-api-key-owner-0123456789abcdef0123456789abcdef"
                        .to_string(),
                created_at: chrono::Utc::now(),
            },
        ))
    }

    /// The tenant the request names, which the v1 web client sets to the
    /// Library platform rather than to the organization.
    const CALLER_TENANT: &str = "tn_01hjryxysgey07h5jz5wagqj0m";

    fn org_tenant() -> TenantId {
        TenantId::new("tn_01organization00000000000").unwrap()
    }

    async fn create(
        auth: MockAuthApp,
        role: Option<ApiKeyRole>,
    ) -> errors::Result<CreateApiKeyOutputData> {
        create_as(auth, role, user_executor()).await
    }

    async fn create_as(
        auth: MockAuthApp,
        role: Option<ApiKeyRole>,
        executor: tachyon_sdk::auth::Executor,
    ) -> errors::Result<CreateApiKeyOutputData> {
        let mut get_org = MockGetOrgByUsername::new();
        get_org.expect_execute().returning(|_| {
            Ok(Some(Organization::new(
                &org_tenant(),
                &Text::new("Test Organization").unwrap(),
                &Identifier::from_str("test-org").unwrap(),
                None,
                None,
            )))
        });
        let multi_tenancy = create_test_multi_tenancy();
        CreateApiKey::new(Arc::new(auth), Arc::new(get_org))
            .execute(&CreateApiKeyInputData {
                executor: &executor,
                multi_tenancy: &multi_tenancy,
                org_name: &Identifier::from_str("test-org").unwrap(),
                name: "ci",
                role,
            })
            .await
    }

    #[tokio::test]
    async fn a_role_key_gets_its_policy_on_its_own_service_account() {
        let calls = Calls::default();
        let output = create(auth(&calls, None), Some(ApiKeyRole::Reader))
            .await
            .unwrap();

        assert_eq!(output.role, Some(ApiKeyRole::Reader));
        assert_eq!(
            calls.lock().unwrap().as_slice(),
            [
                // Only the first check reads the caller's own scope;
                // everything about the role is decided in the
                // organization's tenant, whatever operator was named.
                format!("policy:library:CreateApiKey@{CALLER_TENANT}"),
                format!("policy:library:ManageRepoPolicy@{}", org_tenant()),
                // The caller gets what granting the account needs.
                format!(
                    "grant:{}@{}",
                    LIBRARY_API_KEY_ISSUER_POLICY_ID,
                    org_tenant()
                ),
                format!(
                    "grant:{}@{}",
                    LIBRARY_API_KEY_ACCOUNTS_POLICY_ID,
                    org_tenant()
                ),
                "sa:Some(Reader)".to_string(),
                format!(
                    "attach:sa_01key:pol_01libraryreporeader@{}",
                    org_tenant()
                ),
                "key:sa_01key".to_string(),
            ]
        );
    }

    #[tokio::test]
    async fn a_key_without_a_role_gets_its_own_account_ungranted() {
        let calls = Calls::default();
        let output = create(auth(&calls, None), None).await.unwrap();

        assert_eq!(output.role, None);
        assert_eq!(
            calls.lock().unwrap().as_slice(),
            [
                format!("policy:library:CreateApiKey@{CALLER_TENANT}"),
                // A key without a role still needs its own account, and
                // its holder need not be an owner.
                format!(
                    "grant:{}@{}",
                    LIBRARY_API_KEY_ACCOUNTS_POLICY_ID,
                    org_tenant()
                ),
                "sa:None".to_string(),
                "key:sa_01key".to_string(),
            ]
        );
    }

    /// A key issues keys when it carries the owner role, which is what
    /// the repository policy check recognises; `library:CreateApiKey`
    /// belongs to people. The account it makes gets what issuing takes,
    /// so a key it issues as an owner can issue in turn.
    #[tokio::test]
    async fn an_owner_key_issues_keys_of_its_own() {
        let calls = Calls::default();
        let output = create_as(
            auth(&calls, None),
            Some(ApiKeyRole::Owner),
            key_executor(),
        )
        .await
        .unwrap();

        assert_eq!(output.role, Some(ApiKeyRole::Owner));
        let calls = calls.lock().unwrap();
        assert_eq!(
            calls.first().unwrap(),
            &format!("policy:library:ManageRepoPolicy@{}", org_tenant())
        );
        assert!(!calls
            .iter()
            .any(|call| call.contains("library:CreateApiKey")));
        // A key cannot attach a policy to itself, so nothing is granted
        // to it; what it needs is on the account it acts as already.
        assert!(!calls.iter().any(|call| call.starts_with("grant:")));
        assert!(calls.iter().any(|call| call
            == &format!(
                "attach:sa_01key:{LIBRARY_API_KEY_ACCOUNTS_POLICY_ID}@{}",
                org_tenant()
            )));
        assert!(calls.iter().any(|call| call
            == &format!(
                "attach:sa_01key:{LIBRARY_API_KEY_ISSUER_POLICY_ID}@{}",
                org_tenant()
            )));
    }

    /// A key that carries no repository policy is refused, which is every
    /// key but an owner's.
    #[tokio::test]
    async fn a_key_without_the_owner_role_issues_nothing() {
        let calls = Calls::default();
        let result = create_as(
            auth(&calls, Some("library:ManageRepoPolicy")),
            None,
            key_executor(),
        )
        .await;

        assert!(matches!(result, Err(errors::Error::Forbidden { .. })));
        assert_eq!(
            calls.lock().unwrap().as_slice(),
            [format!("policy:library:ManageRepoPolicy@{}", org_tenant())]
        );
    }

    /// Attaching a policy to oneself is itself an owner's to do, so a
    /// member who may issue a key without a role is refused it. What they
    /// may do is decided by the operation that follows, not by the grant.
    #[tokio::test]
    async fn a_refused_grant_does_not_stop_the_key() {
        let calls = Calls::default();
        let output =
            create(auth(&calls, Some("grant")), None).await.unwrap();

        assert_eq!(output.role, None);
        assert!(calls
            .lock()
            .unwrap()
            .iter()
            .any(|call| call.starts_with("key:")));
    }

    /// Making an account is an owner's to do, and a key without a role is
    /// not: it goes where keys like it have always gone, which grants it
    /// nothing it would not have had.
    #[tokio::test]
    async fn a_key_without_a_role_falls_back_to_the_shared_account() {
        let calls = Calls::default();
        let output = create(auth(&calls, Some("sa")), None).await.unwrap();

        assert_eq!(output.role, None);
        assert_eq!(
            calls.lock().unwrap().as_slice(),
            [
                format!("policy:library:CreateApiKey@{CALLER_TENANT}"),
                format!(
                    "grant:{}@{}",
                    LIBRARY_API_KEY_ACCOUNTS_POLICY_ID,
                    org_tenant()
                ),
                "sa:None".to_string(),
                "find-sa:default".to_string(),
                "key:sa_01default".to_string(),
            ]
        );
    }

    /// A key with a role has to reach that key alone, so it never shares.
    #[tokio::test]
    async fn a_role_key_is_refused_rather_than_sharing_an_account() {
        let calls = Calls::default();
        let result =
            create(auth(&calls, Some("sa")), Some(ApiKeyRole::Writer))
                .await;

        assert!(matches!(result, Err(errors::Error::Forbidden { .. })));
        let calls = calls.lock().unwrap();
        assert!(!calls.iter().any(|call| call.starts_with("find-sa:")));
        assert!(!calls.iter().any(|call| call.starts_with("key:")));
    }

    #[tokio::test]
    async fn granting_a_role_needs_repo_policy_management() {
        let calls = Calls::default();
        let result = create(
            auth(&calls, Some("library:ManageRepoPolicy")),
            Some(ApiKeyRole::Owner),
        )
        .await;

        assert!(matches!(result, Err(errors::Error::Forbidden { .. })));
        assert_eq!(
            calls.lock().unwrap().as_slice(),
            [
                format!("policy:library:CreateApiKey@{CALLER_TENANT}"),
                format!("policy:library:ManageRepoPolicy@{}", org_tenant())
            ]
        );
    }

    #[tokio::test]
    async fn a_refused_grant_issues_no_key() {
        let calls = Calls::default();
        let result =
            create(auth(&calls, Some("attach")), Some(ApiKeyRole::Writer))
                .await;

        assert!(matches!(result, Err(errors::Error::Forbidden { .. })));
        let calls = calls.lock().unwrap();
        assert!(!calls.iter().any(|call| call.starts_with("key:")));
        // The account made for the key does not outlive the failure.
        assert_eq!(calls.last().unwrap(), "delete-sa:sa_01key");
    }
}
