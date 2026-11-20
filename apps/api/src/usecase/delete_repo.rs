use crate::domain::RepoRepository;
use crate::usecase::{
    DeleteRepoInputData, DeleteRepoInputPort,
    GetOrganizationByUsernameQuery, GetRepoByUsernameQuery,
};
use std::sync::Arc;
use tachyon_sdk::auth::{AuthApp, CheckPolicyInput};

#[derive(Debug, Clone)]
pub struct DeleteRepo {
    get_org_by_username: Arc<dyn GetOrganizationByUsernameQuery>,
    get_repo_by_username: Arc<dyn GetRepoByUsernameQuery>,
    repo_repo: Arc<dyn RepoRepository>,
    auth: Arc<dyn AuthApp>,
}

impl DeleteRepo {
    pub fn new(
        get_org_by_username: Arc<dyn GetOrganizationByUsernameQuery>,
        get_repo_by_username: Arc<dyn GetRepoByUsernameQuery>,
        repo_repo: Arc<dyn RepoRepository>,
        auth: Arc<dyn AuthApp>,
    ) -> Arc<Self> {
        Arc::new(Self {
            get_org_by_username,
            get_repo_by_username,
            repo_repo,
            auth,
        })
    }
}

#[async_trait::async_trait]
impl DeleteRepoInputPort for DeleteRepo {
    #[tracing::instrument(name = "DeleteRepo::execute", skip(self))]
    async fn execute<'a>(
        &self,
        input: DeleteRepoInputData<'a>,
    ) -> errors::Result<()> {
        // let user = self
        //     .auth
        //     .authentication()
        //     .get_user(input.tenant_id, &input.actor.parse()?)
        //     .await?
        //     .ok_or(errors::Error::not_found("user"))?;
        let org = self
            .get_org_by_username
            .execute(&input.org_username.parse()?)
            .await?
            .ok_or(errors::Error::not_found("org"))?;
        let repo = self
            .get_repo_by_username
            .execute(org.username(), &input.repo_username.parse()?)
            .await?
            .ok_or(errors::Error::not_found("repo"))?;

        let scope = super::authorization_scope_for_org(
            input.multi_tenancy,
            repo.organization_id(),
        )?;
        self.auth
            .check_policy(&CheckPolicyInput {
                executor: input.executor,
                multi_tenancy: &scope,
                action: "library:DeleteRepo",
            })
            .await?;

        // repo.can_delete(user.id())?;

        self.repo_repo.delete(org.id(), repo.id()).await?;

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
    use value_object::{Identifier, TenantId, Text};

    use super::*;
    use crate::domain::{Organization, Repo, RepoId, LIBRARY_TENANT};

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
        Repos {}
        #[async_trait::async_trait]
        impl GetRepoByUsernameQuery for Repos {
            async fn execute(&self, org_username: &Identifier, repo_username: &Identifier) -> errors::Result<Option<Repo>>;
        }
    }

    mock! {
        #[derive(Debug)]
        RepoStore {}
        #[async_trait::async_trait]
        impl RepoRepository for RepoStore {
            async fn save(&self, entity: &Repo) -> errors::Result<()>;
            async fn get_by_id(&self, tenant_id: &TenantId, id: &RepoId) -> errors::Result<Option<Repo>>;
            async fn find_all(&self, org_id: &TenantId) -> errors::Result<Vec<Repo>>;
            async fn delete(&self, tenant_id: &TenantId, id: &RepoId) -> errors::Result<()>;
        }
    }

    fn caller_org() -> TenantId {
        "tn_01hjryxysgey07h5jz5wagqj0a".parse().unwrap()
    }

    fn victim_org() -> TenantId {
        "tn_01hjryxysgey07h5jz5wagqj0b".parse().unwrap()
    }

    /// Delete the victim org's repository, sending `header` as the
    /// authorization tenant. The policy check passes only when it is
    /// evaluated in `allowed_in`. Returns the result and the tenants a
    /// repository was deleted in.
    async fn delete_victim_repo(
        header: TenantId,
        allowed_in: TenantId,
    ) -> (errors::Result<()>, Vec<TenantId>) {
        let mut auth = MockAuthApp::new();
        auth.expect_check_policy().returning(move |input| {
            let allowed = input.multi_tenancy.operator_id().as_ref()
                == Some(&allowed_in);
            Box::pin(async move {
                if allowed {
                    Ok(())
                } else {
                    Err(errors::Error::permission_denied("denied"))
                }
            })
        });
        let mut orgs = MockOrgs::new();
        orgs.expect_execute().returning(|_| {
            Ok(Some(Organization::new(
                &victim_org(),
                &Text::new("Victim").unwrap(),
                &Identifier::from_str("victim-org").unwrap(),
                None,
                None,
            )))
        });
        let mut repos = MockRepos::new();
        repos.expect_execute().returning(|_, _| {
            Ok(Some(Repo::new(
                &RepoId::default(),
                &victim_org(),
                &Identifier::from_str("victim-org").unwrap(),
                &Text::new("Victim repo").unwrap(),
                &Identifier::from_str("victim-repo").unwrap(),
                false,
                None,
                vec![],
                vec![],
            )))
        });
        let deleted = Arc::new(Mutex::new(Vec::new()));
        let log = deleted.clone();
        let mut store = MockRepoStore::new();
        store.expect_delete().returning(move |tenant, _| {
            log.lock().unwrap().push(tenant.clone());
            Ok(())
        });
        let usecase = DeleteRepo::new(
            Arc::new(orgs),
            Arc::new(repos),
            Arc::new(store),
            Arc::new(auth),
        );
        let executor = create_test_executor();
        let scope = MultiTenancy::new(None, Some(header));
        let result = usecase
            .execute(DeleteRepoInputData {
                executor: &executor,
                multi_tenancy: &scope,
                org_username: "victim-org".into(),
                repo_username: "victim-repo".into(),
            })
            .await;
        let deleted = deleted.lock().unwrap().clone();
        (result, deleted)
    }

    #[tokio::test]
    async fn a_policy_in_the_callers_own_org_cannot_delete_another_orgs_repo(
    ) {
        let (result, deleted) =
            delete_victim_repo(caller_org(), caller_org()).await;
        assert!(
            matches!(result, Err(errors::Error::Forbidden { .. })),
            "{result:?}"
        );
        assert!(deleted.is_empty(), "{deleted:?}");
    }

    #[tokio::test]
    async fn the_target_org_tenant_deletes_its_repo() {
        let (result, deleted) =
            delete_victim_repo(victim_org(), victim_org()).await;
        result.unwrap();
        assert_eq!(deleted, vec![victim_org()]);
    }

    #[tokio::test]
    async fn the_platform_tenant_header_is_checked_in_the_target_org() {
        let (result, deleted) =
            delete_victim_repo(LIBRARY_TENANT.clone(), victim_org()).await;
        result.unwrap();
        assert_eq!(deleted, vec![victim_org()]);
    }

    #[tokio::test]
    async fn a_policy_in_the_platform_tenant_cannot_delete_an_orgs_repo() {
        let (result, deleted) = delete_victim_repo(
            LIBRARY_TENANT.clone(),
            LIBRARY_TENANT.clone(),
        )
        .await;
        assert!(
            matches!(result, Err(errors::Error::Forbidden { .. })),
            "{result:?}"
        );
        assert!(deleted.is_empty(), "{deleted:?}");
    }
}
