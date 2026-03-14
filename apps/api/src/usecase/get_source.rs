use std::sync::Arc;

use crate::domain::{Source, SourceRepository, VisibilityService};
use derive_new::new;
use tachyon_sdk::auth::AuthApp;

use super::GetSourceInputData;
use super::GetSourceInputPort;

#[derive(Debug, Clone, new)]
pub struct GetSource {
    source_repository: Arc<dyn SourceRepository>,
    auth: Arc<dyn AuthApp>,
    get_org_by_username: Arc<dyn super::GetOrganizationByUsernameQuery>,
    get_repo_by_username: Arc<dyn super::GetRepoByUsernameQuery>,
    visibility_service: Arc<VisibilityService>,
}

#[async_trait::async_trait]
impl GetSourceInputPort for GetSource {
    #[tracing::instrument(name = "GetSource::execute", skip(self))]
    async fn execute<'a>(
        &self,
        input: GetSourceInputData<'a>,
    ) -> errors::Result<Option<Source>> {
        // TODO: add English comment
        let org_username =
            input.org_username.parse::<value_object::Identifier>()?;
        let _org =
            self.get_org_by_username
                .execute(&org_username)
                .await?
                .ok_or_else(|| errors::Error::not_found("organization"))?;

        // TODO: add English comment
        let repo_username =
            input.repo_username.parse::<value_object::Identifier>()?;
        let repo = self
            .get_repo_by_username
            .execute(&org_username, &repo_username)
            .await?
            .ok_or_else(|| errors::Error::not_found("repository"))?;

        // TODO: add English comment
        let source_opt =
            self.source_repository.get_by_id(input.source_id).await?;

        // TODO: add English comment
        let source = match source_opt {
            Some(s) => s,
            None => return Ok(None),
        };

        // TODO: add English comment
        if source.repo_id() != repo.id() {
            return Err(errors::Error::not_found("source"));
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
                    action: "library:GetSource",
                })
                .await?;
        }

        Ok(Some(source))
    }
}

// TODO: add English comment
