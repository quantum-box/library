//! Bulk sync ext_github property for all data items

use std::sync::Arc;
use std::time::Duration;

use database_manager::usecase::FindAllPropertiesInputData;
use outbound_sync::{
    SyncDataInputData, SyncDataInputPort, SyncPayload, SyncTarget,
};
use tachyon_sdk::auth::{AuthApp, CheckPolicyInput};
use tokio::time::sleep;

use crate::usecase::{
    BulkSyncExtGithubInputData, BulkSyncExtGithubInputPort,
    BulkSyncExtGithubOutputData,
};

/// Delay between sync operations to respect GitHub API rate limits.
/// GitHub allows 5000 requests/hour for authenticated users, so 100ms
/// between requests gives us ~36000 requests/hour with safety margin.
const SYNC_DELAY_MS: u64 = 100;

/// Sanitize a filename to prevent path traversal and injection attacks.
///
/// Only allows alphanumeric characters, hyphens, underscores, and periods.
/// Removes any path separators and control characters.
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .filter(|c| {
            c.is_alphanumeric() || *c == '-' || *c == '_' || *c == '.'
        })
        .collect::<String>()
        .trim_start_matches('.')
        .to_string()
}

/// Sanitize a path segment to prevent injection attacks.
///
/// Removes path traversal attempts (../, ./) and normalizes the path.
fn sanitize_path(path: &str) -> String {
    path.split('/')
        .filter(|segment| {
            !segment.is_empty() && *segment != "." && *segment != ".."
        })
        .map(sanitize_filename)
        .collect::<Vec<_>>()
        .join("/")
}

#[derive(Clone)]
pub struct BulkSyncExtGithub {
    get_org_by_username:
        Arc<dyn crate::usecase::GetOrganizationByUsernameQuery>,
    get_repo_by_username: Arc<dyn crate::usecase::GetRepoByUsernameQuery>,
    auth: Arc<dyn AuthApp>,
    database: Arc<database_manager::App>,
    sync_data: Arc<dyn SyncDataInputPort>,
}

impl std::fmt::Debug for BulkSyncExtGithub {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BulkSyncExtGithub").finish_non_exhaustive()
    }
}

