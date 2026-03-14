use std::sync::Arc;

use crate::domain::{Source, SourceRepository};
use tachyon_sdk::auth::AuthApp;

use super::{
    GetOrganizationByUsernameQuery, GetRepoByUsernameQuery,
    UpdateSourceInputData, UpdateSourceInputPort,
};

#[derive(Debug, Clone)]
pub struct UpdateSource {
    source_repository: Arc<dyn SourceRepository>,
    auth: Arc<dyn AuthApp>,
    get_org_by_username: Arc<dyn GetOrganizationByUsernameQuery>,
    get_repo_by_username: Arc<dyn GetRepoByUsernameQuery>,
}

impl UpdateSource {
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
impl UpdateSourceInputPort for UpdateSource {
    #[tracing::instrument(name = "UpdateSource::execute", skip(self))]
    async fn execute<'a>(
        &self,
        input: UpdateSourceInputData<'a>,
    ) -> errors::Result<Source> {
        // TODO: add English comment
        self.auth
            .check_policy(&tachyon_sdk::auth::CheckPolicyInput::<'a> {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                action: "library:UpdateSource",
            })
            .await?;

        // TODO: add English comment
        let source = self
            .source_repository
            .get_by_id(input.source_id)
            .await?
            .ok_or_else(|| errors::Error::not_found("source"))?;

        // TODO: add English comment
        let org_username =
            input.org_username.parse::<value_object::Identifier>()?;
        let _org = self
            .get_org_by_username
            .execute(&org_username)
            .await?
            .ok_or(errors::Error::not_found("organization"))?;

        // TODO: add English comment
        let repo_username =
            input.repo_username.parse::<value_object::Identifier>()?;
        let _repo = self
            .get_repo_by_username
            .execute(&org_username, &repo_username)
            .await?
            .ok_or(errors::Error::not_found("repo"))?;

        // TODO: add English comment
        let updated_source = source.update(input.name, input.url);

        // TODO: add English comment
        self.source_repository.save(&updated_source).await?;

        Ok(updated_source)
    }
}

// TODO: add English comment
