//! PLT-942 Library MDM Hub Phase 1 (Registry).
//!
//! Maps an external system's local code (e.g. bakuure SKU `BWS-001`) to a
//! Library-issued `global_id`, scoped per tenant.

use chrono::{DateTime, Utc};
use derive_getters::Getters;
use util::macros::*;
use value_object::{TenantId, Text};

def_id!(GlobalIdMappingId, "gim_");
def_id!(GlobalId, "gid_");

#[async_trait::async_trait]
pub trait GlobalIdMappingRepository:
    std::marker::Send + Sync + std::fmt::Debug
{
    /// Insert a new mapping. Surfaces unique-key collisions as
    /// `errors::Error::conflict` so the create path can never silently merge
    /// onto a different row's primary key (would break the registry's name
    /// invariant — see PLT-942 / ADR-0001).
    async fn insert(&self, entity: &GlobalIdMapping) -> errors::Result<()>;

    /// Update the `name` of an existing mapping. Other fields are immutable
    /// in Phase 1; correct mistakes by deleting and recreating (deletion is
    /// out of scope for Phase 1).
    async fn update_name(
        &self,
        tenant_id: &TenantId,
        id: &GlobalIdMappingId,
        name: &Text,
    ) -> errors::Result<()>;

    async fn get_by_id(
        &self,
        tenant_id: &TenantId,
        id: &GlobalIdMappingId,
    ) -> errors::Result<Option<GlobalIdMapping>>;

    async fn find_by_system_code(
        &self,
        tenant_id: &TenantId,
        system: &str,
        system_code: &str,
    ) -> errors::Result<Option<GlobalIdMapping>>;

    async fn find_all(
        &self,
        tenant_id: &TenantId,
        system: Option<&str>,
    ) -> errors::Result<Vec<GlobalIdMapping>>;
}

#[derive(Debug, Clone, PartialEq, Eq, Getters)]
pub struct GlobalIdMapping {
    id: GlobalIdMappingId,
    tenant_id: TenantId,
    global_id: GlobalId,
    system: Text,
    system_code: Text,
    name: Text,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl GlobalIdMapping {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: &GlobalIdMappingId,
        tenant_id: &TenantId,
        global_id: &GlobalId,
        system: &Text,
        system_code: &Text,
        name: &Text,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id: id.clone(),
            tenant_id: tenant_id.clone(),
            global_id: global_id.clone(),
            system: system.clone(),
            system_code: system_code.clone(),
            name: name.clone(),
            created_at,
            updated_at,
        }
    }

    pub fn create(
        tenant_id: &TenantId,
        global_id: Option<GlobalId>,
        system: &Text,
        system_code: &Text,
        name: &Text,
    ) -> Self {
        let now = Utc::now();
        Self::new(
            &GlobalIdMappingId::default(),
            tenant_id,
            &global_id.unwrap_or_default(),
            system,
            system_code,
            name,
            now,
            now,
        )
    }

    pub fn update_name(&self, name: &Text) -> Self {
        Self {
            name: name.clone(),
            updated_at: Utc::now(),
            ..self.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    fn tenant() -> TenantId {
        "tn_01hjryxysgey07h5jz5wagqj0m".parse().unwrap()
    }

    #[rstest]
    fn create_assigns_default_ids() {
        let system: Text = "bakuure".parse().unwrap();
        let code: Text = "BWS-001".parse().unwrap();
        let name: Text = "Bakuure Widget Standard 001".parse().unwrap();

        let m =
            GlobalIdMapping::create(&tenant(), None, &system, &code, &name);

        assert!(m.id().as_str().starts_with("gim_"));
        assert!(m.global_id().as_str().starts_with("gid_"));
        assert_eq!(m.tenant_id(), &tenant());
        assert_eq!(m.system(), &system);
        assert_eq!(m.system_code(), &code);
        assert_eq!(m.name(), &name);
    }

    #[rstest]
    fn create_honors_explicit_global_id() {
        let gid: GlobalId =
            "gid_01hkz3700yt46snfewzpakeyj4".parse().unwrap();
        let system: Text = "bakuure".parse().unwrap();
        let code: Text = "BWS-001".parse().unwrap();
        let name: Text = "Widget".parse().unwrap();

        let m = GlobalIdMapping::create(
            &tenant(),
            Some(gid.clone()),
            &system,
            &code,
            &name,
        );

        assert_eq!(m.global_id(), &gid);
    }

    #[rstest]
    fn update_name_only_changes_name_and_updated_at() {
        let system: Text = "bakuure".parse().unwrap();
        let code: Text = "BWS-001".parse().unwrap();
        let name: Text = "Old".parse().unwrap();
        let m =
            GlobalIdMapping::create(&tenant(), None, &system, &code, &name);

        let new_name: Text = "New".parse().unwrap();
        let updated = m.update_name(&new_name);

        assert_eq!(updated.id(), m.id());
        assert_eq!(updated.global_id(), m.global_id());
        assert_eq!(updated.system(), m.system());
        assert_eq!(updated.system_code(), m.system_code());
        assert_eq!(updated.name(), &new_name);
    }
}
