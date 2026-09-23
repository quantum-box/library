//! The tachyon-side grants issuing an API key needs, which Library's own
//! policies do not carry: creating the key's service account
//! (`library_api_key_accounts_policy_id`) and, for a key with repository
//! access, granting and later removing that account
//! (`library_api_key_issuer_policy_id`).

use tachyon_sdk::auth::{
    AttachUserPolicyInput, AuthApp, ExecutorAction, MultiTenancyAction,
    PolicyId,
};
use value_object::TenantId;

use crate::domain::{
    library_api_key_accounts_policy_id, library_api_key_issuer_policy_id,
    ApiKeyRole,
};

/// How many accounts Library asks about at once when it has to look
/// through an organization's accounts for keys. Every key has an account
/// of its own, so the number to ask about grows with the key count.
pub(crate) const CONCURRENT_ACCOUNT_LOOKUPS: usize = 8;

/// Attach one of those policies to the calling user in the organization's
/// tenant, which is the only scope a check made there reads.
///
/// Granted on use rather than when someone becomes a member or an owner,
/// so organizations that predate these policies need no backfill. The
/// attachment is idempotent upstream. A caller that is not a user (a key
/// acting on its own behalf) gets nothing: the grant is for people.
/// A refusal is not an error either. Attaching a policy is itself
/// something tachyon authorizes as the caller: an organization owner may
/// (their operator-owner grant carries it), a member may not. Whether the
/// caller needed the grant at all is answered by the operation it was for
/// — which is refused on its own terms if they did — so a refusal here is
/// reported by that, not by this.
pub(crate) async fn grant_api_key_policy(
    auth_app: &dyn AuthApp,
    policy_id: &PolicyId,
    executor: &dyn ExecutorAction,
    multi_tenancy: &dyn MultiTenancyAction,
    tenant_id: &TenantId,
) -> errors::Result<()> {
    if !executor.is_user() {
        return Ok(());
    }
    if let Err(error) = auth_app
        .attach_user_policy(&AttachUserPolicyInput {
            executor,
            multi_tenancy,
            user_id: &executor.get_user_id()?,
            policy_id,
            tenant_id,
        })
        .await
    {
        tracing::info!(
            policy = %policy_id,
            tenant = %tenant_id,
            error = %error,
            "api key policy was not granted to the caller"
        );
    }

    Ok(())
}

/// What the account behind a key with this role was given, and so what
/// has to come off it when the key goes. An owner key issues keys as
/// itself, which takes more than the role (see `create_api_key`).
pub(crate) fn api_key_account_policies(role: ApiKeyRole) -> Vec<PolicyId> {
    let mut policies = vec![role.policy_id()];
    if role == ApiKeyRole::Owner {
        policies.push(library_api_key_accounts_policy_id());
        policies.push(library_api_key_issuer_policy_id());
    }
    policies
}
