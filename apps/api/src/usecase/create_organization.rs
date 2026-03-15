use std::sync::Arc;

use super::{CreateOrganizationInputData, CreateOrganizationInputPort};
use crate::domain::{Organization, OrganizationRepository, LIBRARY_TENANT};
use tachyon_sdk::auth::PolicyId;
use tachyon_sdk::auth::{
    AttachUserPolicyInput, AuthApp, CheckPolicyInput, CreateOperatorInput,
    NewOperatorOwnerMethod,
};
use tracing::warn;
use value_object::{FromStr, LongText, TenantId, Text};

#[derive(Debug, Clone)]
pub struct CreateOrganization {
    organization_repository: Arc<dyn OrganizationRepository>,
    auth_app: Arc<dyn AuthApp>,
}

impl CreateOrganization {
    pub fn new(
        organization_repository: Arc<dyn OrganizationRepository>,
        auth_app: Arc<dyn AuthApp>,
    ) -> Arc<Self> {
        Arc::new(Self {
            organization_repository,
            auth_app,
        })
    }
}

#[async_trait::async_trait]
impl CreateOrganizationInputPort for CreateOrganization {
    #[tracing::instrument(name = "CreateOrganization::execute", skip(self))]
    async fn execute<'a>(
        &self,
        input: &CreateOrganizationInputData<'a>,
    ) -> errors::Result<Organization> {
        self.auth_app
            .check_policy(&CheckPolicyInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                action: "library:CreateOrganization",
            })
            .await?;

        let operator = self
            .auth_app
            .create_operator(&CreateOperatorInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                platform_id: &LIBRARY_TENANT,
                operator_alias: &input.username.parse()?,
                operator_name: &input.name,
                new_operator_owner_method: NewOperatorOwnerMethod::Inherit,
                new_operator_owner_id: &input.executor.get_user_id()?,
                new_operator_owner_password: None,
            })
            .await?;

        let organization = Organization::new(
            operator.id(),
            &Text::from_str(&input.name)?,
            operator.operator_name(),
            input
                .description
                .as_ref()
                .map(|d| LongText::from_str(d))
                .transpose()?
                .as_ref(),
            input
                .website
                .as_ref()
                .map(|w| value_object::Url::from_str(w))
                .transpose()?
                .as_ref(),
        );

        self.organization_repository.insert(&organization).await?;

        // TODO: add English comment
        const LIBRARY_POLICY_ID: &str = "pol_01libraryuserpolicy";
        let policy_id = PolicyId::new(LIBRARY_POLICY_ID);
        if let (Ok(tenant_id), Ok(user_id)) = (
            TenantId::new(operator.id().as_ref()),
            input.executor.get_user_id(),
        ) {
            if let Err(err) = self
                .auth_app
                .attach_user_policy(&AttachUserPolicyInput {
                    executor: &tachyon_sdk::auth::Executor::SystemUser,
                    multi_tenancy:
                        &tachyon_sdk::auth::MultiTenancy::default(),
                    user_id: &user_id,
                    policy_id: &policy_id,
                    tenant_id: &tenant_id,
                })
                .await
            {
                warn!(
                    error = ?err,
                    "failed to attach default library policy; continuing organization creation"
                );
            }
        }

        Ok(organization)
    }
}