impl BulkSyncExtGithub {
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
impl BulkSyncExtGithubInputPort for BulkSyncExtGithub {
    #[tracing::instrument(name = "BulkSyncExtGithub::execute", skip(self))]
    async fn execute<'a>(
        &self,
        input: BulkSyncExtGithubInputData<'a>,
    ) -> errors::Result<BulkSyncExtGithubOutputData> {
        // Get organization and repository
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

        // Check permission
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

        // Get all properties (for future use if needed)
        let _properties = self
            .database
            .find_all_properties()
            .execute(FindAllPropertiesInputData {
                tenant_id: org.id().clone(),
                database_id: database_id.clone(),
            })
            .await?;

        // Get first repo config as default
        let default_repo_config = match input.repo_configs.first() {
            Some(config) => config,
            None => {
                return Ok(BulkSyncExtGithubOutputData {
                    updated_count: 0,
                    skipped_count: 0,
                    total_count: 0,
                });
            }
        };

        // Process data items with pagination to avoid memory issues
        // Using a reasonable page size to balance memory usage and API calls
        const PAGE_SIZE: u32 = 100;
        let mut total_count = 0;
        let mut updated_count = 0;
        let mut skipped_count = 0;
        let mut current_page = 1;

        loop {
            let (data_list, paginator) = self
                .database
                .search_data()
                .execute(&database_manager::SearchDataInputData {
                    executor: input.executor,
                    multi_tenancy: input.multi_tenancy,
                    tenant_id: org.id(),
                    database_id: Some(database_id.clone()),
                    page_size: Some(PAGE_SIZE),
                    page: Some(current_page),
                    query: "",
                })
                .await?;

            if data_list.is_empty() {
                break;
            }

            total_count += data_list.len();

            // Process each data item with rate-limiting delay
            // Note: True parallel processing would require restructuring
            // SyncDataInputData to use owned types instead of references.
            for data in data_list {
                // Check if ext_github is already configured
                let existing_ext_github =
                    data.property_data().iter().find(|pd| {
                        pd.property_id().to_string().as_str()
                            == input.ext_github_property_id
                    });

                if let Some(existing) = existing_ext_github {
                    let value = existing.string_value();
                    // Skip if already has a valid configuration
                    if !value.is_empty() && value != "{}" {
                        if let Ok(parsed) = serde_json::from_str::<
                            serde_json::Value,
                        >(&value)
                        {
                            if parsed.get("repo").is_some() {
                                skipped_count += 1;
                                continue;
                            }
                        }
                    }
                }

                // Create ext_github value with sanitized filename
                let data_name = data.name().as_str();
                let sanitized_name = sanitize_filename(data_name);
                let path = default_repo_config
                    .default_path
                    .as_ref()
                    .map(|p| {
                        // Sanitize the entire path after template substitution
                        sanitize_path(
                            &p.replace("{{name}}", &sanitized_name),
                        )
                    })
                    .unwrap_or_else(|| format!("{sanitized_name}.md"));

                let ext_github_value = serde_json::json!({
                    "repo": default_repo_config.repo,
                    "path": path,
                    "enabled": true,
                })
                .to_string();

                // Build property data for database update
                let db_property_data: Vec<
                    database_manager::PropertyDataInputData,
                > = data
                    .property_data()
                    .iter()
                    .filter(|pd| {
                        pd.property_id().to_string().as_str()
                            != input.ext_github_property_id
                    })
                    .map(|pd| database_manager::PropertyDataInputData {
                        property_id: pd.property_id().to_string(),
                        value: pd.string_value(),
                    })
                    .chain(std::iter::once(
                        database_manager::PropertyDataInputData {
                            property_id: input
                                .ext_github_property_id
                                .clone(),
                            value: ext_github_value,
                        },
                    ))
                    .collect();

                // Update the data with ext_github property
                let updated_data = self
                    .database
                    .update_data_usecase()
                    .execute(database_manager::UpdateDataInputData {
                        executor: input.executor,
                        multi_tenancy: input.multi_tenancy,
                        tenant_id: org.id(),
                        database_id: &database_id,
                        data_id: data.id(),
                        name: data_name,
                        data: db_property_data,
                    })
                    .await
                    .map_err(|e| {
                        tracing::error!(
                            "Failed to update data {}: {}",
                            data.id(),
                            e
                        );
                        errors::Error::application_logic_error(
                            e.to_string(),
                        )
                    })?;

                // Generate markdown content for sync
                let markdown =
                    crate::usecase::markdown_composer::compose_markdown(
                        &updated_data,
                        &_properties,
                    );

                // Build sync target
                let target = SyncTarget::git_with_branch(
                    &default_repo_config.repo,
                    &path,
                    "main".to_string(),
                );

                // Build payload
                let message = format!("chore(library): sync {data_name}");
                let payload =
                    SyncPayload::markdown_with_message(&markdown, &message);

                // Execute sync to GitHub with rate limiting delay
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
                            "Synced data {} to GitHub: {:?}",
                            data.id(),
                            result.status
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to sync data {} to GitHub: {}",
                            data.id(),
                            e
                        );
                        // Continue with other data items even if sync fails
                    }
                }

                updated_count += 1;

                // Add delay between syncs to respect GitHub API rate limits
                sleep(Duration::from_millis(SYNC_DELAY_MS)).await;
            }

            // Move to next page
            current_page += 1;

            // If we got fewer items than page size, we've reached the end
            if (paginator.total_items as usize) <= total_count {
                break;
            }
        }

        Ok(BulkSyncExtGithubOutputData {
            updated_count,
            skipped_count,
            total_count,
        })
    }
}
