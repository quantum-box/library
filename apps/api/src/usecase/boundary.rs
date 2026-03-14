//! # Usecase boundary
//!
//! Defines input/output DTOs and port traits for Library API use cases.
//! Authorization levels are described per use case (public, signed-in, repository member, owner).

use crate::domain::{Organization, Repo};
use async_graphql::{InputObject, OneofObject};
use database_manager::domain::{self, Data, Property};
use std::fmt::Debug;
use tachyon_sdk::auth::{ExecutorAction, MultiTenancyAction};
use value_object::{Location, OffsetPaginator};

/// TODO: add English documentation
#[async_trait::async_trait]
pub trait ViewOrganizationInputPort: Debug + Send + Sync {
    /// TODO: add English documentation
    async fn execute<'a>(
        &self,
        input: &ViewOrgInputData<'a>,
    ) -> errors::Result<ViewOrgOutputData>;
}

/// TODO: add English documentation
#[async_trait::async_trait]
pub trait CreateOrganizationInputPort: Debug + Send + Sync {
    /// TODO: add English documentation
    async fn execute<'a>(
        &self,
        input: &CreateOrganizationInputData<'a>,
    ) -> errors::Result<Organization>;
}

/// TODO: add English documentation
///
/// TODO: add English documentation
#[async_trait::async_trait]
pub trait ViewRepoInputPort: Debug + Send + Sync {
    /// TODO: add English documentation
    async fn execute<'a>(
        &self,
        input: &ViewRepoInputData<'a>,
    ) -> errors::Result<ViewRepoOutputData>;
}

/// TODO: add English documentation
#[async_trait::async_trait]
pub trait SearchRepoInputPort: Debug + Send + Sync {
    /// TODO: add English documentation
    async fn execute(
        &self,
        input: &SearchRepoInputData,
    ) -> errors::Result<Vec<Repo>>;
}

/// TODO: add English documentation
#[async_trait::async_trait]
pub trait ViewDataInputPort: Debug + Send + Sync {
    /// TODO: add English documentation
    async fn execute<'a>(
        &self,
        input: &ViewDataInputData<'a>,
    ) -> errors::Result<(domain::Data, Vec<domain::Property>)>;
}

/// TODO: add English documentation
///
/// TODO: add English documentation
#[async_trait::async_trait]
pub trait SearchDataInputPort: Debug + Send + Sync {
    /// TODO: add English documentation
    async fn execute(
        &self,
        input: &SearchDataInputData,
    ) -> errors::Result<(
        Vec<domain::Data>,
        Vec<domain::Property>,
        OffsetPaginator,
    )>;
}

/// TODO: add English documentation
#[async_trait::async_trait]
pub trait CreatePullRequestInputPort: Debug + Send + Sync {
    /// TODO: add English documentation
    #[allow(dead_code)]
    async fn execute(
        &self,
        input: CreatePullRequestInputData,
    ) -> errors::Result<()>;
}

/// TODO: add English documentation
#[async_trait::async_trait]
pub trait CreateRepoInputPort: Debug + Send + Sync {
    /// TODO: add English documentation
    async fn execute<'a>(
        &self,
        input: CreateRepoInputData<'a>,
    ) -> errors::Result<Repo>;
}

/// TODO: add English documentation
#[async_trait::async_trait]
pub trait UpdateRepoInputPort: Debug + Send + Sync {
    /// TODO: add English documentation
    #[allow(dead_code)]
    async fn execute<'a>(
        &self,
        input: UpdateRepoInputData<'a>,
    ) -> errors::Result<Repo>;
}

/// TODO: add English documentation
#[async_trait::async_trait]
pub trait AddDataInputPort: Debug + Send + Sync {
    /// TODO: add English documentation
    async fn execute<'a>(
        &self,
        input: AddDataInputData<'a>,
    ) -> errors::Result<(Data, Vec<Property>)>;
}

