use std::sync::Arc;

use super::{CreateOrganizationInputData, CreateOrganizationInputPort};
use crate::domain::{
    library_repo_owner_policy_id, library_user_policy_id, Organization,
    OrganizationRepository, LIBRARY_TENANT,
};
use tachyon_sdk::auth::{
    AttachUserPolicyInput, AuthApp, CheckPolicyInput, CreateOperatorInput,
    Executor, MultiTenancy, NewOperatorOwnerMethod,
};
use value_object::{FromStr, LongText, TenantId, Text, UserId};

async fn attach_creator_policies(
    auth_app: &dyn AuthApp,
    user_id: &UserId,
    tenant_id: &TenantId,
) -> errors::Result<()> {
    let scope = MultiTenancy::new(
        Some(LIBRARY_TENANT.clone()),
        Some(tenant_id.clone()),
    );
    let executor = Executor::SystemUser;

    for policy_id in
        [library_user_policy_id(), library_repo_owner_policy_id()]
    {
        auth_app
            .attach_user_policy(&AttachUserPolicyInput {
                executor: &executor,
                multi_tenancy: &scope,
                user_id,
                policy_id: &policy_id,
                tenant_id,
            })
            .await?;
    }

    Ok(())
}

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

        let tenant_id = TenantId::new(operator.id().as_ref())?;
        let user_id = input.executor.get_user_id()?;
        attach_creator_policies(
            self.auth_app.as_ref(),
            &user_id,
            &tenant_id,
        )
        .await?;

        self.organization_repository.insert(&organization).await?;

        Ok(organization)
    }
}

#[cfg(test)]
mod tests {
    use super::attach_creator_policies;
    use crate::domain::{
        LIBRARY_REPO_OWNER_POLICY_ID, LIBRARY_TENANT,
        LIBRARY_USER_POLICY_ID,
    };
    use std::sync::{Arc, Mutex};
    use tachyon_sdk::auth::MockAuthApp;
    use value_object::{TenantId, UserId};

    #[tokio::test]
    async fn attaches_creator_policies_in_the_new_org_scope() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let mut auth = MockAuthApp::new();
        auth.expect_attach_user_policy().times(2).returning({
            let calls = calls.clone();
            move |input| {
                calls.lock().unwrap().push((
                    input.policy_id.to_string(),
                    input.tenant_id.to_string(),
                    input
                        .multi_tenancy
                        .platform_id()
                        .map(|id| id.to_string()),
                    input
                        .multi_tenancy
                        .operator_id()
                        .map(|id| id.to_string()),
                ));
                Box::pin(async { Ok(()) })
            }
        });

        let user_id = UserId::new("us_01testcreator").unwrap();
        let tenant_id = TenantId::new("tn_01testorganization").unwrap();

        attach_creator_policies(&auth, &user_id, &tenant_id)
            .await
            .unwrap();

        assert_eq!(
            calls.lock().unwrap().as_slice(),
            &[
                (
                    LIBRARY_USER_POLICY_ID.to_string(),
                    tenant_id.to_string(),
                    Some(LIBRARY_TENANT.to_string()),
                    Some(tenant_id.to_string()),
                ),
                (
                    LIBRARY_REPO_OWNER_POLICY_ID.to_string(),
                    tenant_id.to_string(),
                    Some(LIBRARY_TENANT.to_string()),
                    Some(tenant_id.to_string()),
                ),
            ]
        );
    }

    #[tokio::test]
    async fn returns_policy_attachment_failures() {
        let mut auth = MockAuthApp::new();
        auth.expect_attach_user_policy().times(1).returning(|_| {
            Box::pin(async {
                Err(errors::Error::forbidden("attach failed"))
            })
        });

        let user_id = UserId::new("us_01testcreator").unwrap();
        let tenant_id = TenantId::new("tn_01testorganization").unwrap();

        let error = attach_creator_policies(&auth, &user_id, &tenant_id)
            .await
            .unwrap_err();

        assert!(error.to_string().contains("attach failed"));
    }
}
