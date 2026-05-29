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
        if let Some(org) =
            self.org_repo.get_by_username(org_username).await?
        {
            return Ok(Some(org));
        }

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::OrganizationRepository;
    use value_object::{LongText, TenantId, Text};

    #[derive(Debug)]
    struct LocalOrgRepo {
        org: Organization,
    }

    #[async_trait::async_trait]
    impl OrganizationRepository for LocalOrgRepo {
        async fn insert(
            &self,
            _organization: &Organization,
        ) -> errors::Result<()> {
            Ok(())
        }

        async fn update(
            &self,
            _organization: &Organization,
        ) -> errors::Result<()> {
            Ok(())
        }

        async fn get_by_id(
            &self,
            org_id: &TenantId,
        ) -> errors::Result<Option<Organization>> {
            Ok((self.org.id() == org_id).then(|| self.org.clone()))
        }

        async fn get_by_username(
            &self,
            username: &Identifier,
        ) -> errors::Result<Option<Organization>> {
            Ok((self.org.username() == username).then(|| self.org.clone()))
        }

        async fn find_all(&self) -> errors::Result<Vec<Organization>> {
            Ok(vec![self.org.clone()])
        }

        async fn delete(&self, _org_id: &TenantId) -> errors::Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn execute_prefers_local_library_org_username() {
        let org = Organization::new(
            &TenantId::default(),
            &Text::new("Aser").unwrap(),
            &"aser".parse().unwrap(),
            Some(&LongText::new("Local Library org").unwrap()),
            None,
        );
        let query = GetOrganizationByUsernameQueryImpl::new(
            Arc::new(SdkAuthApp::new(
                "http://127.0.0.1:9",
                &LIBRARY_TENANT,
                "unused-token",
            )),
            Arc::new(LocalOrgRepo { org: org.clone() }),
        );

        let resolved =
            query.execute(&"aser".parse().unwrap()).await.unwrap();

        assert_eq!(
            resolved.map(|org| org.id().clone()),
            Some(org.id().clone())
        );
    }
}
