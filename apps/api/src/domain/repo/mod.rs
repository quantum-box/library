mod policy;
mod role;
mod source;
mod visibility;

use derive_getters::Getters;
#[allow(unused_imports)]
pub use policy::*;
pub use role::*;
pub use source::*;
use tachyon_sdk::auth::UserId;
pub use visibility::*;

use database_manager::domain::DatabaseId;
use util::macros::*;
use value_object::{Identifier, LongText, OperatorId, TenantId, Text};

def_id!(RepoId, "rp_");

// TODO: add English comment
#[async_trait::async_trait]
pub trait RepoRepository:
    std::marker::Send + Sync + std::fmt::Debug
{
    // TODO: add English comment
    async fn save(&self, entity: &Repo) -> errors::Result<()>;

    #[allow(dead_code)]
    async fn get_by_id(
        &self,
        tenant_id: &TenantId,
        id: &RepoId,
    ) -> errors::Result<Option<Repo>>;

    async fn find_all(
        &self,
        org_id: &TenantId,
    ) -> errors::Result<Vec<Repo>>;

    async fn delete(
        &self,
        tenant_id: &TenantId,
        id: &RepoId,
    ) -> errors::Result<()>;
}

#[derive(Debug, Clone, Getters)]
pub struct Repo {
    id: RepoId,
    organization_id: OperatorId,
    org_username: Identifier,
    name: Text,
    username: Identifier,
    is_public: bool,
    description: Option<LongText>,
    databases: Vec<DatabaseId>,
    tags: Vec<Text>,
}

impl Repo {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: &RepoId,
        organization_id: &OperatorId,
        org_username: &Identifier,
        name: &Text,
        username: &Identifier,
        is_public: bool,
        description: Option<LongText>,
        databases: Vec<DatabaseId>,
        tags: Vec<Text>,
    ) -> Self {
        Self {
            id: id.clone(),
            organization_id: organization_id.clone(),
            org_username: org_username.clone(),
            name: name.clone(),
            username: username.clone(),
            is_public,
            description,
            databases,
            tags,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create(
        id: &RepoId,
        organization_id: &OperatorId,
        org_username: &Identifier,
        name: &Text,
        username: &Identifier,
        _create_user: &UserId,
        is_public: bool,
        description: Option<LongText>,
        databases: Vec<DatabaseId>,
        tags: Vec<Text>,
    ) -> Self {
        Self::new(
            id,
            organization_id,
            org_username,
            name,
            username,
            is_public,
            description,
            databases,
            tags,
        )
    }

    pub fn update(
        &self,
        name: Option<Text>,
        description: Option<LongText>,
        is_public: Option<bool>,
        tags: Option<Vec<Text>>,
    ) -> Self {
        Self {
            name: name.unwrap_or(self.name.clone()),
            description,
            is_public: is_public.unwrap_or(self.is_public),
            tags: tags.unwrap_or(self.tags.clone()),
            ..self.clone()
        }
    }

    pub fn add_tag(&mut self, tag: &Text) -> Self {
        if !self.tags.contains(tag) {
            self.tags.push(tag.clone());
        }
        self.clone()
    }

    pub fn remove_tag(&mut self, tag: &Text) -> Self {
        self.tags.retain(|t| t != tag);
        self.clone()
    }

    pub fn with_operator_alias(&self, username: &Identifier) -> Self {
        Self {
            username: username.clone(),
            ..self.clone()
        }
    }

    pub fn is_private(&self) -> bool {
        !self.is_public
    }

    pub fn add_database(&mut self, database_id: &DatabaseId) -> Self {
        self.databases.push(database_id.clone());
        self.clone()
    }

    // TODO: add English comment
}
