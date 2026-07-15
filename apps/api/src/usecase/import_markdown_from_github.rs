//! Import Markdown from GitHub usecase

use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;

use database_manager::domain as db;
use tachyon_sdk::auth::{
    AuthApp, CheckPolicyInput, GetOAuthTokenByProviderInput,
};
use value_object::{Identifier, Text, MAX_PAGE_SIZE};

use crate::usecase::{
    AddDataInputData, AddDataInputPort, AddPropertyInputData,
    AddPropertyInputPort, CreateRepoInputData, CreateRepoInputPort,
    GetPropertiesInputData, GetPropertiesInputPort, ImportError,
    ImportMarkdownFromGitHubInputData, ImportMarkdownFromGitHubInputPort,
    ImportMarkdownResult, UpdateDataInputData, UpdateDataInputPort,
    ViewDataListInputData, ViewDataListInputPort, ViewOrgInputData,
    ViewOrganizationInputPort,
};

use super::get_markdown_previews::{extract_title, parse_frontmatter};

#[derive(Clone)]
pub struct ImportMarkdownFromGitHub {
    auth: Arc<dyn AuthApp>,
    view_org: Arc<dyn ViewOrganizationInputPort>,
    create_repo: Arc<dyn CreateRepoInputPort>,
    get_properties: Arc<dyn GetPropertiesInputPort>,
    add_property: Arc<dyn AddPropertyInputPort>,
    view_data_list: Arc<dyn ViewDataListInputPort>,
    add_data: Arc<dyn AddDataInputPort>,
    update_data: Arc<dyn UpdateDataInputPort>,
}

impl std::fmt::Debug for ImportMarkdownFromGitHub {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImportMarkdownFromGitHub")
            .finish_non_exhaustive()
    }
}

impl ImportMarkdownFromGitHub {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        auth: Arc<dyn AuthApp>,
        view_org: Arc<dyn ViewOrganizationInputPort>,
        create_repo: Arc<dyn CreateRepoInputPort>,
        get_properties: Arc<dyn GetPropertiesInputPort>,
        add_property: Arc<dyn AddPropertyInputPort>,
        view_data_list: Arc<dyn ViewDataListInputPort>,
        add_data: Arc<dyn AddDataInputPort>,
        update_data: Arc<dyn UpdateDataInputPort>,
    ) -> Arc<Self> {
        Arc::new(Self {
            auth,
            view_org,
            create_repo,
            get_properties,
            add_property,
            view_data_list,
            add_data,
            update_data,
        })
    }
}

