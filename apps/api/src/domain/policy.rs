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

/// Companion policy granting `auth:CreateOperator`, which organization
/// creation needs on the tachyon side once it is authorized as the
/// caller. It is a custom policy — `LibraryUserPolicy` is a system
/// policy and cannot be amended through the API — so its id is
/// generated at apply time and injected per environment
/// (see .tachyon/manifests/library-api-runtime.yml).
///
/// `None` when the environment does not configure it; sign-in then
/// skips the grant instead of failing.
pub fn library_org_creator_policy_id() -> Option<PolicyId> {
    let id = std::env::var("LIBRARY_ORG_CREATOR_POLICY_ID").ok()?;
    let id = id.trim();
    if id.is_empty() {
        return None;
    }
    Some(PolicyId::new(id))
}
