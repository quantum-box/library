use std::str::FromStr;
use std::sync::Arc;

use async_trait::async_trait;
use value_object::Identifier;

use crate::domain::{Repo, RepoRepository};
use crate::usecase::{
    GetOrganizationByUsernameQuery, GetRepoByUsernameQuery,
};
use std::fmt::Debug;

#[derive(Debug, Clone, async_graphql::InputObject)]
pub struct ChangeRepoUsernameInput {
    pub org_username: String,
    pub old_repo_username: String,
    pub new_repo_username: String,
}

#[async_trait]
pub trait ChangeRepoUsernameInputPort: Debug + Send + Sync {
    async fn execute(
        &self,
        input: ChangeRepoUsernameInput,
    ) -> errors::Result<Repo>;
}

#[derive(Debug, Clone)]
pub struct ChangeRepoUsername {
    get_org_by_username: Arc<dyn GetOrganizationByUsernameQuery>,
    get_repo_by_username: Arc<dyn GetRepoByUsernameQuery>,
    repo_repository: Arc<dyn RepoRepository>,
}

impl ChangeRepoUsername {
    pub fn new(
        get_org_by_username: Arc<dyn GetOrganizationByUsernameQuery>,
        get_repo_by_username: Arc<dyn GetRepoByUsernameQuery>,
        repo_repository: Arc<dyn RepoRepository>,
    ) -> Arc<Self> {
        Arc::new(Self {
            get_org_by_username,
            get_repo_by_username,
            repo_repository,
        })
    }
}

#[async_trait]
impl ChangeRepoUsernameInputPort for ChangeRepoUsername {
    async fn execute(
        &self,
        input: ChangeRepoUsernameInput,
    ) -> errors::Result<Repo> {
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

    #[tokio::test]
    async fn test_change_repo_username_success() {
        let mut mock_org_query = MockGetOrgByUsername::new();
        let mut mock_repo_query = MockGetRepoByUsername::new();
        let mut mock_repo_repo = MockRepoRepo::new();

        let org_username = "test-org".to_string();
        let old_repo_username = "old-repo".to_string();
        let new_repo_username = "new-repo".to_string();

        let org = Organization::new(
            &TenantId::default(),
            &Text::new("Test Organization").unwrap(),
            &Identifier::from_str(&org_username).unwrap(),
            None,
            None,
        );

        let repo = Repo::new(
            &RepoId::default(),
            &OperatorId::default(),
            &Identifier::from_str(&org_username).unwrap(),
            &Text::new("test-repo").unwrap(),
            &Identifier::from_str(&old_repo_username).unwrap(),
            true,
            None,
            vec![DatabaseId::default()],
            vec![], // tags
        );

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

        let usecase = ChangeRepoUsername::new(
            Arc::new(mock_org_query),
            Arc::new(mock_repo_query),
            Arc::new(mock_repo_repo),
        );

        let result = usecase
            .execute(ChangeRepoUsernameInput {
                org_username: org_username.clone(),
                old_repo_username: old_repo_username.clone(),
                new_repo_username: new_repo_username.clone(),
            })
            .await;

        assert!(result.is_ok());
        let updated_repo = result.unwrap();
        assert_eq!(updated_repo.username().to_string(), new_repo_username);
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
        );

        let result = usecase
            .execute(ChangeRepoUsernameInput {
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
        let org = Organization::new(
            &TenantId::default(),
            &Text::new("Test Organization").unwrap(),
            &Identifier::from_str(&org_username).unwrap(),
            None,
            None,
        );

        mock_org_query
            .expect_execute()
            .returning(move |_| Ok(Some(org.clone())));

        mock_repo_query.expect_execute().returning(|_, _| Ok(None));

        let usecase = ChangeRepoUsername::new(
            Arc::new(mock_org_query),
            Arc::new(mock_repo_query),
            Arc::new(mock_repo_repo),
        );

        let result = usecase
            .execute(ChangeRepoUsernameInput {
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
        let mut mock_org_query = MockGetOrgByUsername::new();
        let mut mock_repo_query = MockGetRepoByUsername::new();
        let mut mock_repo_repo = MockRepoRepo::new();

        let org_username = "test-org".to_string();
        let old_repo_username = "old-repo".to_string();
        let org = Organization::new(
            &TenantId::default(),
            &Text::new("Test Organization").unwrap(),
            &Identifier::from_str(&org_username).unwrap(),
            None,
            None,
        );

        let repo = Repo::new(
            &RepoId::default(),
            &OperatorId::default(),
            &Identifier::from_str(&org_username).unwrap(),
            &Text::new("test-repo").unwrap(),
            &Identifier::from_str(&old_repo_username).unwrap(),
            true,
            None,
            vec![DatabaseId::default()],
            vec![], // tags
        );

        mock_org_query
            .expect_execute()
            .returning(move |_| Ok(Some(org.clone())));

        mock_repo_query
            .expect_execute()
            .with(
                eq(Identifier::from_str(&org_username).unwrap()),
                eq(Identifier::from_str(&old_repo_username).unwrap()),
            )
            .returning(move |_, _| Ok(Some(repo.clone())));

        mock_repo_repo.expect_save().returning(|_| Ok(()));

        let usecase = ChangeRepoUsername::new(
            Arc::new(mock_org_query),
            Arc::new(mock_repo_query),
            Arc::new(mock_repo_repo),
        );

        let result = usecase
            .execute(ChangeRepoUsernameInput {
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
