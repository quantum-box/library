use crate::domain;

mod source;
use async_graphql::{Enum, SimpleObject, Union};
use chrono::{DateTime, Utc};
pub use source::Source;
use tachyon_sdk::auth as auth_domain;

#[derive(SimpleObject, Debug, Clone)]
pub struct Operator {
    pub id: String,
    pub name: String,
    pub operator_name: String,
    pub platform_tenant_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<auth_domain::Operator> for Operator {
    fn from(value: auth_domain::Operator) -> Self {
        Self {
            id: value.id().to_string(),
            name: value.name().to_string(),
            operator_name: value.operator_name().to_string(),
            platform_tenant_id: value.platform_id().to_string(),
            created_at: *value.created_at(),
            updated_at: *value.updated_at(),
        }
    }
}

#[derive(SimpleObject, Debug, Clone)]
pub struct ServiceAccount {
    pub id: String,
    pub tenant_id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

impl From<auth_domain::ServiceAccount> for ServiceAccount {
    fn from(value: auth_domain::ServiceAccount) -> Self {
        Self {
            id: value.id().to_string(),
            tenant_id: value.tenant_id().to_string(),
            name: value.name().to_string(),
            created_at: *value.created_at(),
        }
    }
}

#[derive(SimpleObject, Debug, Clone)]
pub struct PublicApiKey {
    pub id: String,
    pub tenant_id: String,
    pub service_account_id: String,
    pub name: String,
    pub value: String,
    pub created_at: DateTime<Utc>,
}

impl From<auth_domain::PublicApiKey> for PublicApiKey {
    fn from(value: auth_domain::PublicApiKey) -> Self {
        Self {
            id: value.id().to_string(),
            tenant_id: value.tenant_id().to_string(),
            service_account_id: value.service_account_id().to_string(),
            name: value.name().to_string(),
            value: value.value().to_string(),
            created_at: *value.created_at(),
        }
    }
}

#[derive(SimpleObject, Debug, Clone)]
#[graphql(complex)]
pub struct User {
    pub id: String,
    pub email: Option<String>,
    pub name: Option<String>,
    pub username: Option<String>,
    pub email_verified: Option<DateTime<Utc>>,
    pub image: Option<String>,
    pub role: auth_domain::DefaultRole,
    pub tenant_id_list: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<auth_domain::User> for User {
    fn from(user: auth_domain::User) -> Self {
        Self {
            id: user.id().to_string(),
            email: user.email().map(|e| e.to_string()),
            name: user.name().map(|n| n.to_string()),
            username: Some(user.username().to_string()),
            email_verified: *user.email_verified(),
            image: user.image().map(|i| i.to_string()),
            role: *user.role(),
            tenant_id_list: user
                .tenants()
                .iter()
                .map(|t| t.to_string())
                .collect(),
            created_at: *user.created_at(),
            updated_at: *user.updated_at(),
        }
    }
}

#[derive(SimpleObject, Debug, Clone)]
#[graphql(complex)]
pub struct Organization {
    pub id: String,
    pub name: String,
    pub username: String,
    pub description: Option<String>,
    pub website: Option<String>,
    pub repos: Vec<Repo>,
}

/// Source of a repository permission
///
/// Indicates whether a permission was explicitly assigned to the repository
/// or inherited from the organization-level role.
#[derive(async_graphql::Enum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionSource {
    /// Permission explicitly assigned to this repository
    Repo,
    /// Permission inherited from organization-level role (org owner)
    Org,
}

#[derive(SimpleObject, Debug, Clone)]
pub struct RepoPolicy {
    #[graphql(name = "userId")]
    pub user_id: String,
    pub role: String,
    pub user: Option<User>,
    /// Source of this permission (Repo or Org level)
    pub permission_source: PermissionSource,
}

/// Repository member with resource-based access control
///
/// Represents a user who has been granted access to a repository
/// via a scoped policy assignment.
#[derive(SimpleObject, Debug, Clone)]
pub struct RepoMember {
    /// User ID
    pub user_id: String,
    /// Policy ID that grants access
    pub policy_id: String,
    /// Policy name (e.g., "LibraryRepoOwnerPolicy", "LibraryRepoWriterPolicy")
    pub policy_name: Option<String>,
    /// TRN resource scope (e.g., "trn:library:repo:rp_xxx")
    pub resource_scope: Option<String>,
    /// When the policy was assigned
    pub assigned_at: chrono::DateTime<chrono::Utc>,
    /// User details (if available)
    pub user: Option<User>,
    /// Source of this permission (Repo or Org level)
    pub permission_source: PermissionSource,
}

#[derive(SimpleObject, Debug, Clone)]
#[graphql(complex)]
pub struct Repo {
    pub id: String,
    pub organization_id: String,
    pub org_username: String,
    pub name: String,
    pub username: String,
    pub description: Option<String>,
    /// Whether the repository is public
    pub is_public: bool,
    pub databases: Vec<String>,
    pub tags: Vec<String>,
    #[graphql(skip)]
    pub policies: Vec<RepoPolicy>,
}

#[derive(async_graphql::Enum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyType {
    String,
    Integer,
    #[graphql(deprecation = "Use MARKDOWN instead of HTML.")]
    Html,
    Markdown,
    Relation,
    Select,
    MultiSelect,
    Id,
    Location,
    Date,
    Image,
}

#[derive(SimpleObject, Debug, Clone)]
pub struct Property {
    pub id: String,
    pub tenant_id: String,
    pub database_id: String,
    pub name: String,
    /// TODO: add English documentation
    /// STRING, INTEGER, HTML, MARKDOWN, RELATION, SELECT, MULTI_SELECT
    pub typ: PropertyType,
    /// TODO: add English documentation
    pub meta: Option<PropertyTypeMeta>,
    pub is_indexed: bool,
    pub property_num: u32,
}

impl From<database_manager::domain::Property> for Property {
    fn from(value: database_manager::domain::Property) -> Self {
        // Check if property has meta_json, return it as JsonType
        let meta = if let Some(json) = value.meta_json() {
            Some(PropertyTypeMeta::Json(JsonType { json: json.clone() }))
        } else {
            match value.property_type() {
                database_manager::domain::PropertyType::String => None,
                database_manager::domain::PropertyType::Integer => None,
                database_manager::domain::PropertyType::Html => None,
                database_manager::domain::PropertyType::Markdown => None,
                database_manager::domain::PropertyType::Relation(r) => {
                    Some(PropertyTypeMeta::Relation(RelationType {
                        database_id: r.database_id.to_string(),
                    }))
                }
                database_manager::domain::PropertyType::Select(s) => {
                    Some(PropertyTypeMeta::Select(SelectType {
                        options: s
                            .items
                            .clone()
                            .into_iter()
                            .map(|i| i.into())
                            .collect(),
                    }))
                }
                database_manager::domain::PropertyType::MultiSelect(ms) => {
                    Some(PropertyTypeMeta::MultiSelect(MultiSelectType {
                        options: ms
                            .items
                            .clone()
                            .into_iter()
                            .map(|i| i.into())
                            .collect(),
                    }))
                }
                database_manager::domain::PropertyType::Id(id) => {
                    Some(PropertyTypeMeta::Id(IdType {
                        auto_generate: id.auto_generate,
                    }))
                }
                database_manager::domain::PropertyType::Location(_) => None,
                database_manager::domain::PropertyType::Date => None,
                database_manager::domain::PropertyType::Image => None,
            }
        };

        Self {
            id: value.id().to_string(),
            tenant_id: value.tenant_id().to_string(),
            database_id: value.database_id().to_string(),
            name: value.name().to_string(),
            typ: value.property_type().clone().into(),
            meta,
            is_indexed: *value.is_indexed(),
            property_num: *value.property_num(),
        }
    }
}

impl From<database_manager::domain::PropertyType> for PropertyType {
    fn from(value: database_manager::domain::PropertyType) -> Self {
        match value {
            database_manager::domain::PropertyType::String => Self::String,
            database_manager::domain::PropertyType::Integer => {
                Self::Integer
            }
            database_manager::domain::PropertyType::Html => Self::Html,
            database_manager::domain::PropertyType::Markdown => {
                Self::Markdown
            }
            database_manager::domain::PropertyType::Relation(_) => {
                Self::Relation
            }
            database_manager::domain::PropertyType::Select(_) => {
                Self::Select
            }
            database_manager::domain::PropertyType::MultiSelect(_) => {
                Self::MultiSelect
            }
            database_manager::domain::PropertyType::Id(_) => Self::Id,
            database_manager::domain::PropertyType::Location(_) => {
                Self::Location
            }
            database_manager::domain::PropertyType::Date => Self::Date,
            database_manager::domain::PropertyType::Image => Self::Image,
        }
    }
}

#[derive(Union, Debug, Clone)]
pub enum PropertyTypeMeta {
    Relation(RelationType),
    Select(SelectType),
    MultiSelect(MultiSelectType),
    Id(IdType),
    Json(JsonType),
}

#[derive(SimpleObject, Debug, Clone)]
pub struct JsonType {
    /// TODO: add English documentation
    pub json: String,
}

#[derive(SimpleObject, Debug, Clone)]
pub struct RelationType {
    pub database_id: String,
}

#[derive(SimpleObject, Debug, Clone)]
pub struct SelectType {
    pub options: Vec<SelectItem>,
}

#[derive(SimpleObject, Debug, Clone)]
pub struct MultiSelectType {
    pub options: Vec<SelectItem>,
}

#[derive(SimpleObject, Debug, Clone)]
pub struct SelectItem {
    pub id: String,
    pub key: String,
    pub name: String,
}

impl From<database_manager::domain::SelectItem> for SelectItem {
    fn from(value: database_manager::domain::SelectItem) -> Self {
        Self {
            id: value.id().to_string(),
            key: value.key().to_string(),
            name: value.name().to_string(),
        }
    }
}

#[derive(SimpleObject, Debug, Clone)]
pub struct IdType {
    pub auto_generate: bool,
}

#[derive(SimpleObject, Debug, Clone)]
pub struct Data {
    pub id: String,
    pub tenant_id: String,
    pub database_id: String,
    pub name: String,
    pub property_data: Vec<PropertyData>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(SimpleObject, Debug, Clone)]
pub struct PropertyData {
    pub property_id: String,
    pub value: PropertyDataValue,
}

#[derive(Union, Debug, Clone)]
pub enum PropertyDataValue {
    String(StringValue),
    Integer(IntegerValue),
    Html(HtmlValue),
    Markdown(MarkdownValue),
    Relation(RelationValue),
    Select(SelectValue),
    MultiSelect(MultiSelectValue),
    Id(IdValue),
    Location(LocationValue),
    Date(DateValue),
    Image(ImageValue),
}

#[derive(SimpleObject, Debug, Clone)]
pub struct StringValue {
    pub string: String,
}

#[derive(SimpleObject, Debug, Clone)]
pub struct IntegerValue {
    pub number: String,
}

#[derive(SimpleObject, Debug, Clone)]
pub struct HtmlValue {
    pub html: String,
}

#[derive(SimpleObject, Debug, Clone)]
pub struct MarkdownValue {
    pub markdown: String,
}

#[derive(SimpleObject, Debug, Clone)]
pub struct SelectValue {
    pub option_id: String,
}

#[derive(SimpleObject, Debug, Clone)]
pub struct MultiSelectValue {
    pub option_ids: Vec<String>,
}

#[derive(SimpleObject, Debug, Clone)]
pub struct RelationValue {
    pub database_id: String,
    pub data_ids: Vec<String>,
}

#[derive(SimpleObject, Debug, Clone)]
pub struct IdValue {
    pub id: String,
}

#[derive(SimpleObject, Debug, Clone)]
pub struct LocationValue {
    /// TODO: add English documentation
    /// -90.0 ~ 90.0
    pub latitude: f64,
    /// TODO: add English documentation
    /// -180.0 ~ 180.0
    pub longitude: f64,
}

#[derive(SimpleObject, Debug, Clone)]
pub struct DateValue {
    /// Date in ISO 8601 format (YYYY-MM-DD)
    pub date: String,
}

#[derive(SimpleObject, Debug, Clone)]
pub struct ImageValue {
    /// Image URL
    pub url: String,
}

impl From<database_manager::domain::Data> for Data {
    fn from(value: database_manager::domain::Data) -> Self {
        Data {
            id: value.id().to_string(),
            tenant_id: value.tenant_id().to_string(),
            database_id: value.database_id().to_string(),
            name: value.name().to_string(),
            property_data: value
                .property_data()
                .iter()
                .map(|p| p.clone().into())
                .collect(),
            created_at: *value.created_at(),
            updated_at: *value.updated_at(),
        }
    }
}

impl From<database_manager::domain::PropertyData> for PropertyData {
    fn from(value: database_manager::domain::PropertyData) -> Self {
        PropertyData {
            property_id: value.property_id().to_string(),
            value: if let Some(v) = value.value() {
                v.clone().into()
            } else {
                PropertyDataValue::String(StringValue {
                    string: String::new(),
                })
            },
        }
    }
}

impl From<database_manager::domain::PropertyDataValue>
    for PropertyDataValue
{
    fn from(value: database_manager::domain::PropertyDataValue) -> Self {
        match value {
            database_manager::domain::PropertyDataValue::String(s) => {
                PropertyDataValue::String(StringValue {
                    string: s.to_string(),
                })
            }
            database_manager::domain::PropertyDataValue::Integer(i) => {
                PropertyDataValue::Integer(IntegerValue {
                    number: i.to_string(),
                })
            }
            database_manager::domain::PropertyDataValue::Html(h) => {
                PropertyDataValue::Html(HtmlValue {
                    html: h.to_string(),
                })
            }
            database_manager::domain::PropertyDataValue::Markdown(m) => {
                PropertyDataValue::Markdown(MarkdownValue {
                    markdown: m.to_string(),
                })
            }
            database_manager::domain::PropertyDataValue::Relation(
                db_id,
                data_ids,
            ) => PropertyDataValue::Relation(RelationValue {
                database_id: db_id.to_string(),
                data_ids: data_ids
                    .into_iter()
                    .map(|d| d.to_string())
                    .collect(),
            }),
            database_manager::domain::PropertyDataValue::Id(id) => {
                PropertyDataValue::Id(IdValue { id: id.to_string() })
            }
            database_manager::domain::PropertyDataValue::Location(
                location,
            ) => PropertyDataValue::Location(LocationValue {
                latitude: location.latitude(),
                longitude: location.longitude(),
            }),
            database_manager::domain::PropertyDataValue::Select(id) => {
                PropertyDataValue::Select(SelectValue {
                    option_id: id.to_string(),
                })
            }
            database_manager::domain::PropertyDataValue::MultiSelect(
                ids,
            ) => PropertyDataValue::MultiSelect(MultiSelectValue {
                option_ids: ids
                    .into_iter()
                    .map(|d| d.to_string())
                    .collect(),
            }),
            database_manager::domain::PropertyDataValue::Date(date) => {
                PropertyDataValue::Date(DateValue {
                    date: date.to_string(),
                })
            }
            database_manager::domain::PropertyDataValue::Image(url) => {
                PropertyDataValue::Image(ImageValue {
                    url: url.to_string(),
                })
            }
        }
    }
}

impl From<domain::Organization> for Organization {
    fn from(org: domain::Organization) -> Self {
        Self {
            id: org.id().to_string(),
            name: org.name().to_string(),
            username: org.username().to_string(),
            description: org.description().clone().map(|d| d.to_string()),
            website: org.website().clone().map(|w| w.to_string()),
            repos: vec![],
        }
    }
}

impl From<domain::Repo> for Repo {
    fn from(value: domain::Repo) -> Self {
        Self {
            id: value.id().to_string(),
            organization_id: value.organization_id().to_string(),
            org_username: value.org_username().to_string(),
            name: value.name().to_string(),
            username: value.username().to_string(),
            description: value.description().clone().map(|d| d.to_string()),
            is_public: *value.is_public(),
            databases: value
                .databases()
                .clone()
                .into_iter()
                .map(|d| d.to_string())
                .collect(),
            tags: value.tags().iter().map(|t| t.to_string()).collect(),
            policies: vec![],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, SimpleObject)]
pub struct Paginator {
    pub current_page: u32,
    pub items_per_page: u32,
    pub total_items: u32,
    pub total_pages: u32,
}

impl From<value_object::OffsetPaginator> for Paginator {
    fn from(value: value_object::OffsetPaginator) -> Self {
        Self {
            current_page: value.current_page,
            items_per_page: value.items_per_page,
            total_items: value.total_items,
            total_pages: value.total_pages,
        }
    }
}

#[derive(Debug, Clone, SimpleObject)]
pub struct DataList {
    pub items: Vec<Data>,
    pub paginator: Paginator,
}

#[derive(SimpleObject, Debug, Clone)]
pub struct ApiKeyResponse {
    pub api_key: PublicApiKey,
    pub service_account: ServiceAccount,
}

// ==================== GitHub Sync Types ====================

/// GitHub OAuth connection status
#[derive(SimpleObject, Debug, Clone)]
pub struct GitHubConnection {
    /// Whether GitHub is connected
    pub connected: bool,
    /// GitHub username (if connected)
    pub username: Option<String>,
    /// When the token was last refreshed
    pub connected_at: Option<DateTime<Utc>>,
    /// When the token expires (if applicable)
    pub expires_at: Option<DateTime<Utc>>,
}

/// GitHub OAuth authorization URL response
#[derive(SimpleObject, Debug, Clone)]
pub struct GitHubAuthUrl {
    /// The URL to redirect the user to for authorization
    pub url: String,
    /// State parameter for CSRF protection
    pub state: String,
}

/// GitHub repository information
#[derive(SimpleObject, Debug, Clone)]
pub struct GitHubRepository {
    /// Repository ID
    pub id: String,
    /// Repository name
    pub name: String,
    /// Full name (owner/repo format)
    pub full_name: String,
    /// Repository description
    pub description: Option<String>,
    /// Whether the repository is private
    pub private: bool,
    /// HTML URL to the repository
    pub html_url: String,
    /// Default branch name
    pub default_branch: Option<String>,
}

impl From<github_provider::GitHubRepository> for GitHubRepository {
    fn from(repo: github_provider::GitHubRepository) -> Self {
        Self {
            id: repo.id.to_string(),
            name: repo.name,
            full_name: repo.full_name,
            description: repo.description,
            private: repo.private,
            html_url: repo.html_url,
            default_branch: repo.default_branch,
        }
    }
}

/// Sync status enum
#[derive(async_graphql::Enum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncStatus {
    /// Never synchronized
    NeverSynced,
    /// Successfully synchronized
    Synced,
    /// Synchronization pending
    Pending,
    /// Synchronization failed
    Failed,
}

impl From<outbound_sync::SyncStatus> for SyncStatus {
    fn from(value: outbound_sync::SyncStatus) -> Self {
        match value {
            outbound_sync::SyncStatus::NeverSynced => Self::NeverSynced,
            outbound_sync::SyncStatus::Synced => Self::Synced,
            outbound_sync::SyncStatus::Pending => Self::Pending,
            outbound_sync::SyncStatus::Failed(_) => Self::Failed,
        }
    }
}

/// Sync configuration for a data item
#[derive(SimpleObject, Debug, Clone)]
pub struct SyncConfig {
    /// Sync configuration ID
    pub id: String,
    /// Data ID being synced
    pub data_id: String,
    /// Provider name (github, gitlab, etc.)
    pub provider: String,
    /// Target container (repo for GitHub)
    pub target_container: String,
    /// Target resource path
    pub target_resource: Option<String>,
    /// Target version/branch
    pub target_version: Option<String>,
    /// Current sync status
    pub status: SyncStatus,
    /// Error message if failed
    pub error_message: Option<String>,
    /// Last successful sync time
    pub last_synced_at: Option<DateTime<Utc>>,
    /// Last result ID (commit SHA, etc.)
    pub last_result_id: Option<String>,
    /// When the config was created
    pub created_at: DateTime<Utc>,
    /// When the config was last updated
    pub updated_at: DateTime<Utc>,
}

impl From<outbound_sync::SyncConfig> for SyncConfig {
    fn from(value: outbound_sync::SyncConfig) -> Self {
        Self {
            id: value.id().to_string(),
            data_id: value.data_id().to_string(),
            provider: value.provider().to_string(),
            target_container: value.target().container.clone(),
            target_resource: value.target().resource.clone(),
            target_version: value.target().version.clone(),
            status: value.status().clone().into(),
            error_message: value
                .status()
                .error_message()
                .map(|s| s.to_string()),
            last_synced_at: *value.last_synced_at(),
            last_result_id: value.last_result_id().clone(),
            created_at: *value.created_at(),
            updated_at: *value.updated_at(),
        }
    }
}

/// Result of a sync operation
#[derive(SimpleObject, Debug, Clone)]
pub struct SyncResult {
    /// Whether the sync succeeded
    pub success: bool,
    /// Sync status after operation
    pub status: SyncStatus,
    /// Result ID (commit SHA, etc.)
    pub result_id: Option<String>,
    /// URL to the synced resource
    pub url: Option<String>,
    /// Diff preview (for dry-run)
    pub diff: Option<String>,
}

// ==================== GitHub Import Types ====================

/// GitHub file/directory information
#[derive(SimpleObject, Debug, Clone)]
pub struct GitHubFileInfo {
    /// File/directory name
    pub name: String,
    /// Full path in the repository
    pub path: String,
    /// SHA hash of the content
    pub sha: String,
    /// Size in bytes (0 for directories)
    pub size: i64,
    /// Type: "file" or "dir"
    pub file_type: String,
    /// HTML URL to view on GitHub
    pub html_url: Option<String>,
}

impl From<github_provider::GitHubContent> for GitHubFileInfo {
    fn from(content: github_provider::GitHubContent) -> Self {
        Self {
            name: content.name,
            path: content.path,
            sha: content.sha,
            size: content.size,
            file_type: content.content_type,
            html_url: content.html_url,
        }
    }
}

/// Result of listing directory contents
#[derive(SimpleObject, Debug, Clone)]
pub struct GitHubDirectoryContents {
    /// List of files and directories
    pub files: Vec<GitHubFileInfo>,
    /// Whether the listing was truncated (more than 1000 items)
    pub truncated: bool,
}

/// Preview of a Markdown file for import
#[derive(SimpleObject, Debug, Clone)]
pub struct MarkdownImportPreview {
    /// File path in the repository
    pub path: String,
    /// SHA hash for change detection
    pub sha: String,
    /// Parsed frontmatter as JSON string
    pub frontmatter_json: Option<String>,
    /// List of frontmatter keys found
    pub frontmatter_keys: Vec<String>,
    /// Suggested data name (from title, h1, or filename)
    pub suggested_name: String,
    /// Preview of the markdown body (first 500 chars)
    pub body_preview: String,
    /// Error message if parsing failed
    pub parse_error: Option<String>,
}

/// Suggested property type and options for a frontmatter key
#[derive(SimpleObject, Debug, Clone)]
pub struct SuggestedProperty {
    /// Frontmatter key name
    pub key: String,
    /// Suggested property type
    pub suggested_type: PropertyType,
    /// Unique values found (for Select type suggestion)
    pub unique_values: Vec<String>,
    /// Whether this should be a Select type (<=5 unique values)
    pub suggest_select: bool,
}

/// Result of analyzing frontmatter across multiple files
#[derive(SimpleObject, Debug, Clone)]
pub struct FrontmatterAnalysis {
    /// List of suggested properties
    pub properties: Vec<SuggestedProperty>,
    /// Total files analyzed
    pub total_files: i32,
    /// Files with valid frontmatter
    pub valid_files: i32,
}

/// Error during import
#[derive(SimpleObject, Debug, Clone)]
pub struct ImportError {
    /// File path that caused the error
    pub path: String,
    /// Error message
    pub message: String,
}

/// Result of importing Markdown files
#[derive(SimpleObject, Debug, Clone)]
pub struct ImportMarkdownResult {
    /// Number of files successfully imported
    pub imported_count: i32,
    /// Number of files updated (already existed)
    pub updated_count: i32,
    /// Number of files skipped
    pub skipped_count: i32,
    /// List of errors encountered
    pub errors: Vec<ImportError>,
    /// IDs of created/updated data items
    pub data_ids: Vec<String>,
    /// ID of the repository (created or existing)
    pub repo_id: String,
}

/// Integration in the marketplace
#[derive(SimpleObject, Debug, Clone)]
pub struct Integration {
    /// Integration ID
    pub id: String,
    /// Provider (github, linear, hubspot, etc.)
    pub provider: String,
    /// Display name
    pub name: String,
    /// Description
    pub description: String,
    /// Icon name
    pub icon: Option<String>,
    /// Category
    pub category: String,
    /// Sync capability (inbound, outbound, bidirectional)
    pub sync_capability: String,
    /// Supported object types
    pub supported_objects: Vec<String>,
    /// Requires OAuth authentication
    pub requires_oauth: bool,
    /// Is enabled
    pub is_enabled: bool,
    /// Is featured
    pub is_featured: bool,
}

/// A tenant's connection to an integration
#[derive(SimpleObject, Debug, Clone)]
pub struct Connection {
    /// Connection ID
    pub id: String,
    /// Integration ID
    pub integration_id: String,
    /// Provider
    pub provider: String,
    /// Connection status
    pub status: ConnectionStatus,
    /// External account ID
    pub external_account_id: Option<String>,
    /// External account name
    pub external_account_name: Option<String>,
    /// Connected at
    pub connected_at: chrono::DateTime<chrono::Utc>,
    /// Last synced at
    pub last_synced_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Error message
    pub error_message: Option<String>,
}

/// Connection status
#[derive(Enum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStatus {
    /// Active connection
    Active,
    /// Expired (token expired)
    Expired,
    /// Paused (manually disabled)
    Paused,
    /// Error (authentication or sync error)
    Error,
    /// Disconnected (soft delete)
    Disconnected,
}

/// Linear Team for GraphQL
#[derive(SimpleObject, Debug, Clone)]
pub struct LinearTeam {
    /// Team ID
    pub id: String,
    /// Team name
    pub name: String,
    /// Team key (e.g., "ENG")
    pub key: String,
}

/// Linear Project for GraphQL
#[derive(SimpleObject, Debug, Clone)]
pub struct LinearProject {
    /// Project ID
    pub id: String,
    /// Project name
    pub name: String,
}

/// Linear Issue for GraphQL
#[derive(SimpleObject, Debug, Clone)]
pub struct LinearIssue {
    /// Issue ID
    pub id: String,
    /// Issue identifier (e.g., "ENG-123")
    pub identifier: String,
    /// Issue title
    pub title: String,
    /// Issue state name
    pub state_name: Option<String>,
    /// Assignee name
    pub assignee_name: Option<String>,
    /// Issue URL
    pub url: Option<String>,
}
