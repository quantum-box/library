use std::sync::Arc;

use derive_new::new;
use futures_util::{StreamExt, TryStreamExt};
use tachyon_sdk::auth::PublicApiKey;
use tachyon_sdk::auth::{
    AuthApp, CheckPolicyInput, FindAllPublicApiKeyInput,
    FindAllServiceAccountsInput,
};
use value_object::{Identifier, TenantId};

use tachyon_sdk::auth::MultiTenancy;

use super::GetOrganizationByUsernameQuery;
use crate::domain::{ApiKeyRole, ApiKeyServiceAccount, LIBRARY_TENANT};

#[derive(Debug, Clone)]
pub struct ListApiKeysInputData<'a> {
    pub executor: &'a dyn tachyon_sdk::auth::ExecutorAction,
    pub multi_tenancy: &'a dyn tachyon_sdk::auth::MultiTenancyAction,

    pub org_name: &'a Identifier,
}

/// A listed key and the role it was issued with.
#[derive(Debug, Clone)]
pub struct ListedApiKey {
    pub api_key: PublicApiKey,
    pub role: Option<ApiKeyRole>,
}

#[derive(Debug, Clone, new)]
pub struct ListApiKeys {
    auth_app: Arc<dyn AuthApp>,
    get_org_by_name: Arc<dyn GetOrganizationByUsernameQuery>,
}

#[async_trait::async_trait]
pub trait ListApiKeysInputPort: std::fmt::Debug + Send + Sync {
    async fn execute<'a>(
        &self,
        input: &ListApiKeysInputData<'a>,
    ) -> errors::Result<Vec<ListedApiKey>>;
}

