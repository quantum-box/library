use std::{fmt::Debug, sync::Arc};

use super::{
    ViewOrgInputData, ViewOrgOutputData, ViewOrganizationInputPort,
};
use crate::{
    domain::RepoRepository, usecase::GetOrganizationByUsernameQuery,
};

#[derive(Debug, Clone)]
pub struct ViewOrganization {
    get_organization_by_username: Arc<dyn GetOrganizationByUsernameQuery>,
    repo_repo: Arc<dyn RepoRepository>,
}

impl ViewOrganization {
    pub fn new(
        get_organization_by_username: Arc<
            dyn GetOrganizationByUsernameQuery,
        >,
        repo_repo: Arc<dyn RepoRepository>,
    ) -> Arc<Self> {
        Arc::new(Self {
            get_organization_by_username,
            repo_repo,
        })
    }
}

#[async_trait::async_trait]
impl ViewOrganizationInputPort for ViewOrganization {
    #[tracing::instrument(name = "ViewOrganization::execute", skip(self))]
    async fn execute<'a>(
        &self,
        input: &ViewOrgInputData<'a>,
    ) -> errors::Result<ViewOrgOutputData> {
        let org = self
            .get_organization_by_username
            .execute(&input.organization_username.parse()?)
            .await?
            .ok_or(errors::not_found!("organization is not found"))?;
        let repos = self.repo_repo.find_all(org.id()).await?;

        if input.executor.is_none() {
            return Ok(ViewOrgOutputData {
                organization: org,
                repos: repos
                    .into_iter()
                    .filter(|r| *r.is_public())
                    .collect(),
            });
        }

        Ok(ViewOrgOutputData {
            organization: org,
            repos,
        })
    }
}
