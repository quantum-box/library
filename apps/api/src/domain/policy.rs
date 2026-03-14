//! Library-specific policy constants.
//!
//! These policy IDs are defined in the auth seed data
//! (scripts/seeds/n1-seed/008-auth-policies.yaml).

use tachyon_sdk::auth::PolicyId;

/// Policy for basic library user access.
/// Grants standard read/write permissions for library resources.
pub const LIBRARY_USER_POLICY_ID: &str = "pol_01libraryuserpolicy";

/// Policy for full repository access within an organization.
/// Attached to org owners to grant access to all repos (resource_scope = NULL).
pub const LIBRARY_REPO_OWNER_POLICY_ID: &str = "pol_01libraryrepoowner";

/// Create a PolicyId for library user policy.
pub fn library_user_policy_id() -> PolicyId {
    PolicyId::new(LIBRARY_USER_POLICY_ID)
}

/// Create a PolicyId for library repo owner policy.
pub fn library_repo_owner_policy_id() -> PolicyId {
    PolicyId::new(LIBRARY_REPO_OWNER_POLICY_ID)
}
