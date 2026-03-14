mod add_data;
mod add_property;
mod analyze_frontmatter;
mod boundary;
mod bulk_sync_ext_github;
mod change_org_member_role;
mod change_repo_member_role;
mod change_repo_policy;
pub mod change_repo_username;
mod create_api_key;
mod create_organization;
mod create_pull_request;
mod create_repo;
mod create_source;
mod delete_data;
mod delete_property;
mod delete_repo;
mod delete_source;
mod find_sources;
mod get_markdown_previews;
mod get_properties;
mod get_repo_members;
mod get_repo_policies;
mod get_source;
mod import_markdown_from_github;
mod invite_org_member;
mod invite_repo_member;
mod library_org;
mod list_api_keys;
mod list_github_directory;
pub mod markdown_composer;
mod remove_repo_member;
mod search_data;
mod search_repo;
mod sign_in;
mod update_data;
mod update_organization;
mod update_property;
mod update_repo;
mod update_source;
mod view_data;
mod view_data_list;
mod view_organization;
mod view_repo;

use std::fmt::Debug;

use errors::Result;
use value_object::Identifier;

use crate::domain::{Organization, Repo};

pub use library_org::LibraryOrg;

pub use add_data::*;
pub use add_property::*;
pub use analyze_frontmatter::*;
pub use boundary::*;
pub use bulk_sync_ext_github::*;
pub use change_org_member_role::*;
pub use change_repo_member_role::*;
#[allow(unused_imports)]
pub use change_repo_policy::*;
pub use change_repo_username::*;
pub use create_api_key::*;
pub use create_organization::*;
pub use create_repo::*;
pub use create_source::*;
pub use delete_data::*;
pub use delete_property::*;
pub use delete_repo::*;
pub use delete_source::*;
pub use find_sources::*;
pub use get_markdown_previews::*;
pub use get_properties::*;
pub use get_repo_members::*;
pub use get_repo_policies::*;
pub use get_source::*;
pub use import_markdown_from_github::*;
pub use invite_org_member::*;
pub use invite_repo_member::*;
pub use list_api_keys::*;
pub use list_github_directory::*;
pub use remove_repo_member::*;
pub use search_data::*;
pub use search_repo::*;
pub use sign_in::*;
pub use update_data::*;
pub use update_organization::*;
pub use update_property::*;
pub use update_repo::*;
pub use update_source::*;
pub use view_data::*;
pub use view_data_list::*;
pub use view_organization::*;

pub struct UpdateOrganizationInputData<'a> {
    pub executor: &'a dyn tachyon_sdk::auth::ExecutorAction,
    pub multi_tenancy: &'a dyn tachyon_sdk::auth::MultiTenancyAction,
    pub username: String,
    pub name: String,
    pub description: Option<String>,
    pub website: Option<String>,
}

pub struct UpdateOrganizationOutputData {
    pub organization: Organization,
}

#[async_trait::async_trait]
pub trait UpdateOrganizationInputPort: Debug + Send + Sync {
    async fn execute(
        &self,
        input: &UpdateOrganizationInputData<'_>,
    ) -> errors::Result<UpdateOrganizationOutputData>;
}
#[allow(unused_imports)]
pub use change_repo_policy::*;
pub use view_repo::*;

#[async_trait::async_trait]
pub trait GetOrganizationByUsernameQuery: Debug + Send + Sync {
    async fn execute(
        &self,
        username: &Identifier,
    ) -> Result<Option<Organization>>;
}

#[async_trait::async_trait]
pub trait GetRepoByUsernameQuery: Debug + Send + Sync {
    async fn execute(
        &self,
        organization_username: &Identifier,
        repo_username: &Identifier,
    ) -> Result<Option<Repo>>;
}
