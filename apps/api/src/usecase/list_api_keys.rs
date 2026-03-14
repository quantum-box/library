use std::sync::Arc;

use derive_new::new;
use tachyon_sdk::auth::PublicApiKey;
use tachyon_sdk::auth::{
    AuthApp, CheckPolicyInput, CreateServiceAccountInput,
    FindAllPublicApiKeyInput, GetServiceAccountByNameInput,
};
use value_object::Identifier;

use super::GetOrganizationByUsernameQuery;

#[derive(Debug, Clone)]
pub struct ListApiKeysInputData<'a> {
    pub executor: &'a dyn tachyon_sdk::auth::ExecutorAction,
    pub multi_tenancy: &'a dyn tachyon_sdk::auth::MultiTenancyAction,

    pub org_name: &'a Identifier,
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
    ) -> errors::Result<Vec<PublicApiKey>>;
}

#[async_trait::async_trait]
impl ListApiKeysInputPort for ListApiKeys {
    #[tracing::instrument(name = "ListApiKeys::execute", skip(self))]
    async fn execute<'a>(
        &self,
        input: &ListApiKeysInputData<'a>,
    ) -> errors::Result<Vec<PublicApiKey>> {
        self.auth_app
            .check_policy(&CheckPolicyInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                action: "library:ListApiKeys",
            })
            .await?;

        // TODO: add English comment
        let organization = self
            .get_org_by_name
            .execute(&input.org_name.to_string().parse()?)
            .await?
            .ok_or(errors::not_found!("Organization not found"))?;

        // TODO: add English comment
        let service_account = self
            .auth_app
            .get_service_account_by_name(&GetServiceAccountByNameInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                tenant_id: organization.id(),
                name: "default",
            })
            .await?;

        let service_account = match service_account {
            Some(sa) => sa,
            None => {
                // TODO: add English comment
                self.auth_app
                    .create_service_account(&CreateServiceAccountInput {
                        executor: input.executor,
                        multi_tenancy: input.multi_tenancy,
                        tenant_id: organization.id(),
                        name: "default",
                    })
                    .await?
            }
        };

        // TODO: add English comment
        let api_keys = self
            .auth_app
            .find_all_public_api_key(&FindAllPublicApiKeyInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                operator_id: organization.id(),
                service_account_id: service_account.id(),
            })
            .await?;

        Ok(api_keys)
    }
}