/// TODO: add English documentation
#[async_trait::async_trait]
pub trait UpdateDataInputPort: Debug + Send + Sync {
    async fn execute<'a>(
        &self,
        input: UpdateDataInputData<'a>,
    ) -> errors::Result<(Data, Vec<Property>)>;
}

/// TODO: add English documentation
#[async_trait::async_trait]
pub trait DeleteDataInputPort: Debug + Send + Sync {
    /// TODO: add English documentation
    async fn execute<'a>(
        &self,
        input: DeleteDataInputData<'a>,
    ) -> errors::Result<()>;
}

/// TODO: add English documentation
#[async_trait::async_trait]
pub trait ChangeRepoPolicyInputPort: Debug + Send + Sync {
    /// TODO: add English documentation
    async fn execute<'a>(
        &self,
        input: ChangeRepoPolicyInputData<'a>,
    ) -> errors::Result<()>;
}

/// TODO: add English documentation
#[async_trait::async_trait]
pub trait DeleteRepoInputPort: Debug + Send + Sync {
    /// TODO: add English documentation
    async fn execute<'a>(
        &self,
        input: DeleteRepoInputData<'a>,
    ) -> errors::Result<()>;
}

/// TODO: add English documentation
#[async_trait::async_trait]
pub trait AddPropertyInputPort: Debug + Send + Sync {
    /// TODO: add English documentation
    async fn execute<'a>(
        &self,
        input: AddPropertyInputData<'a>,
    ) -> errors::Result<domain::Property>;
}

/// TODO: add English documentation
#[async_trait::async_trait]
pub trait UpdatePropertyInputPort: Debug + Send + Sync {
    async fn execute<'a>(
        &self,
        input: UpdatePropertyInputData<'a>,
    ) -> errors::Result<domain::Property>;
}

/// TODO: add English documentation
#[async_trait::async_trait]
pub trait DeletePropertyInputPort: Debug + Send + Sync {
    /// TODO: add English documentation
    async fn execute<'a>(
        &self,
        input: DeletePropertyInputData<'a>,
    ) -> errors::Result<domain::Property>;
}

/// TODO: add English documentation
#[async_trait::async_trait]
pub trait GetPropertiesInputPort: Debug + Send + Sync {
    /// TODO: add English documentation
    async fn execute<'a>(
        &self,
        input: GetPropertiesInputData<'a>,
    ) -> errors::Result<Vec<domain::Property>>;
}

/// TODO: add English documentation
#[async_trait::async_trait]
pub trait CreateSourceInputPort: Debug + Send + Sync {
    /// TODO: add English documentation
    async fn execute<'a>(
        &self,
        input: CreateSourceInputData<'a>,
    ) -> errors::Result<crate::domain::Source>;
}

/// TODO: add English documentation
#[async_trait::async_trait]
pub trait UpdateSourceInputPort: Debug + Send + Sync {
    /// TODO: add English documentation
    async fn execute<'a>(
        &self,
        input: UpdateSourceInputData<'a>,
    ) -> errors::Result<crate::domain::Source>;
}

/// TODO: add English documentation
#[async_trait::async_trait]
pub trait DeleteSourceInputPort: Debug + Send + Sync {
    /// TODO: add English documentation
    async fn execute<'a>(
        &self,
        input: DeleteSourceInputData<'a>,
    ) -> errors::Result<()>;
}

/// TODO: add English documentation
#[async_trait::async_trait]
pub trait GetSourceInputPort: Debug + Send + Sync {
    /// TODO: add English documentation
    async fn execute<'a>(
        &self,
        input: GetSourceInputData<'a>,
    ) -> errors::Result<Option<crate::domain::Source>>;
}

/// TODO: add English documentation
#[async_trait::async_trait]
pub trait FindSourcesInputPort: Debug + Send + Sync {
    /// TODO: add English documentation
    async fn execute<'a>(
        &self,
        input: FindSourcesInputData<'a>,
    ) -> errors::Result<Vec<crate::domain::Source>>;
}

