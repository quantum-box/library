use std::fmt::Debug;

use derive_getters::Getters;
use value_object::{Identifier, LongText, TenantId, Text, Url};

#[derive(Debug, Clone, Getters)]
pub struct Organization {
    id: TenantId,
    name: Text,
    username: Identifier,
    description: Option<LongText>,
    website: Option<Url>,
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
        }
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
    #[allow(dead_code)]
    async fn find_all(&self) -> errors::Result<Vec<Organization>>;
    #[allow(dead_code)]
    async fn delete(&self, org_id: &TenantId) -> errors::Result<()>;
}
