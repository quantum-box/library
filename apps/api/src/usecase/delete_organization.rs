use std::sync::Arc;

use super::{DeleteOrganizationInputData, DeleteOrganizationInputPort};
use crate::domain::OrganizationRepository;
use tachyon_sdk::auth::{AuthApp, CheckPolicyInput};
use value_object::{FromStr, Identifier};

/// Deletes a Library organization and the repositories it owns.
///
/// Only Library's own rows are removed. The tachyon tenant (operator)
/// behind the organization is left alone: its lifecycle belongs to
/// tachyon, which removes it on its own side.
#[derive(Debug, Clone)]
pub struct DeleteOrganization {
    get_organization_by_username:
        Arc<dyn super::GetOrganizationByUsernameQuery>,
    organization_repository: Arc<dyn OrganizationRepository>,
    auth_app: Arc<dyn AuthApp>,
}

impl DeleteOrganization {
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
impl DeleteOrganizationInputPort for DeleteOrganization {
    #[tracing::instrument(
        name = "DeleteOrganization::execute",
        skip(self, input)
    )]
    async fn execute(
        &self,
        input: &DeleteOrganizationInputData<'_>,
    ) -> errors::Result<()> {
        let org = self
            .get_organization_by_username
            .execute(&Identifier::from_str(&input.username)?)
            .await?
            .ok_or(errors::Error::not_found("organization is not found"))?;

        let scope = super::authorization_scope_for_org(
            input.multi_tenancy,
            org.id(),
        )?;
        self.auth_app
            .check_policy(&CheckPolicyInput {
                executor: input.executor,
                multi_tenancy: &scope,
                action: "library:DeleteOrganization",
            })
            .await?;

        if org.personal_owner_user_id().is_some() {
            return Err(errors::Error::bad_request(
                "a personal space cannot be deleted",
            ));
        }

        // Removes the organization's repos in the same transaction.
        self.organization_repository.delete(org.id()).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use std::sync::{Arc, Mutex};

    use mockall::mock;
    use tachyon_sdk::auth::{
        test_helper::create_test_executor, MockAuthApp, MultiTenancy,
    };
    use value_object::{Identifier, TenantId, Text, UserId};

    use super::*;
    use crate::domain::Organization;
    use crate::usecase::GetOrganizationByUsernameQuery;

    mock! {
        #[derive(Debug)]
        Orgs {}
        #[async_trait::async_trait]
        impl GetOrganizationByUsernameQuery for Orgs {
            async fn execute(&self, username: &Identifier) -> errors::Result<Option<Organization>>;
        }
    }

    mock! {
        #[derive(Debug)]
        OrgStore {}
        #[async_trait::async_trait]
        impl OrganizationRepository for OrgStore {
            async fn insert(&self, organization: &Organization) -> errors::Result<()>;
            async fn update(&self, organization: &Organization) -> errors::Result<()>;
            async fn get_by_id(&self, org_id: &TenantId) -> errors::Result<Option<Organization>>;
            async fn get_by_username(&self, username: &Identifier) -> errors::Result<Option<Organization>>;
            async fn find_all(&self) -> errors::Result<Vec<Organization>>;
            async fn delete(&self, org_id: &TenantId) -> errors::Result<()>;
        }
    }

    fn org_id() -> TenantId {
        "tn_01hjryxysgey07h5jz5wagqj0a".parse().unwrap()
    }

    fn org(personal: bool) -> Organization {
        let org = Organization::new(
            &org_id(),
            &Text::new("Org").unwrap(),
            &Identifier::from_str("some-org").unwrap(),
            None,
            None,
        );
        if personal {
            org.with_personal_owner(&UserId::default())
        } else {
            org
        }
    }

    /// Returns the result and whether the org (with its repos) was deleted.
    async fn run(
        personal: bool,
        allowed: bool,
    ) -> (errors::Result<()>, bool) {
        let mut auth = MockAuthApp::new();
        auth.expect_check_policy().returning(move |input| {
            assert_eq!(input.action, "library:DeleteOrganization");
            Box::pin(async move {
                if allowed {
                    Ok(())
                } else {
                    Err(errors::Error::permission_denied("denied"))
                }
            })
        });
        // No call to tachyon's operator deletion is expected: MockAuthApp
        // panics on any unexpected method, `delete_operator` included.
        let mut orgs = MockOrgs::new();
        orgs.expect_execute()
            .returning(move |_| Ok(Some(org(personal))));
        let org_deleted = Arc::new(Mutex::new(false));
        let flag = org_deleted.clone();
        let mut org_store = MockOrgStore::new();
        org_store.expect_delete().returning(move |id| {
            assert_eq!(id, &org_id());
            *flag.lock().unwrap() = true;
            Ok(())
        });
        let usecase = DeleteOrganization::new(
            Arc::new(orgs),
            Arc::new(org_store),
            Arc::new(auth),
        );
        let executor = create_test_executor();
        let scope = MultiTenancy::new(None, Some(org_id()));
        let result = usecase
            .execute(&DeleteOrganizationInputData {
                executor: &executor,
                multi_tenancy: &scope,
                username: "some-org".into(),
            })
            .await;
        let org = *org_deleted.lock().unwrap();
        (result, org)
    }

    #[tokio::test]
    async fn deletes_the_org_without_touching_the_tenant() {
        let (result, org) = run(false, true).await;
        result.unwrap();
        assert!(org);
    }

    #[tokio::test]
    async fn denied_policy_deletes_nothing() {
        let (result, org) = run(false, false).await;
        assert!(result.is_err());
        assert!(!org);
    }

    #[tokio::test]
    async fn a_personal_space_is_not_deleted() {
        let (result, org) = run(true, true).await;
        assert!(result.is_err());
        assert!(!org);
    }
}