// InputData

/// TODO: add English documentation
#[derive(Debug, Clone)]
pub struct ViewOrgInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,

    pub organization_username: String,
}

#[derive(Debug, Clone)]
pub struct ViewOrgOutputData {
    pub organization: Organization,
    pub repos: Vec<Repo>,
}

/// TODO: add English documentation
#[derive(Debug, Clone)]
pub struct CreateOrganizationInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,

    pub name: String,
    pub username: String,
    pub description: Option<String>,
    pub website: Option<String>,
}

/// TODO: add English documentation
#[derive(Debug, Clone)]
pub struct ViewRepoInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,

    pub organization_username: String,
    pub repo_username: String,
}

#[derive(Debug, Clone)]
pub struct ViewRepoOutputData {
    pub repo: Repo,
    // pub properties: Vec<domain::Property>,
    // pub data: Vec<domain::Data>,
}

/// TODO: add English documentation
///
/// TODO: add English documentation
/// TODO: add English documentation
#[derive(Debug, Clone, Default)]
pub struct SearchRepoInputData {
    pub org_username: Option<String>,
    pub name: Option<String>,
    pub limit: Option<i64>,
}

/// TODO: add English documentation
#[derive(Debug, Clone)]
pub struct ViewDataInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,

    pub org_username: String,
    pub repo_username: String,
    pub data_id: String,
}

/// TODO: add English documentation
///
/// TODO: add English documentation
/// TODO: add English documentation
#[derive(Debug, Clone)]
pub struct SearchDataInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    /// TODO: add English documentation
    pub org_username: &'a str,
    pub repo_username: &'a str,
    pub name: &'a str,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

/// TODO: add English documentation
#[derive(Debug, Clone)]
pub struct CreatePullRequestInputData {
    #[allow(dead_code)]
    pub organization_id: String,
    #[allow(dead_code)]
    pub repo_id: String,
}

/// TODO: add English documentation
#[derive(Debug, Clone)]
pub struct CreateRepoInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,

    pub org_username: String,
    pub repo_name: String,
    pub repo_username: String,
    pub user_id: String,
    pub is_public: bool,
    pub description: Option<String>,
    /// TODO: add English documentation
    /// TODO: add English documentation
    pub database_id: Option<String>,
    /// TODO: add English documentation
    /// TODO: add English documentation
    pub skip_sample_data: bool,
}

/// TODO: add English documentation
#[derive(Debug, Clone)]
pub struct UpdateRepoInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,

    pub org_username: String,
    pub repo_username: String,

    pub name: Option<value_object::Text>,
    pub description: Option<value_object::LongText>,
    pub is_public: Option<bool>,
    pub tags: Option<Vec<value_object::Text>>,
}

#[derive(Debug, Clone)]
pub struct UpdateDataInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,

    /// user_id
    pub actor: &'a str,
    pub org_username: &'a str,
    pub repo_username: &'a str,
    pub data_id: &'a str,
    pub data_name: &'a str,
    pub property_data: Vec<PropertyDataInputData>,
}

#[derive(Debug, Clone, InputObject)]
pub struct PropertyDataInputData {
    pub property_id: String,
    pub value: PropertyDataValueInputData,
}

#[derive(Debug, Clone, OneofObject)]
pub enum PropertyDataValueInputData {
    String(String),
    Integer(String),
    Html(String),
    Markdown(String),
    Relation(Vec<String>),
    Select(String),
    MultiSelect(Vec<String>),
    Location(Location),
    Date(String),
    Image(String),
}

/// TODO: add English documentation
#[derive(Debug, Clone)]
pub struct AddDataInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,

    pub actor: &'a str,
    pub org_username: &'a str,
    pub repo_username: &'a str,
    pub data_name: &'a str,
    pub property_data: Vec<PropertyDataInputData>,
}