#[async_trait::async_trait]
impl ImportMarkdownFromGitHubInputPort for ImportMarkdownFromGitHub {
    #[tracing::instrument(
        name = "ImportMarkdownFromGitHub::execute",
        skip(self)
    )]
    async fn execute<'a>(
        &self,
        input: ImportMarkdownFromGitHubInputData<'a>,
    ) -> errors::Result<ImportMarkdownResult> {
        // Check permission
        self.auth
            .check_policy(&CheckPolicyInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                action: "library:CreateData",
            })
            .await?;

        // Get GitHub OAuth token
        let token = self
            .auth
            .get_oauth_token_by_provider(&GetOAuthTokenByProviderInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                provider: "github",
            })
            .await?
            .ok_or_else(|| {
                errors::Error::unauthorized(
                    "GitHub is not connected. Please connect your GitHub account first.",
                )
            })?;

        // Parse owner/repo format
        let parts: Vec<&str> = input.github_repo.split('/').collect();
        if parts.len() != 2 {
            return Err(errors::Error::bad_request(
                "Invalid github_repo format. Expected 'owner/repo'.",
            ));
        }
        let owner = parts[0];
        let github_repo_name = parts[1];

        // Get organization and find/create repo
        let org_output = self
            .view_org
            .execute(&ViewOrgInputData {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                organization_username: input.org_username.clone(),
            })
            .await?;

        // Find or create the repo
        let repo = org_output
            .repos
            .iter()
            .find(|r| r.username().to_string() == input.repo_username);

        let repo_id = if let Some(r) = repo {
            r.id().to_string()
        } else {
            // Create new repo
            let new_repo = self
                .create_repo
                .execute(CreateRepoInputData {
                    executor: input.executor,
                    multi_tenancy: input.multi_tenancy,
                    org_username: input.org_username.clone(),
                    repo_name: input
                        .repo_name
                        .clone()
                        .unwrap_or_else(|| input.repo_username.clone()),
                    repo_username: input.repo_username.clone(),
                    user_id: input.executor.get_id().to_string(),
                    is_public: false,
                    database_id: None,
                    description: Some(format!(
                        "Imported from GitHub: {}",
                        input.github_repo
                    )),
                    skip_sample_data: true,
                })
                .await?;
            new_repo.id().to_string()
        };

        // Get existing properties
        let existing_properties = self
            .get_properties
            .execute(GetPropertiesInputData {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                org_username: input.org_username.clone(),
                repo_username: input.repo_username.clone(),
            })
            .await?;

        // Create properties from mappings if they don't exist
        let mut property_map: HashMap<String, String> = HashMap::new();

        // First, ensure ext_github property exists
        let ext_github_prop = existing_properties
            .iter()
            .find(|p| p.name() == "ext_github");
        let ext_github_prop_id = if let Some(prop) = ext_github_prop {
            prop.id().to_string()
        } else {
            let prop = self
                .add_property
                .execute(AddPropertyInputData {
                    executor: input.executor,
                    multi_tenancy: input.multi_tenancy,
                    org_username: input.org_username.clone(),
                    repo_username: input.repo_username.clone(),
                    property_name: "ext_github".to_string(),
                    property_type: db::PropertyType::String,
                })
                .await?;
            prop.id().to_string()
        };

        // Ensure content property exists
        let content_prop = existing_properties
            .iter()
            .find(|p| p.name() == &input.content_property_name);
        let content_prop_id = if let Some(prop) = content_prop {
            validate_import_target(prop)?;
            prop.id().to_string()
        } else {
            let prop = self
                .add_property
                .execute(AddPropertyInputData {
                    executor: input.executor,
                    multi_tenancy: input.multi_tenancy,
                    org_username: input.org_username.clone(),
                    repo_username: input.repo_username.clone(),
                    property_name: input.content_property_name.clone(),
                    property_type: db::PropertyType::Markdown,
                })
                .await?;
            prop.id().to_string()
        };
        property_map.insert(
            input.content_property_name.clone(),
            content_prop_id.clone(),
        );

        // Create properties from mappings
        for mapping in &input.property_mappings {
            let existing = existing_properties
                .iter()
                .find(|p| p.name() == &mapping.property_name);

            let prop_id = if let Some(prop) = existing {
                validate_import_target(prop)?;
                prop.id().to_string()
            } else {
                let prop_type = match mapping.property_type.as_str() {
                    "STRING" => db::PropertyType::String,
                    "INTEGER" => db::PropertyType::Integer,
                    "MARKDOWN" => db::PropertyType::Markdown,
                    "SELECT" => {
                        let options = mapping
                            .select_options
                            .as_ref()
                            .map(|opts| {
                                opts.iter()
                                    .filter_map(|o| {
                                        let key: Identifier =
                                            o.parse().ok()?;
                                        let label: Text = o.parse().ok()?;
                                        Some(db::SelectItem::new(
                                            db::SelectItemId::default(),
                                            key,
                                            label,
                                        ))
                                    })
                                    .collect()
                            })
                            .unwrap_or_default();
                        db::PropertyType::Select(db::TypeSelect {
                            items: options,
                        })
                    }
                    "MULTI_SELECT" => {
                        let options = mapping
                            .select_options
                            .as_ref()
                            .map(|opts| {
                                opts.iter()
                                    .filter_map(|o| {
                                        let key: Identifier =
                                            o.parse().ok()?;
                                        let label: Text = o.parse().ok()?;
                                        Some(db::SelectItem::new(
                                            db::SelectItemId::default(),
                                            key,
                                            label,
                                        ))
                                    })
                                    .collect()
                            })
                            .unwrap_or_default();
                        db::PropertyType::MultiSelect(db::TypeMultiSelect {
                            items: options,
                        })
                    }
                    _ => db::PropertyType::String,
                };

                let prop = self
                    .add_property
                    .execute(AddPropertyInputData {
                        executor: input.executor,
                        multi_tenancy: input.multi_tenancy,
                        org_username: input.org_username.clone(),
                        repo_username: input.repo_username.clone(),
                        property_name: mapping.property_name.clone(),
                        property_type: prop_type,
                    })
                    .await?;
                prop.id().to_string()
            };

            property_map.insert(mapping.frontmatter_key.clone(), prop_id);
        }

        // Build a map of ext_github path -> data_id for duplicate detection
        let mut path_to_data_id: HashMap<String, String> = HashMap::new();
        let mut page = 1;
        loop {
            let (existing_data, _, paginator) = self
                .view_data_list
                .execute(&ViewDataListInputData {
                    executor: input.executor,
                    multi_tenancy: input.multi_tenancy,
                    org_username: input.org_username.clone(),
                    repo_username: input.repo_username.clone(),
                    page_size: Some(MAX_PAGE_SIZE),
                    page: Some(page),
                })
                .await?;

            for data in &existing_data {
                for prop_data in data.property_data() {
                    if *prop_data.property_id() == ext_github_prop_id {
                        if let Some(db::PropertyDataValue::String(path)) =
                            prop_data.value()
                        {
                            // Parse ext_github JSON value to get path
                            if let Ok(github_meta) =
                                serde_json::from_str::<serde_json::Value>(
                                    &path.to_string(),
                                )
                            {
                                if let Some(github_path) = github_meta
                                    .get("path")
                                    .and_then(|p| p.as_str())
                                {
                                    path_to_data_id.insert(
                                        github_path.to_string(),
                                        data.id().to_string(),
                                    );
                                }
                            }
                        }
                    }
                }
            }

            if page >= paginator.total_pages {
                break;
            }
            page += 1;
        }

        let mut imported_count = 0;
        let mut updated_count = 0;
        let mut skipped_count = 0;
        let mut errors = Vec::new();
        let mut data_ids = Vec::new();

        // Expand paths (directories are expanded to their markdown files)
        let expanded_paths = expand_paths(
            &token.access_token,
            owner,
            github_repo_name,
            &input.paths,
            input.ref_name.as_deref(),
        )
        .await;

        // Import each file
        for path in &expanded_paths {
            // Check for duplicate
            if let Some(existing_id) = path_to_data_id.get(path) {
                if input.skip_existing {
                    skipped_count += 1;
                    continue;
                }
                // Update existing data
                match import_single_file(
                    &token.access_token,
                    owner,
                    github_repo_name,
                    path,
                    input.ref_name.as_deref(),
                    &input.property_mappings,
                    &property_map,
                    &content_prop_id,
                    &ext_github_prop_id,
                    &input.github_repo,
                )
                .await
                {
                    Ok((name, property_data)) => {
                        let actor_id = input.executor.get_id();
                        match self
                            .update_data
                            .execute(UpdateDataInputData {
                                executor: input.executor,
                                multi_tenancy: input.multi_tenancy,
                                actor: actor_id,
                                org_username: &input.org_username,
                                repo_username: &input.repo_username,
                                data_id: existing_id,
                                data_name: &name,
                                property_data,
                            })
                            .await
                        {
                            Ok(_) => {
                                updated_count += 1;
                                data_ids.push(existing_id.clone());
                            }
                            Err(e) => {
                                errors.push(ImportError {
                                    path: path.clone(),
                                    message: e.to_string(),
                                });
                            }
                        }
                    }
                    Err(e) => {
                        errors.push(ImportError {
                            path: path.clone(),
                            message: e,
                        });
                    }
                }
            } else {
                // Create new data
                match import_single_file(
                    &token.access_token,
                    owner,
                    github_repo_name,
                    path,
                    input.ref_name.as_deref(),
                    &input.property_mappings,
                    &property_map,
                    &content_prop_id,
                    &ext_github_prop_id,
                    &input.github_repo,
                )
                .await
                {
                    Ok((name, property_data)) => {
                        let actor_id = input.executor.get_id();
                        match self
                            .add_data
                            .execute(AddDataInputData {
                                executor: input.executor,
                                multi_tenancy: input.multi_tenancy,
                                actor: actor_id,
                                org_username: &input.org_username,
                                repo_username: &input.repo_username,
                                data_name: &name,
                                property_data,
                            })
                            .await
                        {
                            Ok(data) => {
                                imported_count += 1;
                                data_ids.push(data.0.id().to_string());
                            }
                            Err(e) => {
                                errors.push(ImportError {
                                    path: path.clone(),
                                    message: e.to_string(),
                                });
                            }
                        }
                    }
                    Err(e) => {
                        errors.push(ImportError {
                            path: path.clone(),
                            message: e,
                        });
                    }
                }
            }
        }

        Ok(ImportMarkdownResult {
            imported_count,
            updated_count,
            skipped_count,
            errors,
            data_ids,
            repo_id,
        })
    }
}

