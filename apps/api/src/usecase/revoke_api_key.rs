use std::sync::Arc;

use derive_new::new;
use tachyon_sdk::auth::{
    AuthApp, CheckPolicyInput, GetServiceAccountByNameInput,
    PublicApiKeyId, RevokePublicApiKeyInput,
};
use value_object::Identifier;

use super::GetOrganizationByUsernameQuery;

#[derive(Debug, Clone)]
pub struct RevokeApiKeyInputData<'a> {
    pub executor: &'a dyn tachyon_sdk::auth::ExecutorAction,
    pub multi_tenancy: &'a dyn tachyon_sdk::auth::MultiTenancyAction,

    pub org_name: &'a Identifier,
    pub api_key_id: &'a str,
    pub service_account_name: Option<&'a str>,
}

#[derive(Debug, Clone, new)]
pub struct RevokeApiKey {
    auth_app: Arc<dyn AuthApp>,
    get_org_by_name: Arc<dyn GetOrganizationByUsernameQuery>,
}

#[async_trait::async_trait]
pub trait RevokeApiKeyInputPort: std::fmt::Debug + Send + Sync {
    async fn execute<'a>(
        &self,
        input: &RevokeApiKeyInputData<'a>,
    ) -> errors::Result<()>;
}

#[async_trait::async_trait]
impl RevokeApiKeyInputPort for RevokeApiKey {
    #[tracing::instrument(name = "RevokeApiKey::execute", skip(self))]
    async fn execute<'a>(
        &self,
        input: &RevokeApiKeyInputData<'a>,
    ) -> errors::Result<()> {
        self.auth_app
            .check_policy(&CheckPolicyInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                action: "library:RevokeApiKey",
            })
            .await?;

        // The key belongs to the organization named in the request, so
        // the operator it is revoked under is resolved the same way
        // issuing resolves it.
        let organization = self
            .get_org_by_name
            .execute(&input.org_name.to_string().parse()?)
            .await?
            .ok_or(errors::not_found!("Organization not found"))?;

        let service_account_name =
            input.service_account_name.unwrap_or("default");

        // Keys hang off a service account, and an organization that
        // never issued one has no key to revoke either. Missing here is
        // therefore not-found rather than a reason to create one.
        let service_account = self
            .auth_app
            .get_service_account_by_name(&GetServiceAccountByNameInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                tenant_id: organization.id(),
                name: service_account_name,
            })
            .await?
            .ok_or(errors::not_found!("Service account not found"))?;

        let api_key_id = PublicApiKeyId::new(input.api_key_id);

        self.auth_app
            .revoke_public_api_key(&RevokePublicApiKeyInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                operator_id: organization.id(),
                service_account_id: service_account.id(),
                api_key_id: &api_key_id,
            })
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Organization;
    use crate::usecase::GetOrganizationByUsernameQuery;
    use async_trait::async_trait;
    use mockall::mock;
    use std::str::FromStr;
    use std::sync::{Arc, Mutex};
    use tachyon_sdk::auth::{
        test_helper::{create_test_executor, create_test_multi_tenancy},
        MockAuthApp, ServiceAccount, ServiceAccountId,
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

    fn org(id: &TenantId) -> Organization {
        Organization::new(
            id,
            &Text::new("Test Organization").unwrap(),
            &Identifier::from_str("test-org").unwrap(),
            None,
            None,
        )
    }

    fn service_account(tenant_id: &TenantId) -> ServiceAccount {
        ServiceAccount {
            id: ServiceAccountId::new("sa_01test").unwrap(),
            tenant_id: tenant_id.clone(),
            name: "default".to_string(),
            created_at: chrono::Utc::now(),
        }
    }

    #[tokio::test]
    async fn revoking_targets_the_key_under_the_organization_that_issued_it(
    ) {
        let tenant_id = TenantId::default();
        let calls = Arc::new(Mutex::new(Vec::new()));

        let mut auth = MockAuthApp::new();
        auth.expect_check_policy().returning({
            let calls = calls.clone();
            move |input| {
                calls
                    .lock()
                    .unwrap()
                    .push(format!("policy:{}", input.action));
                Box::pin(async { Ok(()) })
            }
        });
        auth.expect_get_service_account_by_name().returning({
            let tenant_id = tenant_id.clone();
            move |_| {
                let sa = service_account(&tenant_id);
                Box::pin(async move { Ok(Some(sa)) })
            }
        });
        auth.expect_revoke_public_api_key().returning({
            let calls = calls.clone();
            move |input| {
                calls.lock().unwrap().push(format!(
                    "revoke:{}:{}:{}",
                    input.operator_id,
                    input.service_account_id.as_str(),
                    input.api_key_id.as_str(),
                ));
                Box::pin(async { Ok(()) })
            }
        });

        let mut org_query = MockGetOrgByUsername::new();
        org_query.expect_execute().returning({
            let tenant_id = tenant_id.clone();
            move |_| {
                let organization = org(&tenant_id);
                Ok(Some(organization))
            }
        });

        let usecase =
            RevokeApiKey::new(Arc::new(auth), Arc::new(org_query));
        let executor = create_test_executor();
        let multi_tenancy = create_test_multi_tenancy();

        usecase
            .execute(&RevokeApiKeyInputData {
                executor: &executor,
                multi_tenancy: &multi_tenancy,
                org_name: &Identifier::from_str("test-org").unwrap(),
                api_key_id: "pak_01test",
                service_account_name: None,
            })
            .await
            .unwrap();

        assert_eq!(
            calls.lock().unwrap().as_slice(),
            &[
                "policy:library:RevokeApiKey".to_string(),
                format!("revoke:{tenant_id}:sa_01test:pak_01test"),
            ]
        );
    }

    #[tokio::test]
    async fn a_missing_service_account_is_not_created_to_revoke_against() {
        let tenant_id = TenantId::default();

        let mut auth = MockAuthApp::new();
        auth.expect_check_policy()
            .returning(|_| Box::pin(async { Ok(()) }));
        auth.expect_get_service_account_by_name()
            .returning(|_| Box::pin(async { Ok(None) }));
        auth.expect_create_service_account().never();
        auth.expect_revoke_public_api_key().never();

        let mut org_query = MockGetOrgByUsername::new();
        org_query.expect_execute().returning({
            let tenant_id = tenant_id.clone();
            move |_| {
                let organization = org(&tenant_id);
                Ok(Some(organization))
            }
        });

        let usecase =
            RevokeApiKey::new(Arc::new(auth), Arc::new(org_query));
        let executor = create_test_executor();
        let multi_tenancy = create_test_multi_tenancy();

        let result = usecase
            .execute(&RevokeApiKeyInputData {
                executor: &executor,
                multi_tenancy: &multi_tenancy,
                org_name: &Identifier::from_str("test-org").unwrap(),
                api_key_id: "pak_01test",
                service_account_name: None,
            })
            .await;

        assert!(matches!(result, Err(errors::Error::NotFound { .. })));
    }

    #[tokio::test]
    async fn a_denied_policy_stops_before_anything_is_revoked() {
        let mut auth = MockAuthApp::new();
        auth.expect_check_policy().returning(|_| {
            Box::pin(async { Err(errors::Error::forbidden("denied")) })
        });
        auth.expect_get_service_account_by_name().never();
        auth.expect_revoke_public_api_key().never();

        let mut org_query = MockGetOrgByUsername::new();
        org_query.expect_execute().never();

        let usecase =
            RevokeApiKey::new(Arc::new(auth), Arc::new(org_query));
        let executor = create_test_executor();
        let multi_tenancy = create_test_multi_tenancy();

        let result = usecase
            .execute(&RevokeApiKeyInputData {
                executor: &executor,
                multi_tenancy: &multi_tenancy,
                org_name: &Identifier::from_str("test-org").unwrap(),
                api_key_id: "pak_01test",
                service_account_name: None,
            })
            .await;

        assert!(matches!(result, Err(errors::Error::Forbidden { .. })));
    }
}
