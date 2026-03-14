use std::sync::Arc;

use super::{
    UpdateOrganizationInputData, UpdateOrganizationInputPort,
    UpdateOrganizationOutputData,
};
use crate::domain::{Organization, OrganizationRepository};
use tachyon_sdk::auth::{AuthApp, CheckPolicyInput};
use value_object::{FromStr, Identifier, LongText, Text};

#[derive(Debug, Clone)]
pub struct UpdateOrganization {
    get_organization_by_username:
        Arc<dyn super::GetOrganizationByUsernameQuery>,
    organization_repository: Arc<dyn OrganizationRepository>,
    auth_app: Arc<dyn AuthApp>,
}

impl UpdateOrganization {
    pub fn new(
        get_organization_by_username: Arc<
            dyn super::GetOrganizationByUsernameQuery,
        >,
        organization_repository: Arc<dyn OrganizationRepository>,
        auth_app: Arc<dyn AuthApp>,
    ) -> Arc<Self> {
        Arc::new(Self {
            get_organization_by_username,
            organization_repository,
            auth_app,
        })
    }
}

#[async_trait::async_trait]
impl UpdateOrganizationInputPort for UpdateOrganization {
    async fn execute(
        &self,
        input: &UpdateOrganizationInputData<'_>,
    ) -> errors::Result<UpdateOrganizationOutputData> {
        self.auth_app
            .check_policy(&CheckPolicyInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                action: "library:UpdateOrganization",
            })
            .await?;

        let org = self
            .get_organization_by_username
            .execute(&Identifier::from_str(&input.username)?)
            .await?
            .ok_or(errors::Error::not_found("organization is not found"))?;

        let updated_org = Organization::new(
            org.id(),
            &Text::from_str(&input.name)?,
            org.username(),
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

        self.organization_repository.update(&updated_org).await?;

        Ok(UpdateOrganizationOutputData {
            organization: updated_org,
        })
    }
}
