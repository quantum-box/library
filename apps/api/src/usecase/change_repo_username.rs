use std::str::FromStr;
use std::sync::Arc;

use async_trait::async_trait;
use tachyon_sdk::auth::{
    AuthApp, CheckPolicyForResourceInput, ExecutorAction,
    MultiTenancyAction,
};
use value_object::Identifier;

use crate::domain::{Repo, RepoRepository};
use crate::usecase::{
    GetOrganizationByUsernameQuery, GetRepoByUsernameQuery,
};
use std::fmt::Debug;

/// Input for renaming a repository.
///
/// Carries the caller's `executor` / `multi_tenancy` so the use case can
/// authorize the rename itself, the same way `DeleteRepo` does.
#[derive(Debug, Clone)]
pub struct ChangeRepoUsernameInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,

    pub org_username: String,
    pub old_repo_username: String,
    pub new_repo_username: String,
}

#[async_trait]
pub trait ChangeRepoUsernameInputPort: Debug + Send + Sync {
    async fn execute<'a>(
        &self,
        input: ChangeRepoUsernameInputData<'a>,
    ) -> errors::Result<Repo>;
}

#[derive(Debug, Clone)]
pub struct ChangeRepoUsername {
    get_org_by_username: Arc<dyn GetOrganizationByUsernameQuery>,
    get_repo_by_username: Arc<dyn GetRepoByUsernameQuery>,
    repo_repository: Arc<dyn RepoRepository>,
    auth: Arc<dyn AuthApp>,
}

impl ChangeRepoUsername {
    pub fn new(
        get_org_by_username: Arc<dyn GetOrganizationByUsernameQuery>,
        get_repo_by_username: Arc<dyn GetRepoByUsernameQuery>,
        repo_repository: Arc<dyn RepoRepository>,
        auth: Arc<dyn AuthApp>,
    ) -> Arc<Self> {
        Arc::new(Self {
            get_org_by_username,
            get_repo_by_username,
            repo_repository,
            auth,
        })
    }
}

