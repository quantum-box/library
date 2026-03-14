use async_graphql::{InputObject, OneofObject};
use tachyon_sdk::auth as auth_domain;

mod source;
pub use source::{CreateSourceInput, UpdateSourceInput};

#[derive(async_graphql::InputObject, Debug)]
pub struct UpdateOrganizationInput {
    pub username: String,
    pub name: String,
    pub description: Option<String>,
    pub website: Option<String>,
}

#[derive(async_graphql::InputObject, Debug)]
pub struct UpdateRepoInput {
    pub org_username: String,
    pub repo_username: String,
    pub name: Option<String>,
    /// Description of the repository
    /// not allow empty string, but can be null
    pub description: Option<String>,
    /// Whether the repository is public
    pub is_public: Option<bool>,
    /// Tags associated with the repository
    pub tags: Option<Vec<String>>,
}

#[derive(InputObject, Debug)]
pub struct CreateOrganizationInput {
    pub name: String,
    pub username: String,
    pub description: Option<String>,
    pub website: Option<String>,
}

#[derive(InputObject, Debug)]
pub struct CreateApiKeyInput {
    /// TODO: add English documentation
    pub organization_username: String,
    /// TODO: add English documentation
    pub name: String,
    /// TODO: add English documentation
    pub service_account_name: Option<String>,
}

#[derive(InputObject, Debug, Clone)]
pub struct CreateOperatorInput {
    /// TODO: add English documentation
    pub platform_id: String,
    /// TODO: add English documentation
    pub operator_alias: Option<String>,
    /// TODO: add English documentation
    pub operator_name: String,
    /// TODO: add English documentation
    pub new_operator_owner_method: auth_domain::NewOperatorOwnerMethod,
    /// TODO: add English documentation
    pub new_operator_owner_id: String,
    /// TODO: add English documentation
    pub new_operator_owner_password: Option<String>,
}

#[derive(OneofObject, Debug, Clone)]
pub enum IdOrEmail {
    Id(String),
    Email(String),
}

// ==================== GitHub Import Inputs ====================

/// Input for listing GitHub directory contents
#[derive(InputObject, Debug, Clone)]
pub struct ListGitHubDirectoryInput {
    /// GitHub repository in "owner/repo" format
    pub github_repo: String,
    /// Path to the directory (empty for root)
    pub path: String,
    /// Branch/tag/commit (optional, defaults to default branch)
    pub ref_name: Option<String>,
    /// Whether to include subdirectories recursively
    pub recursive: Option<bool>,
}

/// Input for getting Markdown import previews
#[derive(InputObject, Debug, Clone)]
pub struct GetMarkdownPreviewsInput {
    /// GitHub repository in "owner/repo" format
    pub github_repo: String,
    /// List of file paths to preview
    pub paths: Vec<String>,
    /// Branch/tag/commit (optional)
    pub ref_name: Option<String>,
}

/// Property mapping for import
#[derive(InputObject, Debug, Clone)]
pub struct PropertyMappingInput {
    /// Frontmatter key name
    pub frontmatter_key: String,
    /// Property name to create/use
    pub property_name: String,
    /// Property type
    pub property_type: super::model::PropertyType,
    /// Select options (if type is Select)
    pub select_options: Option<Vec<String>>,
}

/// Input for importing Markdown files from GitHub
#[derive(InputObject, Debug, Clone)]
pub struct ImportMarkdownFromGitHubInput {
    /// Organization username
    pub org_username: String,
    /// Repository username (will be created if it doesn't exist)
    pub repo_username: String,
    /// Repository name (for creating new repo)
    pub repo_name: Option<String>,
    /// GitHub repository in "owner/repo" format
    pub github_repo: String,
    /// List of file paths to import
    pub paths: Vec<String>,
    /// Branch/tag/commit (optional)
    pub ref_name: Option<String>,
    /// Property mappings from frontmatter
    pub property_mappings: Vec<PropertyMappingInput>,
    /// Property name for markdown content
    pub content_property_name: String,
    /// Whether to skip files that already exist (by ext_github path)
    pub skip_existing: Option<bool>,
    /// Whether to enable GitHub sync (default: true)
    /// If false, ext_github property will be created but without repo config
    pub enable_github_sync: Option<bool>,
}