#[async_trait::async_trait]
impl ListApiKeysInputPort for ListApiKeys {
    #[tracing::instrument(name = "ListApiKeys::execute", skip(self))]
    async fn execute<'a>(
        &self,
        input: &ListApiKeysInputData<'a>,
    ) -> errors::Result<Vec<ListedApiKey>> {
        let organization = self
            .get_org_by_name
            .execute(&input.org_name.to_string().parse()?)
            .await?
            .ok_or(errors::not_found!("Organization not found"))?;
        let tenant_id: TenantId = organization.id().to_string().parse()?;

        // `library:ListApiKeys` is a person's permission; a key acting
        // for whoever holds it is recognised by the repository policy it
        // carries, in the organization's own tenant, as when it issues a
        // key.
        if input.executor.is_service_account() {
            self.auth_app
                .check_policy(&CheckPolicyInput {
                    executor: input.executor,
                    multi_tenancy: &MultiTenancy::new(
                        Some(LIBRARY_TENANT.clone()),
                        Some(tenant_id.clone()),
                    ),
                    action: "library:ManageRepoPolicy",
                })
                .await?;
        } else {
            self.auth_app
                .check_policy(&CheckPolicyInput {
                    executor: input.executor,
                    multi_tenancy: input.multi_tenancy,
                    action: "library:ListApiKeys",
                })
                .await?;
        }

        let service_accounts = self
            .auth_app
            .find_all_service_accounts(&FindAllServiceAccountsInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                tenant_id: &tenant_id,
            })
            .await?;

        // Keys live one per account (see `ApiKeyServiceAccount`), so an
        // organization has as many accounts to ask about as it has keys.
        // A few requests run at a time: one after another would be as
        // slow as the key count, all at once as many upstream requests as
        // it has keys.
        let library_accounts: Vec<_> = service_accounts
            .iter()
            .filter_map(|service_account| {
                ApiKeyServiceAccount::from_name(service_account.name()).map(
                    |account| {
                        (service_account.id().clone(), account.role())
                    },
                )
            })
            .collect();
        let organization = &organization;
        let per_account: Vec<_> =
            futures_util::stream::iter(library_accounts)
                .map(|(service_account_id, role)| async move {
                    let api_keys = self
                        .auth_app
                        .find_all_public_api_key(
                            &FindAllPublicApiKeyInput {
                                executor: input.executor,
                                multi_tenancy: input.multi_tenancy,
                                operator_id: organization.id(),
                                service_account_id: &service_account_id,
                            },
                        )
                        .await?;
                    Ok::<_, errors::Error>(
                        api_keys.into_iter().map(move |api_key| {
                            ListedApiKey { api_key, role }
                        }),
                    )
                })
                .buffered(super::api_key_issuer::CONCURRENT_ACCOUNT_LOOKUPS)
                .try_collect()
                .await?;

        let mut listed: Vec<_> =
            per_account.into_iter().flatten().collect();
        listed.sort_by_key(|key| *key.api_key.created_at());
        Ok(listed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Organization;
    use async_trait::async_trait;
    use mockall::mock;
    use std::str::FromStr;
    use tachyon_sdk::auth::{
        test_helper::{create_test_executor, create_test_multi_tenancy},
        MockAuthApp, PublicApiKeyId, PublicApiKeyValue, ServiceAccount,
    };
    use value_object::Text;

    mock! {
        #[derive(Debug)]
        GetOrgByUsername {}
        #[async_trait]
        impl GetOrganizationByUsernameQuery for GetOrgByUsername {
            async fn execute(&self, username: &Identifier) -> errors::Result<Option<Organization>>;
        }
    }

    fn org(id: &TenantId) -> Organization {
        Organization::new(
            id,
            &Text::new("Test Organization").unwrap(),
            &Identifier::from_str("test-org").unwrap(),
            None,
            None,
        )
    }

    #[tokio::test]
    async fn lists_every_library_key_with_the_role_it_was_issued_with() {
        let tenant_id = TenantId::default();
        let mut auth = MockAuthApp::new();
        auth.expect_check_policy()
            .returning(|_| Box::pin(async { Ok(()) }));
        auth.expect_find_all_service_accounts().returning({
            let tenant_id = tenant_id.clone();
            move |_| {
                let accounts = [
                    ("sa_01legacy", "default"),
                    ("sa_01reader", "library-api-key-reader-0123456789abcdef0123456789abcdef"),
                    ("sa_01public", "library-api-key-public-fedcba9876543210fedcba9876543210"),
                    // Not Library's: its keys stay out of the list.
                    ("sa_01cli", "tachyon-cli"),
                ]
                .map(|(id, name)| ServiceAccount {
                    id: id.to_string().into(),
                    tenant_id: tenant_id.clone(),
                    name: name.to_string(),
                    created_at: chrono::Utc::now(),
                })
                .to_vec();
                Box::pin(async move { Ok(accounts) })
            }
        });
        auth.expect_find_all_public_api_key().returning({
            let tenant_id = tenant_id.clone();
            move |input| {
                let sa = input.service_account_id.as_str().to_string();
                // Older accounts issued earlier keys.
                let offset = match sa.as_str() {
                    "sa_01legacy" => 3,
                    "sa_01reader" => 2,
                    _ => 1,
                };
                let key = PublicApiKey {
                    id: PublicApiKeyId::from(format!("pk_of_{sa}")),
                    tenant_id: tenant_id.clone(),
                    service_account_id: input.service_account_id.clone(),
                    name: "key".to_string(),
                    value: PublicApiKeyValue::new("pk_****"),
                    created_at: chrono::Utc::now()
                        - chrono::Duration::hours(offset),
                };
                Box::pin(async move { Ok(vec![key]) })
            }
        });

        let mut get_org = MockGetOrgByUsername::new();
        get_org.expect_execute().returning({
            let tenant_id = tenant_id.clone();
            move |_| Ok(Some(org(&tenant_id)))
        });

        let executor = create_test_executor();
        let multi_tenancy = create_test_multi_tenancy();
        let listed = ListApiKeys::new(Arc::new(auth), Arc::new(get_org))
            .execute(&ListApiKeysInputData {
                executor: &executor,
                multi_tenancy: &multi_tenancy,
                org_name: &Identifier::from_str("test-org").unwrap(),
            })
            .await
            .unwrap();

        let got: Vec<_> = listed
            .iter()
            .map(|k| (k.api_key.id().to_string(), k.role))
            .collect();
        assert_eq!(
            got,
            [
                ("pk_of_sa_01legacy".to_string(), None),
                ("pk_of_sa_01reader".to_string(), Some(ApiKeyRole::Reader)),
                ("pk_of_sa_01public".to_string(), None),
            ]
        );
    }
}