/// Expand paths by resolving directories to their markdown files recursively.
///
/// If a path is a file ending in .md or .markdown, it's returned as-is.
/// If a path is a directory, it's recursively expanded to include all markdown files.
async fn expand_paths(
    access_token: &str,
    owner: &str,
    repo: &str,
    paths: &[String],
    ref_name: Option<&str>,
) -> Vec<String> {
    let mut expanded_paths = Vec::new();
    let mut visited = HashSet::new();

    for path in paths {
        expand_path_recursive(
            access_token,
            owner,
            repo,
            path,
            ref_name,
            &mut expanded_paths,
            &mut visited,
        )
        .await;
    }

    expanded_paths
}

/// Recursively expand a single path.
#[async_recursion::async_recursion]
async fn expand_path_recursive(
    access_token: &str,
    owner: &str,
    repo: &str,
    path: &str,
    ref_name: Option<&str>,
    expanded_paths: &mut Vec<String>,
    visited: &mut HashSet<String>,
) {
    // Avoid infinite loops
    if visited.contains(path) {
        return;
    }
    visited.insert(path.to_string());

    // Check if path is a markdown file
    let lowercase = path.to_lowercase();
    if lowercase.ends_with(".md") || lowercase.ends_with(".markdown") {
        expanded_paths.push(path.to_string());
        return;
    }

    // Try to list directory contents (this will succeed if path is a directory)
    match github_provider::GitHub::list_directory_contents(
        access_token,
        owner,
        repo,
        path,
        ref_name,
    )
    .await
    {
        Ok(listing) => {
            for item in listing.contents {
                if item.content_type == "file" {
                    let item_lower = item.path.to_lowercase();
                    if item_lower.ends_with(".md")
                        || item_lower.ends_with(".markdown")
                    {
                        expanded_paths.push(item.path);
                    }
                } else if item.content_type == "dir" {
                    // Recursively expand subdirectory
                    expand_path_recursive(
                        access_token,
                        owner,
                        repo,
                        &item.path,
                        ref_name,
                        expanded_paths,
                        visited,
                    )
                    .await;
                }
            }
        }
        Err(_) => {
            // If it's not a directory, assume it's a file path (even without .md extension)
            expanded_paths.push(path.to_string());
        }
    }
}