/// TODO: add English documentation
#[derive(Debug, Clone)]
pub struct DeleteDataInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,

    pub actor: String,
    pub org_username: String,
    pub repo_username: String,
    pub data_id: String,
}

/// TODO: add English documentation
#[derive(Debug, Clone)]
pub struct ChangeRepoPolicyInputData<'a> {
    pub executor: &'a dyn tachyon_sdk::auth::ExecutorAction,
    pub multi_tenancy: &'a dyn tachyon_sdk::auth::MultiTenancyAction,
    pub org_username: String,
    pub repo_username: String,
    pub repo_id: String,
    pub target_user_id: String,
    pub role: String,
}

/// Input data for inviting a user to a repository.
///
/// This assigns a role-based policy (Owner/Writer/Reader) to a user
/// with resource scope limited to the specific repository.
#[derive(Debug, Clone)]
pub struct InviteRepoMemberInputData<'a> {
    pub executor: &'a dyn tachyon_sdk::auth::ExecutorAction,
    pub multi_tenancy: &'a dyn tachyon_sdk::auth::MultiTenancyAction,
    /// Organization username
    pub org_username: String,
    /// Repository username
    pub repo_username: String,
    /// Repository ID
    pub repo_id: String,
    /// Username or email of the user to invite
    pub username_or_email: String,
    /// Role to assign: "owner", "writer", or "reader"
    pub role: String,
}

/// Port trait for inviting a user to a repository.
#[async_trait::async_trait]
pub trait InviteRepoMemberInputPort: Debug + Send + Sync {
    /// Execute the invite operation.
    ///
    /// Returns `Ok(())` on success, or an error if the invitation fails.
    async fn execute<'a>(
        &self,
        input: InviteRepoMemberInputData<'a>,
    ) -> errors::Result<()>;
}

/// Source of a repository permission
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionSource {
    /// Permission explicitly assigned to this repository
    Repo,
    /// Permission inherited from organization-level role (org owner)
    Org,
}

/// Represents a member of a repository
#[derive(Debug, Clone)]
pub struct RepoMemberInfo {
    pub user_id: tachyon_sdk::auth::UserId,
    pub policy_id: String,
    pub policy_name: Option<String>,
    pub resource_scope: Option<String>,
    pub assigned_at: chrono::DateTime<chrono::Utc>,
    pub user: Option<tachyon_sdk::auth::User>,
    /// Source of this permission (Repo or Org level)
    pub permission_source: PermissionSource,
}

/// Input data for getting repository members
#[derive(Debug, Clone)]
pub struct GetRepoMembersInputData<'a> {
    pub executor: &'a dyn tachyon_sdk::auth::ExecutorAction,
    pub multi_tenancy: &'a dyn tachyon_sdk::auth::MultiTenancyAction,
    pub resource_trn: &'a str,
    pub tenant_id: &'a value_object::TenantId,
}

/// Port trait for getting repository members (internal use for safety checks)
#[async_trait::async_trait]
pub trait GetRepoMembersInputPort: Debug + Send + Sync {
    /// Get all members with access to a repository
    async fn execute<'a>(
        &self,
        input: GetRepoMembersInputData<'a>,
    ) -> errors::Result<Vec<RepoMemberInfo>>;
}

/// Output data for getting repository policies
#[derive(Debug, Clone)]
pub struct RepoPolicyInfo {
    pub user_id: String,
    pub role: String,
    pub user: Option<tachyon_sdk::auth::User>,
    /// Source of this permission (Repo or Org level)
    pub permission_source: PermissionSource,
}

/// Input data for getting repository policies
#[derive(Debug, Clone)]
pub struct GetRepoPoliciesInputData<'a> {
    pub executor: &'a dyn tachyon_sdk::auth::ExecutorAction,
    pub multi_tenancy: &'a dyn tachyon_sdk::auth::MultiTenancyAction,
    pub resource_trn: &'a str,
    pub tenant_id: &'a value_object::TenantId,
}

