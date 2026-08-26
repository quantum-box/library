//! Sync a single Data item's markdown to a GitHub repository.

use std::sync::Arc;

use database_manager::usecase::FindAllPropertiesInputData;
use outbound_sync::{
    SyncDataInputData, SyncDataInputPort, SyncPayload, SyncTarget,
};
use tachyon_sdk::auth::{AuthApp, CheckPolicyInput};

use crate::usecase::{
    SyncDataToGithubInputData, SyncDataToGithubInputPort,
    SyncDataToGithubOutputData,
};

#[derive(Clone)]
pub struct SyncDataToGithub {
    get_org_by_username:
        Arc<dyn crate::usecase::GetOrganizationByUsernameQuery>,
    get_repo_by_username: Arc<dyn crate::usecase::GetRepoByUsernameQuery>,
    auth: Arc<dyn AuthApp>,
    database: Arc<database_manager::App>,
    sync_data: Arc<dyn SyncDataInputPort>,
}

impl std::fmt::Debug for SyncDataToGithub {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SyncDataToGithub").finish_non_exhaustive()
    }
}

impl SyncDataToGithub {
    pub fn new(
        get_org_by_username: Arc<
            dyn crate::usecase::GetOrganizationByUsernameQuery,
        >,
        get_repo_by_username: Arc<
            dyn crate::usecase::GetRepoByUsernameQuery,
        >,
        auth: Arc<dyn AuthApp>,
        database: Arc<database_manager::App>,
        sync_data: Arc<dyn SyncDataInputPort>,
    ) -> Arc<Self> {
        Arc::new(Self {
            get_org_by_username,
            get_repo_by_username,
            auth,
            database,
            sync_data,
        })
    }
}

#[async_trait::async_trait]
impl SyncDataToGithubInputPort for SyncDataToGithub {
    #[tracing::instrument(name = "SyncDataToGithub::execute", skip(self))]
    async fn execute<'a>(
        &self,
        input: SyncDataToGithubInputData<'a>,
    ) -> errors::Result<SyncDataToGithubOutputData> {
        let org = self
            .get_org_by_username
            .execute(&input.org_username.parse()?)
            .await
            .map_err(|e| {
                errors::Error::application_logic_error(e.to_string())
            })?
            .ok_or(errors::Error::not_found("organization"))?;

        let repo = self
            .get_repo_by_username
            .execute(org.username(), &input.repo_username.parse()?)
            .await
            .map_err(|e| {
                errors::Error::application_logic_error(e.to_string())
            })?
            .ok_or(errors::Error::not_found("repo"))?;

        self.auth
            .check_policy(&CheckPolicyInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                action: "library:UpdateRepo",
            })
            .await?;

        let database_id = repo
            .databases()
            .first()
            .ok_or_else(|| {
                errors::Error::application_logic_error(
                    "Repository has no associated database",
                )
            })?
            .clone();

        let properties = self
            .database
            .find_all_properties()
            .execute(FindAllPropertiesInputData {
                tenant_id: org.id().clone(),
                database_id: database_id.clone(),
            })
            .await?;

        let data = self
            .database
            .get_data_usecase()
            .execute(&database_manager::GetDataInputData {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                tenant_id: org.id(),
                database_id: &database_id,
                data_id: &input.data_id.parse()?,
            })
            .await?;

        let markdown = crate::usecase::markdown_composer::compose_markdown(
            &data,
            &properties,
        );

        let branch = input
            .target_branch
            .clone()
            .filter(|b| !b.trim().is_empty())
            .unwrap_or_else(|| "main".to_string());
        let target = SyncTarget::git_with_branch(
            &input.target_repo,
            &input.target_path,
            branch,
        );

        let message = input
            .commit_message
            .clone()
            .filter(|m| !m.trim().is_empty())
            .unwrap_or_else(|| {
                format!("chore(library): sync {}", data.name().as_str())
            });
        let payload =
            SyncPayload::markdown_with_message(&markdown, &message);

        let result = self
            .sync_data
            .execute(&SyncDataInputData {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                data_id: data.id().to_string(),
                provider: "github".to_string(),
                target,
                payload,
                dry_run: input.dry_run,
            })
            .await?;

        Ok(SyncDataToGithubOutputData {
            status: result.status,
            result_id: result.result_id,
            url: result.url,
            diff: result.diff,
        })
    }
}