#[async_trait]
impl ChangeRepoUsernameInputPort for ChangeRepoUsername {
    /// Rename a repository after a resource-level permission check.
    ///
    /// Renaming is an update of the repo, so it is authorized with the
    /// same `library:UpdateRepo` action and repo TRN that `UpdateRepo`
    /// uses: repo owners and writers pass, everyone else is rejected.
    #[tracing::instrument(name = "ChangeRepoUsername::execute", skip(self))]
    async fn execute<'a>(
        &self,
        input: ChangeRepoUsernameInputData<'a>,
    ) -> errors::Result<Repo> {
        if input.executor.is_none() {
            return Err(errors::permission_denied!(
                "execute user is required"
            ));
        }

        // Parse and validate new username first
        let new_repo_username =
            Identifier::from_str(&input.new_repo_username)?;

        // Parse other usernames
        let org_username = Identifier::from_str(&input.org_username)?;
        let old_repo_username =
            Identifier::from_str(&input.old_repo_username)?;

        // Get organization
        let _org = self
            .get_org_by_username
            .execute(&org_username)
            .await?
            .ok_or(errors::Error::not_found(
            "Organization not found",
        ))?;

        // Get repository
        let repo = self
            .get_repo_by_username
            .execute(&org_username, &old_repo_username)
            .await?
            .ok_or(errors::Error::not_found("Repository not found"))?;

        // Check resource-based write permission before mutating anything.
        let resource_trn = format!("trn:library:repo:{}", repo.id());
        self.auth
            .check_policy_for_resource(&CheckPolicyForResourceInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                action: "library:UpdateRepo",
                resource_trn: &resource_trn,
            })
            .await?;

        // Update repository with new username
        let updated_repo = repo.with_operator_alias(&new_repo_username);

        // Save changes
        self.repo_repository.save(&updated_repo).await?;

        Ok(updated_repo)
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::{Organization, RepoId, RepoRepository};

    use super::*;
    use database_manager::domain::DatabaseId;
    use mockall::mock;
    use mockall::predicate::*;
    use std::str::FromStr;
    use std::sync::Mutex;
    use tachyon_sdk::auth::{
        test_helper::{create_test_executor, create_test_multi_tenancy},
        MockAuthApp,
    };
    use value_object::OperatorId;
    use value_object::TenantId;
    use value_object::Text;

    mock! {
        #[derive(Debug)]
        GetOrgByUsername {}
        #[async_trait]
        impl GetOrganizationByUsernameQuery for GetOrgByUsername {
            async fn execute(&self, username: &Identifier) -> errors::Result<Option<Organization>>;
        }
    }

    mock! {
        #[derive(Debug)]
        GetRepoByUsername {}
        #[async_trait]
        impl GetRepoByUsernameQuery for GetRepoByUsername {
            async fn execute(&self, org_username: &Identifier, repo_username: &Identifier) -> errors::Result<Option<Repo>>;
        }
    }

    mock! {
        #[derive(Debug)]
        RepoRepo {}
        #[async_trait]
        impl RepoRepository for RepoRepo {
            async fn save(&self, entity: &Repo) -> errors::Result<()>;
            async fn get_by_id(&self, tenant_id: &value_object::TenantId, id: &RepoId) -> errors::Result<Option<Repo>> {
                Ok(None)
            }
            async fn find_all(&self, tenant_id: &value_object::TenantId) -> errors::Result<Vec<Repo>> {
                Ok(vec![])
            }
            async fn delete(&self, tenant_id: &value_object::TenantId, id: &RepoId) -> errors::Result<()> {
                Ok(())
            }
        }
    }

    /// Auth stub that records every resource check and either allows or
    /// denies it, mirroring `private_repo_access`'s test helper.
    fn mock_auth(
        calls: Arc<Mutex<Vec<String>>>,
        allow: bool,
    ) -> MockAuthApp {
        let mut auth = MockAuthApp::new();
        auth.expect_check_policy_for_resource().returning({
            move |input| {
                calls.lock().unwrap().push(format!(
                    "{}:{}",
                    input.action, input.resource_trn
                ));
                Box::pin(async move {
                    if allow {
                        Ok(())
                    } else {
                        Err(errors::Error::forbidden("denied"))
                    }
                })
            }
        });
        auth
    }

    fn test_org(org_username: &str) -> Organization {
        Organization::new(
            &TenantId::default(),
            &Text::new("Test Organization").unwrap(),
            &Identifier::from_str(org_username).unwrap(),
            None,
            None,
        )
    }

    fn test_repo(
        repo_id: &RepoId,
        org_username: &str,
        repo_username: &str,
    ) -> Repo {
        Repo::new(
            repo_id,
            &OperatorId::default(),
            &Identifier::from_str(org_username).unwrap(),
            &Text::new("test-repo").unwrap(),
            &Identifier::from_str(repo_username).unwrap(),
            true,
            None,
            vec![DatabaseId::default()],
            vec![], // tags
        )
    }

    #[tokio::test]
    async fn test_change_repo_username_success() {
        let mut mock_org_query = MockGetOrgByUsername::new();
        let mut mock_repo_query = MockGetRepoByUsername::new();
        let mut mock_repo_repo = MockRepoRepo::new();

        let org_username = "test-org".to_string();
        let old_repo_username = "old-repo".to_string();
        let new_repo_username = "new-repo".to_string();

        let org = test_org(&org_username);
        let repo_id = RepoId::default();
        let repo = test_repo(&repo_id, &org_username, &old_repo_username);

        mock_org_query
            .expect_execute()
            .with(eq(Identifier::from_str(&org_username).unwrap()))
            .returning(move |_| Ok(Some(org.clone())));

        mock_repo_query
            .expect_execute()
            .with(
                eq(Identifier::from_str(&org_username).unwrap()),
                eq(Identifier::from_str(&old_repo_username).unwrap()),
            )
            .returning(move |_, _| Ok(Some(repo.clone())));

        mock_repo_repo.expect_save().returning(|_| Ok(()));

        let calls = Arc::new(Mutex::new(Vec::new()));
        let usecase = ChangeRepoUsername::new(
            Arc::new(mock_org_query),
            Arc::new(mock_repo_query),
            Arc::new(mock_repo_repo),
            Arc::new(mock_auth(calls.clone(), true)),
        );

        let executor = create_test_executor();
        let multi_tenancy = create_test_multi_tenancy();
        let result = usecase
            .execute(ChangeRepoUsernameInputData {
                executor: &executor,
                multi_tenancy: &multi_tenancy,
                org_username: org_username.clone(),
                old_repo_username: old_repo_username.clone(),
                new_repo_username: new_repo_username.clone(),
            })
            .await;

        assert!(result.is_ok());
        let updated_repo = result.unwrap();
        assert_eq!(updated_repo.username().to_string(), new_repo_username);
        assert_eq!(
            calls.lock().unwrap().as_slice(),
            &[format!("library:UpdateRepo:trn:library:repo:{repo_id}")]
        );
    }

    /// An executor without write permission on the repo must not be able
    /// to rename it, and the repository must never be saved.
    #[tokio::test]
    async fn test_change_repo_username_forbidden_for_unauthorized_executor()
    {
        let mut mock_org_query = MockGetOrgByUsername::new();
        let mut mock_repo_query = MockGetRepoByUsername::new();
        let mut mock_repo_repo = MockRepoRepo::new();

        let org_username = "test-org".to_string();
        let old_repo_username = "old-repo".to_string();

        let org = test_org(&org_username);
        let repo_id = RepoId::default();
        let repo = test_repo(&repo_id, &org_username, &old_repo_username);

        mock_org_query
            .expect_execute()
            .returning(move |_| Ok(Some(org.clone())));
        mock_repo_query
            .expect_execute()
            .returning(move |_, _| Ok(Some(repo.clone())));
        // A denied rename must not reach the repository.
        mock_repo_repo.expect_save().never();

        let calls = Arc::new(Mutex::new(Vec::new()));
        let usecase = ChangeRepoUsername::new(
            Arc::new(mock_org_query),
            Arc::new(mock_repo_query),
            Arc::new(mock_repo_repo),
            Arc::new(mock_auth(calls.clone(), false)),
        );

        let executor = create_test_executor();
        let multi_tenancy = create_test_multi_tenancy();
        let result = usecase
            .execute(ChangeRepoUsernameInputData {
                executor: &executor,
                multi_tenancy: &multi_tenancy,
                org_username: org_username.clone(),
                old_repo_username: old_repo_username.clone(),
                new_repo_username: "new-repo".to_string(),
            })
            .await;

        assert!(matches!(
            result.unwrap_err(),
            errors::Error::Forbidden { .. }
        ));
        assert_eq!(
            calls.lock().unwrap().as_slice(),
            &[format!("library:UpdateRepo:trn:library:repo:{repo_id}")]
        );
    }

    #[tokio::test]
    async fn test_change_repo_username_invalid_org() {
        let mut mock_org_query = MockGetOrgByUsername::new();
        let mock_repo_query = MockGetRepoByUsername::new();
        let mock_repo_repo = MockRepoRepo::new();

        mock_org_query.expect_execute().returning(|_| Ok(None));

        let usecase = ChangeRepoUsername::new(
            Arc::new(mock_org_query),
            Arc::new(mock_repo_query),
            Arc::new(mock_repo_repo),
            Arc::new(mock_auth(Arc::new(Mutex::new(Vec::new())), true)),
        );

        let executor = create_test_executor();
        let multi_tenancy = create_test_multi_tenancy();
        let result = usecase
            .execute(ChangeRepoUsernameInputData {
                executor: &executor,
                multi_tenancy: &multi_tenancy,
                org_username: "non-existent".to_string(),
                old_repo_username: "old-repo".to_string(),
                new_repo_username: "new-repo".to_string(),
            })
            .await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            errors::Error::NotFound { .. }
        ));
    }

    #[tokio::test]
    async fn test_change_repo_username_invalid_repo() {
        let mut mock_org_query = MockGetOrgByUsername::new();
        let mut mock_repo_query = MockGetRepoByUsername::new();
        let mock_repo_repo = MockRepoRepo::new();

        let org_username = "test-org".to_string();
        let org = test_org(&org_username);

        mock_org_query
            .expect_execute()
            .returning(move |_| Ok(Some(org.clone())));

        mock_repo_query.expect_execute().returning(|_, _| Ok(None));

        let usecase = ChangeRepoUsername::new(
            Arc::new(mock_org_query),
            Arc::new(mock_repo_query),
            Arc::new(mock_repo_repo),
            Arc::new(mock_auth(Arc::new(Mutex::new(Vec::new())), true)),
        );

        let executor = create_test_executor();
        let multi_tenancy = create_test_multi_tenancy();
        let result = usecase
            .execute(ChangeRepoUsernameInputData {
                executor: &executor,
                multi_tenancy: &multi_tenancy,
                org_username,
                old_repo_username: "non-existent".to_string(),
                new_repo_username: "new-repo".to_string(),
            })
            .await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            errors::Error::NotFound { .. }
        ));
    }

    #[tokio::test]
    async fn test_change_repo_username_invalid_new_username() {
        let mock_org_query = MockGetOrgByUsername::new();
        let mock_repo_query = MockGetRepoByUsername::new();
        let mut mock_repo_repo = MockRepoRepo::new();

        mock_repo_repo.expect_save().never();

        let usecase = ChangeRepoUsername::new(
            Arc::new(mock_org_query),
            Arc::new(mock_repo_query),
            Arc::new(mock_repo_repo),
            Arc::new(mock_auth(Arc::new(Mutex::new(Vec::new())), true)),
        );

        let executor = create_test_executor();
        let multi_tenancy = create_test_multi_tenancy();
        let result = usecase
            .execute(ChangeRepoUsernameInputData {
                executor: &executor,
                multi_tenancy: &multi_tenancy,
                org_username: "test-org".to_string(),
                old_repo_username: "old-repo".to_string(),
                new_repo_username: "-invalid".to_string(), // Invalid: starts with hyphen
            })
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, errors::Error::BadRequest { .. }));
        assert!(err.to_string().contains(
            "username cannot start or end with hyphens or underscores"
        ));
    }
}
