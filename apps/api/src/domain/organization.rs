use std::fmt::Debug;

use derive_getters::Getters;
use value_object::{
    Identifier, LongText, PlatformId, TenantId, Text, Url, UserId,
};

use super::LIBRARY_TENANT;

#[derive(Debug, Clone, Getters)]
pub struct Organization {
    id: TenantId,
    name: Text,
    username: Identifier,
    description: Option<LongText>,
    website: Option<Url>,
    personal_owner_user_id: Option<UserId>,
    /// The tachyon platform the organization's tenant belongs to. An
    /// organization created in Library lives on Library's platform; one
    /// adopted from tachyon keeps the platform it came from. `None` only for
    /// rows stored before the platform was recorded (see
    /// `place_on_home_platform`).
    home_platform: Option<PlatformId>,
}

impl Organization {
    /// An organization created in Library, on Library's platform.
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
            home_platform: Some(LIBRARY_TENANT.clone()),
        }
    }

    /// An organization adopted from an existing tachyon tenant, which keeps
    /// the platform that tenant belongs to.
    pub fn adopt(
        id: &TenantId,
        name: &Text,
        username: &Identifier,
        home_platform: &PlatformId,
    ) -> Self {
        Self {
            home_platform: Some(home_platform.clone()),
            ..Self::new(id, name, username, None, None)
        }
    }

    /// Restore a stored organization exactly as persisted.
    pub fn reconstruct(
        id: &TenantId,
        name: &Text,
        username: &Identifier,
        description: Option<&LongText>,
        website: Option<&Url>,
        personal_owner_user_id: Option<&UserId>,
        home_platform: Option<&PlatformId>,
    ) -> Self {
        Self {
            personal_owner_user_id: personal_owner_user_id.cloned(),
            home_platform: home_platform.cloned(),
            ..Self::new(id, name, username, description, website)
        }
    }

    /// Record the platform of an organization stored before platforms were
    /// recorded. A tenant's platform never changes, so a recorded platform
    /// cannot be replaced by a different one.
    pub fn place_on_home_platform(
        self,
        platform: &PlatformId,
    ) -> errors::Result<Self> {
        match &self.home_platform {
            Some(recorded) if recorded != platform => {
                Err(errors::Error::conflict(format!(
                    "organization {} is already on platform {recorded}",
                    self.id
                )))
            }
            _ => Ok(Self {
                home_platform: Some(platform.clone()),
                ..self
            }),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn org() -> Organization {
        Organization::reconstruct(
            &"tn_01kshr165k3wwr8xyk0e6vy36c".parse().unwrap(),
            &Text::new("bernard-square").unwrap(),
            &"bernard-square".parse().unwrap(),
            None,
            None,
            None,
            None,
        )
    }

    #[test]
    fn home_platform_is_recorded_once_and_never_replaced() {
        let other: PlatformId =
            "tn_01hjryxysgey07h5jz5wagqj0m".parse().unwrap();
        let placed = org().place_on_home_platform(&other).unwrap();
        assert_eq!(placed.home_platform(), &Some(other.clone()));
        assert!(placed.clone().place_on_home_platform(&other).is_ok());
        assert!(matches!(
            placed.place_on_home_platform(&LIBRARY_TENANT),
            Err(errors::Error::Conflict { .. })
        ));
    }

    #[test]
    fn created_and_adopted_organizations_carry_their_platform() {
        let id: TenantId = "tn_01kshr165k3wwr8xyk0e6vy36c".parse().unwrap();
        let name = Text::new("Bernard Square").unwrap();
        let username: Identifier = "bernard-square".parse().unwrap();
        assert_eq!(
            Organization::new(&id, &name, &username, None, None)
                .home_platform(),
            &Some(LIBRARY_TENANT.clone())
        );
        let other: PlatformId =
            "tn_01hjryxysgey07h5jz5wagqj0m".parse().unwrap();
        assert_eq!(
            Organization::adopt(&id, &name, &username, &other)
                .home_platform(),
            &Some(other)
        );
    }
}
