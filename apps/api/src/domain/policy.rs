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

/// Companion policy granting what issuing an API key with repository
/// access needs on the tachyon side: `auth:AttachServiceAccountPolicy` to
/// grant the key's own service account its role, and
/// `auth:DeleteServiceAccount` to remove that account when the key goes.
/// Neither is in LibraryUserPolicy or LibraryRepoOwnerPolicy, and both are
/// system policies the API cannot amend, so the actions live in this
/// custom policy (`library:ApiKeyIssuer` in
/// .tachyon/manifests/library-api-key-policies.yml). It is shared with the
/// organizations under the Library platform and attached in the
/// organization's tenant, which is the only scope a check made there
/// reads.
///
/// The id is the one the manifest was applied under, like the repository
/// policies above: Library has one platform tenant, so there is no second
/// id for the same policy. `LIBRARY_API_KEY_ISSUER_POLICY_ID` overrides
/// it, for an environment where the policy was applied separately.
pub const LIBRARY_API_KEY_ISSUER_POLICY_ID: &str =
    "pol_01m36cfejtbmqgmk9pwccjjhn5";

pub fn library_api_key_issuer_policy_id() -> PolicyId {
    policy_id_from_env_or(
        "LIBRARY_API_KEY_ISSUER_POLICY_ID",
        LIBRARY_API_KEY_ISSUER_POLICY_ID,
    )
}

/// Companion policy granting `auth:CreateServiceAccount`, which issuing
/// any key needs now that each key has a service account of its own
/// (`library:ApiKeyAccounts` in
/// .tachyon/manifests/library-api-key-policies.yml). Separate from
/// [`library_api_key_issuer_policy_id`] because a key without repository
/// access is issued by members who are not owners, and an account created
/// on its own carries no policy.
/// `LIBRARY_API_KEY_ACCOUNTS_POLICY_ID` overrides it, as above.
pub const LIBRARY_API_KEY_ACCOUNTS_POLICY_ID: &str =
    "pol_01m36cff1vb3eftr27p4f12t1a";

pub fn library_api_key_accounts_policy_id() -> PolicyId {
    policy_id_from_env_or(
        "LIBRARY_API_KEY_ACCOUNTS_POLICY_ID",
        LIBRARY_API_KEY_ACCOUNTS_POLICY_ID,
    )
}

/// The applied id is the same in every environment Library runs, so it is
/// compiled in; the variable is the way out of that if one ever is not.
fn policy_id_from_env_or(variable: &str, applied: &str) -> PolicyId {
    let configured = std::env::var(variable).ok();
    let configured = configured
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    PolicyId::new(configured.unwrap_or(applied))
}

/// Tachyon's built-in tenant administrator policy.
///
/// This is the grant that actually makes someone an administrator of a
/// tenant, as opposed to the `role` label on their user record. It is
/// seeded platform-wide, so the id is the same in every environment.
pub const TENANT_ADMINISTRATOR_POLICY_ID: &str =
    "pol_01hjryxysgey07h5jz5w00001";

/// Whether `policy_id` makes its holder an administrator of the tenant
/// it is attached in.
pub fn is_tenant_administrator_policy(policy_id: &PolicyId) -> bool {
    policy_id.as_str() == TENANT_ADMINISTRATOR_POLICY_ID
}
