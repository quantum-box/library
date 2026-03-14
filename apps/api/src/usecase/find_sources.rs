use std::sync::Arc;

use crate::domain::{Source, SourceRepository, VisibilityService};
use derive_new::new;
use tachyon_sdk::auth::AuthApp;

use super::FindSourcesInputData;
use super::FindSourcesInputPort;

#[derive(Debug, Clone, new)]
pub struct FindSources {
    source_repository: Arc<dyn SourceRepository>,
    auth: Arc<dyn AuthApp>,
    get_org_by_username: Arc<dyn super::GetOrganizationByUsernameQuery>,
    get_repo_by_username: Arc<dyn super::GetRepoByUsernameQuery>,
    visibility_service: Arc<VisibilityService>,
}

#[async_trait::async_trait]
impl FindSourcesInputPort for FindSources {
    #[tracing::instrument(name = "FindSources::execute", skip(self))]
    async fn execute<'a>(
        &self,
        input: FindSourcesInputData<'a>,
    ) -> errors::Result<Vec<Source>> {
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
        let repo = self
            .get_repo_by_username
            .execute(&org_username, &repo_username)
            .await?
            .ok_or_else(|| errors::Error::not_found("repository"))?;

        // TODO: add English comment
        if repo.id() != input.repo_id {
            return Err(errors::Error::not_found("repository"));
        }

        // TODO: add English comment
        // TODO: add English comment
        let need_check = self
            .visibility_service
            .check_access(&repo, input.executor)?;

        // TODO: add English comment
        if need_check && !input.executor.is_none() {
            self.auth
                .check_policy(&tachyon_sdk::auth::CheckPolicyInput::<'a> {
                    executor: input.executor,
                    multi_tenancy: input.multi_tenancy,
                    action: "library:FindSources",
                })
                .await?;
        }

        // TODO: add English comment
        let sources = self
            .source_repository
            .find_by_repo_id(input.repo_id)
            .await?;

        Ok(sources)
    }
}

// TODO: add English comment
