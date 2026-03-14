use std::sync::Arc;

use derive_new::new;
use errors::Result;
use value_object::Identifier;

use crate::{
    domain::{Organization, OrganizationRepository, LIBRARY_TENANT},
    sdk_auth::SdkAuthApp,
    usecase::GetOrganizationByUsernameQuery,
};

#[derive(Debug, Clone, new)]
pub struct GetOrganizationByUsernameQueryImpl {
    sdk: Arc<SdkAuthApp>,
    org_repo: Arc<dyn OrganizationRepository>,
}

#[async_trait::async_trait]
impl GetOrganizationByUsernameQuery for GetOrganizationByUsernameQueryImpl {
    #[tracing::instrument(
        name = "GetOrganizationByUsernameQueryImpl::execute",
        skip(self)
    )]
    async fn execute(
        &self,
        org_username: &Identifier,
    ) -> Result<Option<Organization>> {
        let username_str = org_username.to_string();
        let operator = self
            .sdk
            .get_operator_by_alias(&LIBRARY_TENANT, &username_str)
            .await?;

        let tenant_id = value_object::TenantId::new(&operator.id)?;
        let org = self.org_repo.get_by_id(&tenant_id).await?;

        Ok(org)
    }
}