/// Port trait for getting repository policies
#[async_trait::async_trait]
pub trait GetRepoPoliciesInputPort: Debug + Send + Sync {
    /// Get all policies for a repository
    async fn execute<'a>(
        &self,
        input: GetRepoPoliciesInputData<'a>,
    ) -> errors::Result<Vec<RepoPolicyInfo>>;
}

/// Input data for removing a user from a repository.
#[derive(Debug, Clone)]
pub struct RemoveRepoMemberInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    /// Repository ID
    pub repo_id: String,
    /// User ID to remove
    pub user_id: String,
}

/// Port trait for removing a user from a repository.
#[async_trait::async_trait]
pub trait RemoveRepoMemberInputPort: Debug + Send + Sync {
    /// Execute the remove operation.
    async fn execute<'a>(
        &self,
        input: RemoveRepoMemberInputData<'a>,
    ) -> errors::Result<()>;
}

/// Input data for changing a user's role in a repository.
#[derive(Debug, Clone)]
pub struct ChangeRepoMemberRoleInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    /// Repository ID
    pub repo_id: String,
    /// User ID whose role to change
    pub user_id: String,
    /// New role: "owner", "writer", or "reader"
    pub new_role: String,
}

/// Port trait for changing a user's role in a repository.
#[async_trait::async_trait]
pub trait ChangeRepoMemberRoleInputPort: Debug + Send + Sync {
    /// Execute the role change operation.
    async fn execute<'a>(
        &self,
        input: ChangeRepoMemberRoleInputData<'a>,
    ) -> errors::Result<()>;
}

/// TODO: add English documentation
#[derive(Debug, Clone)]
pub struct DeleteRepoInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,

    pub org_username: String,
    pub repo_username: String,
}

/// TODO: add English documentation
#[derive(Debug, Clone)]
pub struct AddPropertyInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,

    pub org_username: String,
    pub repo_username: String,
    pub property_name: String,
    pub property_type: database_manager::domain::PropertyType,
}

/// TODO: add English documentation
#[derive(Debug, Clone)]
pub struct UpdatePropertyInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,

    pub org_username: String,
    pub repo_username: String,

    pub property_id: String,
    pub property_name: Option<String>,
    pub property_type: Option<&'a database_manager::domain::PropertyType>,
    /// JSON metadata for property configuration (e.g., ext_github repos)
    pub meta_json: Option<Option<String>>,
}

/// TODO: add English documentation
#[derive(Debug, Clone)]
pub struct DeletePropertyInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,

    pub org_username: String,
    pub repo_username: String,
    pub property_id: String,
}

/// TODO: add English documentation
#[derive(Debug, Clone)]
pub struct GetPropertiesInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,

    pub org_username: String,
    pub repo_username: String,
}

/// TODO: add English documentation
#[derive(Debug, Clone)]
pub struct GetPropertyInputData<'a> {
    #[allow(dead_code)]
    pub executor: &'a dyn ExecutorAction,
    #[allow(dead_code)]
    pub multi_tenancy: &'a dyn MultiTenancyAction,

    pub org_username: String,
    pub repo_username: String,
    pub property_id: String,
}

/// TODO: add English documentation
#[derive(Debug, Clone)]
pub struct CreateSourceInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,

    pub org_username: String,
    pub repo_username: String,
    pub name: &'a value_object::Text,
    pub url: Option<value_object::Url>,
}

/// TODO: add English documentation
#[derive(Debug, Clone)]
pub struct UpdateSourceInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,

    pub source_id: &'a crate::domain::SourceId,
    pub org_username: String,
    pub repo_username: String,
    pub name: Option<value_object::Text>,
    pub url: Option<Option<value_object::Url>>,
}