/// Import a single markdown file and return (name, property_data)
#[allow(clippy::too_many_arguments)]
async fn import_single_file(
    access_token: &str,
    owner: &str,
    repo: &str,
    path: &str,
    ref_name: Option<&str>,
    property_mappings: &[crate::usecase::PropertyMapping],
    property_map: &HashMap<String, String>,
    content_prop_id: &str,
    ext_github_prop_id: &str,
    github_repo: &str,
) -> Result<(String, Vec<crate::usecase::PropertyDataInputData>), String> {
    // Get file content
    let content = github_provider::GitHub::get_raw_file_content(
        access_token,
        owner,
        repo,
        path,
        ref_name,
    )
    .await
    .map_err(|e| format!("Failed to fetch file: {e}"))?;

    // Parse frontmatter
    let (frontmatter, body) = parse_frontmatter(&content);

    // Extract name
    let name = extract_title(&frontmatter, &body, path);

    let mut property_data = Vec::new();

    // Add content property
    property_data.push(crate::usecase::PropertyDataInputData {
        property_id: content_prop_id.to_string(),
        value: crate::usecase::PropertyDataValueInputData::Markdown(body),
    });

    // Add ext_github property with metadata
    // sync_to_github flag controls whether this property is included in frontmatter
    // when syncing back to GitHub
    let github_meta = github_import_metadata(github_repo, path, ref_name);
    property_data.push(crate::usecase::PropertyDataInputData {
        property_id: ext_github_prop_id.to_string(),
        value: crate::usecase::PropertyDataValueInputData::String(
            github_meta.to_string(),
        ),
    });

    // Add frontmatter properties
    if let Some(fm) = frontmatter {
        if let Some(obj) = fm.as_object() {
            for mapping in property_mappings {
                if let Some(value) = obj.get(&mapping.frontmatter_key) {
                    if let Some(prop_id) =
                        property_map.get(&mapping.frontmatter_key)
                    {
                        let value_str = match value {
                            serde_json::Value::String(s) => s.clone(),
                            _ => value.to_string(),
                        };
                        property_data
                            .push(crate::usecase::PropertyDataInputData {
                                property_id: prop_id.clone(),
                                value:
                                    crate::usecase::PropertyDataValueInputData::String(
                                        value_str,
                                    ),
                            });
                    }
                }
            }
        }
    }

    Ok((name, property_data))
}

