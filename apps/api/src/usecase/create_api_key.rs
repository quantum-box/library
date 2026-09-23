use std::sync::Arc;

use derive_new::new;
use tachyon_sdk::auth::{
    AttachSaPolicyInput, AuthApp, CheckPolicyInput,
    CreatePublicApiKeyInput, CreateServiceAccountInput,
    DeleteServiceAccountInput, ServiceAccount,
};
use tachyon_sdk::auth::{PolicyId, PublicApiKey};
use value_object::{Identifier, TenantId};

use tachyon_sdk::auth::MultiTenancy;

use super::api_key_issuer::grant_api_key_policy;
use super::GetOrganizationByUsernameQuery;
use crate::domain::{ApiKeyRole, ApiKeyServiceAccount, LIBRARY_TENANT};

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
    /// See `library_api_key_issuer_policy_id`.
    api_key_issuer_policy_id: Option<PolicyId>,
    /// See `library_api_key_accounts_policy_id`.
    api_key_accounts_policy_id: Option<PolicyId>,
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
        self.auth_app
            .check_policy(&CheckPolicyInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                action: "library:CreateApiKey",
            })
            .await?;

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

        // Handing a key repository access is granting repository
        // permissions, which only those who manage repository policy may
        // do. Checked before anything is created so a refusal leaves
        // nothing behind.
        if input.role.is_some() {
            self.auth_app
                .check_policy(&CheckPolicyInput {
                    executor: input.executor,
                    multi_tenancy: &org_scope,
                    action: "library:ManageRepoPolicy",
                })
                .await?;

            // Granting the key's account its role is authorized on the
            // tachyon side as the caller, who needs the issuer grant for
            // it. The owner check above has passed by now. Best effort:
            // see `grant_api_key_policy`.
            grant_api_key_policy(
                self.auth_app.as_ref(),
                self.api_key_issuer_policy_id.as_ref(),
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
            self.api_key_accounts_policy_id.as_ref(),
            input.executor,
            &org_scope,
            &tenant_id,
        )
        .await?;

        // Every key gets an account of its own, so what is granted here
        // reaches this key and no other (see `ApiKeyServiceAccount`).
        let service_account = self
            .auth_app
            .create_service_account(&CreateServiceAccountInput {
                executor: input.executor,
                multi_tenancy: &org_scope,
                tenant_id: &tenant_id,
                name: &ApiKeyServiceAccount::new_name(input.role),
            })
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
            tracing::info!(
                service_account = %service_account.id(),
                "left the empty service account of a key that was not issued"
            );
            return;
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
    use crate::domain::Organization;
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
        auth.expect_create_service_account().returning({
            let calls = calls.clone();
            let tenant_id = tenant_id.clone();
            move |input| {
                // The suffix is random; the role segment is what matters.
                let role = ApiKeyServiceAccount::from_name(input.name)
                    .and_then(ApiKeyServiceAccount::role);
                calls.lock().unwrap().push(format!("sa:{role:?}"));
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
        let executor = user_executor();
        let multi_tenancy = create_test_multi_tenancy();
        CreateApiKey::new(
            Arc::new(auth),
            Arc::new(get_org),
            Some(PolicyId::new("pol_01issuer")),
            Some(PolicyId::new("pol_01accounts")),
        )
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
                format!("grant:pol_01issuer@{}", org_tenant()),
                format!("grant:pol_01accounts@{}", org_tenant()),
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
                format!("grant:pol_01accounts@{}", org_tenant()),
                "sa:None".to_string(),
                "key:sa_01key".to_string(),
            ]
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
