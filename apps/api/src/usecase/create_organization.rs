use std::sync::Arc;

use super::{CreateOrganizationInputData, CreateOrganizationInputPort};
use crate::domain::{
    library_repo_owner_policy_id, library_user_policy_id, Organization,
    OrganizationRepository, LIBRARY_TENANT,
};
use tachyon_sdk::auth::{
    AttachUserPolicyInput, AuthApp, CheckPolicyInput, CreateOperatorInput,
    ExecutorAction, MultiTenancy, NewOperatorOwnerMethod,
};
use tracing::warn;
use value_object::{FromStr, LongText, TenantId, Text, UserId};

/// Grant the creator the Library policies for the organization they
/// just created.
///
/// The creator is the executor: a system executor would fall back to
/// the service account, which belongs to the Library platform tenant
/// and is therefore rejected in any per-organization scope.
///
/// Being the executor is not sufficient on its own. tachyon authorizes
/// this call against the caller, so the caller must hold
/// `auth:AttachUserPolicy` — which reaches them through the
/// `library:org-creator` companion policy attached at sign-in. Owning
/// the new tenant does not stand in for it: production refused this
/// call for the creator of the tenant `create_operator` had just made
/// them the owner of, until the action was added to that policy.
async fn attach_creator_policies(
    auth_app: &dyn AuthApp,
    executor: &dyn ExecutorAction,
    user_id: &UserId,
    tenant_id: &TenantId,
) -> errors::Result<()> {
    let scope = MultiTenancy::new(
        Some(LIBRARY_TENANT.clone()),
        Some(tenant_id.clone()),
    );

    for policy_id in
        [library_user_policy_id(), library_repo_owner_policy_id()]
    {
        auth_app
            .attach_user_policy(&AttachUserPolicyInput {
                executor,
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

        // `create_operator` already created the tenant upstream and it
        // cannot be rolled back from here (`delete_operator` is not
        // supported through this SDK), so record the organization before
        // granting policies. Inserting afterwards would leave a tenant
        // Library never stored whenever the grant fails, and the creator
        // could not even retry: the alias is already taken.
        self.organization_repository.insert(&organization).await?;

        // A failed grant leaves an organization that exists and is
        // usable, only without its creator policies. Failing the whole
        // mutation would strand that tenant, so report the failure as a
        // warning and let the caller keep the organization.
        if let Err(error) = attach_creator_policies(
            self.auth_app.as_ref(),
            input.executor,
            &user_id,
            &tenant_id,
        )
        .await
        {
            warn!(
                error = ?error,
                organization_id = %tenant_id,
                creator_id = %user_id,
                "failed to grant the creator policies for the new organization"
            );
        }

        Ok(organization)
    }
}

#[cfg(test)]
mod tests {
    use super::attach_creator_policies;
    use super::{
        CreateOrganization, CreateOrganizationInputData,
        CreateOrganizationInputPort,
    };
    use crate::domain::{
        Organization, OrganizationRepository, LIBRARY_REPO_OWNER_POLICY_ID,
        LIBRARY_TENANT, LIBRARY_USER_POLICY_ID,
    };
    use std::sync::{Arc, Mutex};
    use tachyon_sdk::auth::{
        ExecutorAction, MockAuthApp, MultiTenancy, Operator,
    };
    use value_object::{Identifier, TenantId, UserId};

    /// Stand-in for the signed-in caller. Only `is_user` matters: it is
    /// what makes the SDK layer carry the caller's own credential
    /// instead of the service account's.
    #[derive(Debug)]
    struct CallerExecutor(&'static str);

    impl ExecutorAction for CallerExecutor {
        fn get_id(&self) -> &str {
            self.0
        }
        fn has_tenant_id(&self, _tenant_id: &TenantId) -> bool {
            true
        }
        fn is_system_user(&self) -> bool {
            false
        }
        fn is_user(&self) -> bool {
            true
        }
        fn is_service_account(&self) -> bool {
            false
        }
        fn is_none(&self) -> bool {
            false
        }
    }

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

        let executor = CallerExecutor("us_01testcreator");
        let user_id = UserId::new("us_01testcreator").unwrap();
        let tenant_id = TenantId::new("tn_01testorganization").unwrap();

        attach_creator_policies(&auth, &executor, &user_id, &tenant_id)
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

        let executor = CallerExecutor("us_01testcreator");
        let user_id = UserId::new("us_01testcreator").unwrap();
        let tenant_id = TenantId::new("tn_01testorganization").unwrap();

        let error =
            attach_creator_policies(&auth, &executor, &user_id, &tenant_id)
                .await
                .unwrap_err();

        assert!(error.to_string().contains("attach failed"));
    }

    #[tokio::test]
    async fn forwards_the_caller_as_the_attaching_executor() {
        let is_user_flags = Arc::new(Mutex::new(Vec::new()));
        let mut auth = MockAuthApp::new();
        auth.expect_attach_user_policy().times(2).returning({
            let is_user_flags = is_user_flags.clone();
            move |input| {
                is_user_flags
                    .lock()
                    .unwrap()
                    .push(input.executor.is_user());
                Box::pin(async { Ok(()) })
            }
        });

        let executor = CallerExecutor("us_01testcreator");
        let user_id = UserId::new("us_01testcreator").unwrap();
        let tenant_id = TenantId::new("tn_01testorganization").unwrap();

        attach_creator_policies(&auth, &executor, &user_id, &tenant_id)
            .await
            .unwrap();

        // A system executor here would resolve to the service account,
        // which is scoped to the Library platform tenant and therefore
        // rejected inside the new organization's tenant.
        assert_eq!(is_user_flags.lock().unwrap().as_slice(), &[true, true]);
    }

    /// Records which collaborator the use case touches, and when, so a
    /// regression that stores the organization only after the policy
    /// grant succeeds is visible.
    #[derive(Debug)]
    struct RecordingRepository {
        calls: Arc<Mutex<Vec<&'static str>>>,
    }

    #[async_trait::async_trait]
    impl OrganizationRepository for RecordingRepository {
        async fn insert(
            &self,
            _organization: &Organization,
        ) -> errors::Result<()> {
            self.calls.lock().unwrap().push("insert");
            Ok(())
        }

        async fn update(
            &self,
            _organization: &Organization,
        ) -> errors::Result<()> {
            unimplemented!("not used by these tests")
        }

        async fn get_by_id(
            &self,
            _org_id: &TenantId,
        ) -> errors::Result<Option<Organization>> {
            unimplemented!("not used by these tests")
        }

        async fn get_by_username(
            &self,
            _username: &Identifier,
        ) -> errors::Result<Option<Organization>> {
            unimplemented!("not used by these tests")
        }

        async fn find_all(&self) -> errors::Result<Vec<Organization>> {
            unimplemented!("not used by these tests")
        }

        async fn delete(&self, _org_id: &TenantId) -> errors::Result<()> {
            unimplemented!("not used by these tests")
        }
    }

    fn created_operator() -> Operator {
        Operator {
            id: TenantId::new("tn_01testorganization").unwrap(),
            name: "Test Organization".to_string(),
            operator_name: "test-organization".parse().unwrap(),
            platform_id: LIBRARY_TENANT.clone(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    /// `create_operator` has already created the tenant upstream by the
    /// time the policies are granted, and nothing here can delete it. A
    /// rejected grant must therefore not fail the mutation: doing so
    /// strands a tenant the creator cannot even recreate, because its
    /// alias is taken.
    #[tokio::test]
    async fn keeps_the_organization_when_the_policy_grant_is_rejected() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let mut auth = MockAuthApp::new();
        auth.expect_check_policy()
            .times(1)
            .returning(|_| Box::pin(async { Ok(()) }));
        auth.expect_create_operator()
            .times(1)
            .returning(|_| Box::pin(async { Ok(created_operator()) }));
        auth.expect_attach_user_policy().returning({
            let calls = calls.clone();
            move |_| {
                calls.lock().unwrap().push("attach");
                Box::pin(async {
                    Err(errors::Error::forbidden(
                        "Upstream authorization rejected",
                    ))
                })
            }
        });

        let usecase = CreateOrganization::new(
            Arc::new(RecordingRepository {
                calls: calls.clone(),
            }),
            Arc::new(auth),
        );

        let executor = CallerExecutor("us_01testcreator");
        let multi_tenancy = MultiTenancy::default();
        let organization = usecase
            .execute(&CreateOrganizationInputData {
                executor: &executor,
                multi_tenancy: &multi_tenancy,
                name: "Test Organization".to_string(),
                username: "test-organization".to_string(),
                description: None,
                website: None,
            })
            .await
            .expect("a rejected policy grant must not fail creation");

        assert_eq!(organization.id().to_string(), "tn_01testorganization");
        assert_eq!(calls.lock().unwrap().first().copied(), Some("insert"));
    }

    /// The organization must be stored before the grant is attempted,
    /// so a rejected grant still leaves Library aware of the tenant that
    /// now exists upstream.
    #[tokio::test]
    async fn stores_the_organization_before_granting_policies() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let mut auth = MockAuthApp::new();
        auth.expect_check_policy()
            .times(1)
            .returning(|_| Box::pin(async { Ok(()) }));
        auth.expect_create_operator()
            .times(1)
            .returning(|_| Box::pin(async { Ok(created_operator()) }));
        auth.expect_attach_user_policy().times(2).returning({
            let calls = calls.clone();
            move |_| {
                calls.lock().unwrap().push("attach");
                Box::pin(async { Ok(()) })
            }
        });

        let usecase = CreateOrganization::new(
            Arc::new(RecordingRepository {
                calls: calls.clone(),
            }),
            Arc::new(auth),
        );

        let executor = CallerExecutor("us_01testcreator");
        let multi_tenancy = MultiTenancy::default();
        usecase
            .execute(&CreateOrganizationInputData {
                executor: &executor,
                multi_tenancy: &multi_tenancy,
                name: "Test Organization".to_string(),
                username: "test-organization".to_string(),
                description: None,
                website: None,
            })
            .await
            .unwrap();

        assert_eq!(
            calls.lock().unwrap().as_slice(),
            &["insert", "attach", "attach"]
        );
    }
}
