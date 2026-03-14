use std::sync::Arc;

use derive_new::new;
use tachyon_sdk::auth::PublicApiKey;
use tachyon_sdk::auth::{
    AuthApp, CheckPolicyInput, CreatePublicApiKeyInput,
    CreateServiceAccountInput, GetServiceAccountByNameInput,
    ServiceAccount,
};
use value_object::{Identifier, OperatorId};

use super::GetOrganizationByUsernameQuery;

#[derive(Debug, Clone)]
pub struct CreateApiKeyInputData<'a> {
    pub executor: &'a dyn tachyon_sdk::auth::ExecutorAction,
    pub multi_tenancy: &'a dyn tachyon_sdk::auth::MultiTenancyAction,

    pub org_name: &'a Identifier,
    pub name: &'a str,
    pub service_account_name: Option<&'a str>,
}

#[derive(Debug, Clone)]
pub struct CreateApiKeyOutputData {
    pub api_key: PublicApiKey,
    pub service_account: ServiceAccount,
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
        self.auth_app
            .check_policy(&CheckPolicyInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                action: "library:CreateApiKey",
            })
            .await?;

        // TODO: add English comment
        let organization = self
            .get_org_by_name
            .execute(&input.org_name.to_string().parse()?)
            .await?
            .ok_or(errors::not_found!("Organization not found"))?;

        // TODO: add English comment
        let service_account_name =
            input.service_account_name.unwrap_or("default");
        let service_account = self
            .get_or_create_service_account(
                input.executor,
                input.multi_tenancy,
                organization.id(),
                service_account_name,
            )
            .await?;

        // TODO: add English comment
        let api_key = self
            .auth_app
            .create_public_api_key(&CreatePublicApiKeyInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                operator_id: organization.id(),
                service_account_id: service_account.id(),
                name: input.name,
            })
            .await?;

        Ok(CreateApiKeyOutputData {
            api_key,
            service_account,
        })
    }
}

impl CreateApiKey {
    #[tracing::instrument(
        name = "CreateApiKey::get_or_create_service_account",
        skip(self)
    )]
    async fn get_or_create_service_account<'a>(
        &self,
        executor: &'a dyn tachyon_sdk::auth::ExecutorAction,
        multi_tenancy: &'a dyn tachyon_sdk::auth::MultiTenancyAction,
        organization_id: &'a OperatorId,
        service_account_name: &'a str,
    ) -> errors::Result<ServiceAccount> {
        // TODO: add English comment
        let tenant_id = organization_id.to_string().parse()?;

        // TODO: add English comment
        let service_account = self
            .auth_app
            .get_service_account_by_name(&GetServiceAccountByNameInput {
                executor,
                multi_tenancy,
                tenant_id: &tenant_id,
                name: service_account_name,
            })
            .await?;

        // TODO: add English comment
        if let Some(service_account) = service_account {
            Ok(service_account)
        } else {
            let service_account = self
                .auth_app
                .create_service_account(&CreateServiceAccountInput {
                    executor,
                    multi_tenancy,
                    tenant_id: &tenant_id,
                    name: service_account_name,
                })
                .await?;
            Ok(service_account)
        }
    }
}
