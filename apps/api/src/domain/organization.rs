use std::fmt::Debug;

use derive_getters::Getters;
use value_object::{Identifier, LongText, TenantId, Text, Url, UserId};

#[derive(Debug, Clone, Getters)]
pub struct Organization {
    id: TenantId,
    name: Text,
    username: Identifier,
    description: Option<LongText>,
    website: Option<Url>,
    personal_owner_user_id: Option<UserId>,
}

impl Organization {
    pub fn new(
        id: &TenantId,
        name: &Text,
        username: &Identifier,
        description: Option<&LongText>,
        website: Option<&Url>,
    ) -> Self {
        Self {
            id: id.clone(),
            name: name.clone(),
            username: username.clone(),
            description: description.cloned(),
            website: website.cloned(),
            personal_owner_user_id: None,
        }
    }

    pub fn with_personal_owner(mut self, user_id: &UserId) -> Self {
        self.personal_owner_user_id = Some(user_id.clone());
        self
    }
}

#[async_trait::async_trait]
pub trait OrganizationRepository: Debug + Send + Sync + 'static {
    async fn insert(
        &self,
        organization: &Organization,
    ) -> errors::Result<()>;
    async fn update(
        &self,
        organization: &Organization,
    ) -> errors::Result<()>;
    async fn get_by_id(
        &self,
        org_id: &TenantId,
    ) -> errors::Result<Option<Organization>>;
    async fn get_by_username(
        &self,
        username: &Identifier,
    ) -> errors::Result<Option<Organization>>;
    async fn get_by_personal_owner_user_id(
        &self,
        user_id: &UserId,
    ) -> errors::Result<Option<Organization>> {
        let _ = user_id;
        Ok(None)
    }
    #[allow(dead_code)]
    async fn find_all(&self) -> errors::Result<Vec<Organization>>;
    #[allow(dead_code)]
    async fn delete(&self, org_id: &TenantId) -> errors::Result<()>;
}
