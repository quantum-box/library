use std::sync::Arc;

use derive_new::new;
use tachyon_sdk::auth::{
    AttachSaPolicyInput, AuthApp, CheckPolicyInput,
    CreatePublicApiKeyInput, CreateServiceAccountInput,
    DeleteServiceAccountInput, ServiceAccount,
};
use tachyon_sdk::auth::{PolicyId, PublicApiKey};
use value_object::{Identifier, TenantId};

use super::api_key_issuer::grant_api_key_issuer;
use super::GetOrganizationByUsernameQuery;
use crate::domain::{ApiKeyRole, ApiKeyServiceAccount};

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

        // Handing a key repository access is granting repository
        // permissions, which only those who manage repository policy may
        // do. Checked before anything is created so a refusal leaves
        // nothing behind.
        if input.role.is_some() {
            self.auth_app
                .check_policy(&CheckPolicyInput {
                    executor: input.executor,
                    multi_tenancy: input.multi_tenancy,
                    action: "library:ManageRepoPolicy",
                })
                .await?;
        }

        let organization = self
            .get_org_by_name
            .execute(&input.org_name.to_string().parse()?)
            .await?
            .ok_or(errors::not_found!("Organization not found"))?;
        let tenant_id: TenantId = organization.id().to_string().parse()?;

        // Granting the key's account its role is authorized on the tachyon
        // side as the caller, who needs the issuer grant for it. Only a
        // role needs it; the owner check above has passed by now.
        if input.role.is_some() {
            grant_api_key_issuer(
                self.auth_app.as_ref(),
                self.api_key_issuer_policy_id.as_ref(),
                input.executor,
                input.multi_tenancy,
                &tenant_id,
            )
            .await?;
        }

        // Every key gets an account of its own, so what is granted here
        // reaches this key and no other (see `ApiKeyServiceAccount`).
        let service_account = self
            .auth_app
            .create_service_account(&CreateServiceAccountInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                tenant_id: &tenant_id,
                name: &ApiKeyServiceAccount::new_name(input.role),
            })
            .await?;

        match self
            .grant_and_issue(input, &organization, &service_account)
            .await
        {
            Ok(api_key) => Ok(CreateApiKeyOutputData {
                api_key,
                service_account,
                role: input.role,
            }),
            Err(error) => {
                self.discard_service_account(input, &service_account).await;
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
        organization: &crate::domain::Organization,
        service_account: &ServiceAccount,
    ) -> errors::Result<PublicApiKey> {
        if let Some(role) = input.role {
            self.auth_app
                .attach_sa_policy(&AttachSaPolicyInput {
                    executor: input.executor,
                    multi_tenancy: input.multi_tenancy,
                    service_account_id: service_account.id(),
                    policy_id: &role.policy_id(),
                })
                .await?;
        }

        self.auth_app
            .create_public_api_key(&CreatePublicApiKeyInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                operator_id: organization.id(),
                service_account_id: service_account.id(),
                name: input.name,
            })
            .await
    }

    /// Best effort: an account left behind holds no key, so it is neither
    /// listed nor usable, and the caller's error is the one worth
    /// reporting.
    async fn discard_service_account<'a>(
        &self,
        input: &CreateApiKeyInputData<'a>,
        service_account: &ServiceAccount,
    ) {
        if let Err(error) = self
            .auth_app
            .delete_service_account(&DeleteServiceAccountInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
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
                calls
                    .lock()
                    .unwrap()
                    .push(format!("policy:{}", input.action));
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
                calls
                    .lock()
                    .unwrap()
                    .push(format!("grant:{}", input.policy_id));
                Box::pin(async { Ok(()) })
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
                    "attach:{}:{}",
                    input.service_account_id.as_str(),
                    input.policy_id
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

    async fn create(
        auth: MockAuthApp,
        role: Option<ApiKeyRole>,
    ) -> errors::Result<CreateApiKeyOutputData> {
        let mut get_org = MockGetOrgByUsername::new();
        get_org.expect_execute().returning(|_| {
            Ok(Some(Organization::new(
                &TenantId::default(),
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
                "policy:library:CreateApiKey",
                "policy:library:ManageRepoPolicy",
                // The caller gets what granting the account needs.
                "grant:pol_01issuer",
                "sa:Some(Reader)",
                "attach:sa_01key:pol_01libraryreporeader",
                "key:sa_01key",
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
            ["policy:library:CreateApiKey", "sa:None", "key:sa_01key"]
        );
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
                "policy:library:CreateApiKey",
                "policy:library:ManageRepoPolicy"
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
