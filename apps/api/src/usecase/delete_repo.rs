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
        self.auth
            .check_policy(&CheckPolicyInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                action: "library:DeleteRepo",
            })
            .await?;

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

        // repo.can_delete(user.id())?;

        self.repo_repo.delete(org.id(), repo.id()).await?;

        Ok(())
    }
}
