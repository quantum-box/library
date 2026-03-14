use std::sync::Arc;

use crate::domain::{Source, SourceRepository};
use tachyon_sdk::auth::AuthApp;

use super::{
    CreateSourceInputData, CreateSourceInputPort,
    GetOrganizationByUsernameQuery, GetRepoByUsernameQuery,
};

#[derive(Debug, Clone)]
pub struct CreateSource {
    source_repository: Arc<dyn SourceRepository>,
    auth: Arc<dyn AuthApp>,
    get_org_by_username: Arc<dyn GetOrganizationByUsernameQuery>,
    get_repo_by_username: Arc<dyn GetRepoByUsernameQuery>,
}

impl CreateSource {
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
impl CreateSourceInputPort for CreateSource {
    #[tracing::instrument(name = "CreateSource::execute", skip(self))]
    async fn execute<'a>(
        &self,
        input: CreateSourceInputData<'a>,
    ) -> errors::Result<Source> {
        // TODO: add English comment
        self.auth
            .check_policy(&tachyon_sdk::auth::CheckPolicyInput::<'a> {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                action: "library:CreateSource",
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
            "organization not found in create source"
        ))?;

        // TODO: add English comment
        let repo_username =
            input.repo_username.parse::<value_object::Identifier>()?;
        let repo = self
            .get_repo_by_username
            .execute(&org_username, &repo_username)
            .await?
            .ok_or(errors::not_found!("repo not found in create source"))?;

        // TODO: add English comment
        let source =
            Source::create(repo.id(), input.name, input.url.clone());

        // TODO: add English comment
        self.source_repository.save(&source).await?;

        Ok(source)
    }
}

// TODO: add English comment
