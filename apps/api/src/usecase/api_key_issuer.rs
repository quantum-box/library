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

/// Attach one of those policies to the calling user in the organization's
/// tenant, which is the only scope a check made there reads.
///
/// Granted on use rather than when someone becomes a member or an owner,
/// so organizations that predate these policies need no backfill. The
/// attachment is idempotent upstream. A caller that is not a user (a key
/// acting on its own behalf) gets nothing: the grant is for people.
/// `policy_id` is `None` where the environment does not configure the
/// policy, and the call then does nothing — issuing still works wherever
/// the caller already holds what it needs.
pub(crate) async fn grant_api_key_policy(
    auth_app: &dyn AuthApp,
    policy_id: Option<&PolicyId>,
    executor: &dyn ExecutorAction,
    multi_tenancy: &dyn MultiTenancyAction,
    tenant_id: &TenantId,
) -> errors::Result<()> {
    if !executor.is_user() {
        return Ok(());
    }
    let Some(policy_id) = policy_id else {
        tracing::debug!(
            "an api key policy id is not configured; skipping the grant"
        );
        return Ok(());
    };

    auth_app
        .attach_user_policy(&AttachUserPolicyInput {
            executor,
            multi_tenancy,
            user_id: &executor.get_user_id()?,
            policy_id,
            tenant_id,
        })
        .await
}
