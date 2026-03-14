use std::sync::Arc;

use crate::domain::SourceRepository;
use tachyon_sdk::auth::AuthApp;

use super::{
    DeleteSourceInputData, DeleteSourceInputPort,
    GetOrganizationByUsernameQuery, GetRepoByUsernameQuery,
};

#[derive(Debug, Clone)]
pub struct DeleteSource {
    source_repository: Arc<dyn SourceRepository>,
    auth: Arc<dyn AuthApp>,
    get_org_by_username: Arc<dyn GetOrganizationByUsernameQuery>,
    get_repo_by_username: Arc<dyn GetRepoByUsernameQuery>,
}

impl DeleteSource {
    pub fn new(
        source_repository: Arc<dyn SourceRepository>,
        auth: Arc<dyn AuthApp>,
        get_org_by_username: Arc<dyn GetOrganizationByUsernameQuery>,
        get_repo_by_username: Arc<dyn GetRepoByUsernameQuery>,
    ) -> Self {
        Self {
            source_repository,
            auth,
            get_org_by_username,
            get_repo_by_username,
        }
    }
}

#[async_trait::async_trait]
impl DeleteSourceInputPort for DeleteSource {
    #[tracing::instrument(name = "DeleteSource::execute", skip(self))]
    async fn execute<'a>(
        &self,
        input: DeleteSourceInputData<'a>,
    ) -> errors::Result<()> {
        // TODO: add English comment
        self.auth
            .check_policy(&tachyon_sdk::auth::CheckPolicyInput::<'a> {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                action: "library:DeleteSource",
            })
            .await?;

        // TODO: add English comment
        let org_username =
            input.org_username.parse::<value_object::Identifier>()?;
        let _org = self
            .get_org_by_username
            .execute(&org_username)
            .await?
            .ok_or(errors::not_found!(
            "organization not found in delete source"
        ))?;

        // TODO: add English comment
        let repo_username =
            input.repo_username.parse::<value_object::Identifier>()?;
        let _repo = self
            .get_repo_by_username
            .execute(&org_username, &repo_username)
            .await?
            .ok_or(errors::not_found!("repo not found in delete source"))?;

        // TODO: add English comment
        self.source_repository.delete(input.source_id).await?;

        Ok(())
    }
}

// TODO: add English comment