fn validate_import_target(property: &db::Property) -> errors::Result<()> {
    if matches!(
        property.property_type(),
        db::PropertyType::Id(db::TypeId {
            auto_generate: true
        })
    ) {
        return Err(errors::Error::bad_request(format!(
            "Property '{}' is an auto-generated Id and cannot be mapped from GitHub content",
            property.name()
        )));
    }

    Ok(())
}

fn github_import_metadata(
    github_repo: &str,
    path: &str,
    ref_name: Option<&str>,
) -> serde_json::Value {
    serde_json::json!({
        "repo": github_repo,
        "path": path,
        "ref": ref_name.unwrap_or("main"),
        "enabled": false,
        "sync_to_github": false,
    })
}

#[cfg(test)]
mod tests {
    use super::{github_import_metadata, validate_import_target};
    use database_manager::domain as db;
    use value_object::TenantId;

    fn property(property_type: db::PropertyType) -> db::Property {
        db::Property::new(
            &db::PropertyId::default(),
            &TenantId::default(),
            &db::DatabaseId::default(),
            "id",
            &property_type,
            false,
            0,
        )
    }

    #[test]
    fn github_import_rejects_an_auto_generated_id_target() {
        let property =
            property(db::PropertyType::Id(db::TypeId::new(true)));

        assert!(validate_import_target(&property).is_err());
    }

    #[test]
    fn github_import_allows_a_manual_id_target() {
        let property =
            property(db::PropertyType::Id(db::TypeId::new(false)));

        assert!(validate_import_target(&property).is_ok());
    }

    #[test]
    fn one_shot_import_metadata_is_default_deny_for_writeback() {
        let metadata = github_import_metadata(
            "quantum-box/library",
            "docs/example.md",
            None,
        );

        assert_eq!(
            metadata,
            serde_json::json!({
                "repo": "quantum-box/library",
                "path": "docs/example.md",
                "ref": "main",
                "enabled": false,
                "sync_to_github": false,
            })
        );
    }
}