/// TODO: add English documentation
#[derive(Debug, Clone)]
pub struct DeleteSourceInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,

    pub source_id: &'a crate::domain::SourceId,
    pub org_username: String,
    pub repo_username: String,
}

/// TODO: add English documentation
#[derive(Debug, Clone)]
pub struct GetSourceInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,

    pub source_id: &'a crate::domain::SourceId,
    pub org_username: String,
    pub repo_username: String,
}

/// TODO: add English documentation
#[derive(Debug, Clone)]
pub struct FindSourcesInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,

    pub repo_id: &'a crate::domain::RepoId,
    pub org_username: String,
    pub repo_username: String,
}

/// TODO: add English documentation
#[async_trait::async_trait]
pub trait BulkSyncExtGithubInputPort: Debug + Send + Sync {
    /// TODO: add English documentation
    async fn execute<'a>(
        &self,
        input: BulkSyncExtGithubInputData<'a>,
    ) -> errors::Result<BulkSyncExtGithubOutputData>;
}

/// TODO: add English documentation
#[derive(Debug, Clone)]
pub struct BulkSyncExtGithubInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,

    pub org_username: String,
    pub repo_username: String,
    /// TODO: add English documentation
    pub ext_github_property_id: String,
    /// TODO: add English documentation
    pub repo_configs: Vec<ExtGithubRepoConfig>,
}

/// TODO: add English documentation
#[derive(Debug, Clone)]
pub struct ExtGithubRepoConfig {
    /// TODO: add English documentation
    pub repo: String,
    /// TODO: add English documentation
    pub label: Option<String>,
    /// TODO: add English documentation
    pub default_path: Option<String>,
}

/// TODO: add English documentation
#[derive(Debug, Clone)]
pub struct BulkSyncExtGithubOutputData {
    /// TODO: add English documentation
    pub updated_count: usize,
    /// TODO: add English documentation
    pub skipped_count: usize,
    /// TODO: add English documentation
    pub total_count: usize,
}

// ==================== GitHub Import Usecases ====================

/// TODO: add English documentation
#[async_trait::async_trait]
pub trait ListGitHubDirectoryInputPort: Debug + Send + Sync {
    async fn execute<'a>(
        &self,
        input: ListGitHubDirectoryInputData<'a>,
    ) -> errors::Result<ListGitHubDirectoryOutputData>;
}

/// TODO: add English documentation
#[derive(Debug, Clone)]
pub struct ListGitHubDirectoryInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    /// TODO: add English documentation
    pub github_repo: String,
    /// TODO: add English documentation
    pub path: String,
    /// TODO: add English documentation
    pub ref_name: Option<String>,
}

/// TODO: add English documentation
#[derive(Debug, Clone)]
pub struct GitHubFileInfo {
    /// TODO: add English documentation
    pub name: String,
    /// TODO: add English documentation
    pub path: String,
    /// TODO: add English documentation
    pub sha: String,
    /// TODO: add English documentation
    pub size: i64,
    /// TODO: add English documentation
    pub file_type: String,
    /// TODO: add English documentation
    pub html_url: Option<String>,
}

/// TODO: add English documentation
#[derive(Debug, Clone)]
pub struct ListGitHubDirectoryOutputData {
    /// TODO: add English documentation
    pub files: Vec<GitHubFileInfo>,
    /// TODO: add English documentation
    pub truncated: bool,
}

/// TODO: add English documentation
#[async_trait::async_trait]
pub trait GetMarkdownPreviewsInputPort: Debug + Send + Sync {
    async fn execute<'a>(
        &self,
        input: GetMarkdownPreviewsInputData<'a>,
    ) -> errors::Result<Vec<MarkdownImportPreview>>;
}

/// TODO: add English documentation
#[derive(Debug, Clone)]
pub struct GetMarkdownPreviewsInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    /// TODO: add English documentation
    pub github_repo: String,
    /// TODO: add English documentation
    pub paths: Vec<String>,
    /// TODO: add English documentation
    pub ref_name: Option<String>,
}

