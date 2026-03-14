//! Library-specific role definitions.

use async_graphql::Enum;

/// Role for library organization members.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum)]
pub enum OrgRole {
    /// Organization owner with full access to all repositories.
    Owner,
    /// Manager with elevated permissions.
    Manager,
    /// General member with basic permissions.
    General,
}

impl From<OrgRole> for tachyon_sdk::auth::DefaultRole {
    fn from(role: OrgRole) -> Self {
        match role {
            OrgRole::Owner => tachyon_sdk::auth::DefaultRole::Owner,
            OrgRole::Manager => tachyon_sdk::auth::DefaultRole::Manager,
            OrgRole::General => tachyon_sdk::auth::DefaultRole::General,
        }
    }
}

impl From<tachyon_sdk::auth::DefaultRole> for OrgRole {
    fn from(role: tachyon_sdk::auth::DefaultRole) -> Self {
        match role {
            tachyon_sdk::auth::DefaultRole::Owner => OrgRole::Owner,
            tachyon_sdk::auth::DefaultRole::Manager => OrgRole::Manager,
            tachyon_sdk::auth::DefaultRole::General => OrgRole::General,
            // Store role does not exist in Library context; treat as General.
            tachyon_sdk::auth::DefaultRole::Store => OrgRole::General,
        }
    }
}
