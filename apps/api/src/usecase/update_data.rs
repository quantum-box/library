use database_manager::usecase::FindAllPropertiesInputData;
use outbound_sync::{
    SyncDataInputData, SyncDataInputPort, SyncPayload, SyncTarget,
};

use crate::handler::data::{compose_markdown, extract_ext_github};
use crate::usecase::{UpdateDataInputData, UpdateDataInputPort};
use std::sync::Arc;
use tachyon_sdk::auth::{AuthApp, CheckPolicyInput};

use super::PropertyDataValueInputData;

#[derive(Clone)]
pub struct UpdateData {
    get_org_by_username:
        Arc<dyn crate::usecase::GetOrganizationByUsernameQuery>,
    get_repo_by_username: Arc<dyn crate::usecase::GetRepoByUsernameQuery>,
    auth: Arc<dyn AuthApp>,
    database: Arc<database_manager::App>,
    sync_data: Arc<dyn SyncDataInputPort>,
}

impl std::fmt::Debug for UpdateData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UpdateData")
            .field("sync_data", &"<SyncDataInputPort>")
            .finish_non_exhaustive()
    }
}

impl UpdateData {
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
impl UpdateDataInputPort for UpdateData {
    #[tracing::instrument(name = "UpdateData::execute", skip(self))]
    async fn execute<'a>(
        &self,
        input: UpdateDataInputData<'a>,
    ) -> errors::Result<(
        database_manager::domain::Data,
        Vec<database_manager::domain::Property>,
    )> {
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
        let property_data = input
            .property_data
            .iter()
            .map(|data| {
                let value = match &data.value {
                    PropertyDataValueInputData::String(s) => s.clone(),
                    PropertyDataValueInputData::Integer(s) => s.clone(),
                    PropertyDataValueInputData::Html(s) => s.clone(),
                    PropertyDataValueInputData::Markdown(s) => s.clone(),
                    PropertyDataValueInputData::Relation(s) => s.join(","),
                    PropertyDataValueInputData::Select(s) => s.clone(),
                    PropertyDataValueInputData::MultiSelect(s) => {
                        s.join(",")
                    }
                    PropertyDataValueInputData::Location(l) => {
                        format!("{},{}", l.latitude(), l.longitude())
                    }
                    PropertyDataValueInputData::Date(s) => s.clone(),
                    PropertyDataValueInputData::Image(s) => s.clone(),
                };
                database_manager::PropertyDataInputData {
                    property_id: data.property_id.clone(),
                    value,
                }
            })
            .collect::<Vec<_>>();

        let data = self
            .database
            .update_data_usecase()
            .execute(database_manager::UpdateDataInputData {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                tenant_id: org.id(),
                database_id: &database_id,
                data_id: &input.data_id.parse()?,
                name: input.data_name,
                data: property_data,
            })
            .await
            .map_err(|e| {
                errors::Error::application_logic_error(e.to_string())
            })?;

        // Check for ext_github and trigger auto-sync if enabled
        if let Some(ext_github) = extract_ext_github(&data, &properties) {
            if ext_github.enabled {
                tracing::info!(
                    "Auto-syncing data {} to GitHub: {}/{}",
                    data.id(),
                    ext_github.repo,
                    ext_github.path
                );

                // Build markdown
                let markdown = compose_markdown(&data, &properties);

                // Build sync target
                let target = SyncTarget::git_with_branch(
                    &ext_github.repo,
                    &ext_github.path,
                    "main".to_string(),
                );

                // Build payload
                let message =
                    format!("chore(library): auto-sync {}", data.name());
                let payload =
                    SyncPayload::markdown_with_message(&markdown, &message);

                // Execute sync (non-blocking, log errors)
                match self
                    .sync_data
                    .execute(&SyncDataInputData {
                        executor: input.executor,
                        multi_tenancy: input.multi_tenancy,
                        data_id: data.id().to_string(),
                        provider: "github".to_string(),
                        target,
                        payload,
                        dry_run: false,
                    })
                    .await
                {
                    Ok(result) => {
                        tracing::info!(
                            "Auto-sync successful: {:?}",
                            result.result_id
                        );
                    }
                    Err(e) => {
                        tracing::warn!("Auto-sync failed: {:?}", e);
                        // Don't fail the update operation, just log the error
                    }
                }
            }
        }

        Ok((data, properties))
    }
}