/// TODO: add English documentation
#[derive(Debug, Clone)]
pub struct MarkdownImportPreview {
    /// TODO: add English documentation
    pub path: String,
    /// TODO: add English documentation
    pub sha: String,
    /// TODO: add English documentation
    pub frontmatter_json: Option<String>,
    /// TODO: add English documentation
    pub frontmatter_keys: Vec<String>,
    /// TODO: add English documentation
    pub suggested_name: String,
    /// TODO: add English documentation
    pub body_preview: String,
    /// TODO: add English documentation
    pub parse_error: Option<String>,
}

/// TODO: add English documentation
#[async_trait::async_trait]
pub trait AnalyzeFrontmatterInputPort: Debug + Send + Sync {
    async fn execute<'a>(
        &self,
        input: GetMarkdownPreviewsInputData<'a>,
    ) -> errors::Result<FrontmatterAnalysis>;
}

/// TODO: add English documentation
#[derive(Debug, Clone)]
pub struct SuggestedProperty {
    /// TODO: add English documentation
    pub key: String,
    /// TODO: add English documentation
    pub suggested_type: String,
    /// TODO: add English documentation
    pub unique_values: Vec<String>,
    /// TODO: add English documentation
    pub suggest_select: bool,
}

/// TODO: add English documentation
#[derive(Debug, Clone)]
pub struct FrontmatterAnalysis {
    /// TODO: add English documentation
    pub properties: Vec<SuggestedProperty>,
    /// TODO: add English documentation
    pub total_files: i32,
    /// TODO: add English documentation
    pub valid_files: i32,
}

/// TODO: add English documentation
#[async_trait::async_trait]
pub trait ImportMarkdownFromGitHubInputPort: Debug + Send + Sync {
    async fn execute<'a>(
        &self,
        input: ImportMarkdownFromGitHubInputData<'a>,
    ) -> errors::Result<ImportMarkdownResult>;
}

/// TODO: add English documentation
#[derive(Debug, Clone)]
pub struct PropertyMapping {
    /// TODO: add English documentation
    pub frontmatter_key: String,
    /// TODO: add English documentation
    pub property_name: String,
    /// TODO: add English documentation
    pub property_type: String,
    /// TODO: add English documentation
    pub select_options: Option<Vec<String>>,
}

/// TODO: add English documentation
#[derive(Debug, Clone)]
pub struct ImportMarkdownFromGitHubInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    /// TODO: add English documentation
    pub org_username: String,
    /// TODO: add English documentation
    pub repo_username: String,
    /// TODO: add English documentation
    pub repo_name: Option<String>,
    /// TODO: add English documentation
    pub github_repo: String,
    /// TODO: add English documentation
    pub paths: Vec<String>,
    /// TODO: add English documentation
    pub ref_name: Option<String>,
    /// TODO: add English documentation
    pub property_mappings: Vec<PropertyMapping>,
    /// TODO: add English documentation
    pub content_property_name: String,
    /// TODO: add English documentation
    pub skip_existing: bool,
    /// TODO: add English documentation
    pub enable_github_sync: bool,
}

/// TODO: add English documentation
#[derive(Debug, Clone)]
pub struct ImportError {
    /// TODO: add English documentation
    pub path: String,
    /// TODO: add English documentation
    pub message: String,
}

/// TODO: add English documentation
#[derive(Debug, Clone)]
pub struct ImportMarkdownResult {
    /// TODO: add English documentation
    pub imported_count: i32,
    /// TODO: add English documentation
    pub updated_count: i32,
    /// TODO: add English documentation
    pub skipped_count: i32,
    /// TODO: add English documentation
    pub errors: Vec<ImportError>,
    /// TODO: add English documentation
    pub data_ids: Vec<String>,
    /// TODO: add English documentation
    pub repo_id: String,
}
