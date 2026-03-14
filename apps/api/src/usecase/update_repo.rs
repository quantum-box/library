use crate::domain::{Repo, RepoRepository};
use crate::usecase::{
    GetOrganizationByUsernameQuery, GetRepoByUsernameQuery,
    UpdateRepoInputData, UpdateRepoInputPort,
};
use std::sync::Arc;
use tachyon_sdk::auth::{AuthApp, CheckPolicyForResourceInput};

#[derive(Debug, Clone)]
pub struct UpdateRepo {
    get_org_by_username: Arc<dyn GetOrganizationByUsernameQuery>,
    get_repo_by_username: Arc<dyn GetRepoByUsernameQuery>,
    auth: Arc<dyn AuthApp>,
    repo_repository: Arc<dyn RepoRepository>,
}

impl UpdateRepo {
    pub fn new(
        get_org_by_username: Arc<dyn GetOrganizationByUsernameQuery>,
        get_repo_by_username: Arc<dyn GetRepoByUsernameQuery>,
        auth: Arc<dyn AuthApp>,
        repo_repository: Arc<dyn RepoRepository>,
    ) -> Arc<Self> {
        Arc::new(Self {
            get_org_by_username,
            get_repo_by_username,
            auth,
            repo_repository,
        })
    }
}

#[async_trait::async_trait]
impl UpdateRepoInputPort for UpdateRepo {
    /// Update repository settings with resource-level permission check.
    ///
    /// This method checks for write access using resource-based policy:
    /// 1. First tries resource-specific access (for invited repo members)
    /// 2. Falls back to org-wide access (for org owners with
    ///    pol_01libraryrepoowner)
    #[tracing::instrument(name = "UpdateRepo::execute", skip(self))]
    async fn execute<'a>(
        &self,
        input: UpdateRepoInputData<'a>,
    ) -> errors::Result<Repo> {
        if input.executor.is_none() {
            return Err(errors::permission_denied!(
                "execute user is required"
            ));
        }

        let org = self
            .get_org_by_username
            .execute(&input.org_username.parse()?)
            .await
            .map_err(|e| {
                errors::Error::application_logic_error(e.to_string())
            })?
            .ok_or(errors::not_found!("organization"))?;
        let repo = self
            .get_repo_by_username
            .execute(org.username(), &input.repo_username.parse()?)
            .await
            .map_err(|e| {
                errors::Error::application_logic_error(e.to_string())
            })?
            .ok_or(errors::not_found!("repo"))?;

        // Check resource-based write permission
        // - pol_01libraryrepoowner has resource_pattern "trn:library:repo:*"
        // - pol_01libraryrepowriter has specific repo resource_scope
        // - pol_01libraryuserpolicy has no resource pattern → won't match
        let resource_trn = format!("trn:library:repo:{}", repo.id());
        self.auth
            .check_policy_for_resource(&CheckPolicyForResourceInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                action: "library:UpdateRepo",
                resource_trn: &resource_trn,
            })
            .await?;

        let repo = repo.update(
            input.name.clone(),
            input.description.clone(),
            input.is_public,
            input.tags.clone(),
        );

        self.repo_repository.save(&repo).await?;
        Ok(repo)
    }
}
