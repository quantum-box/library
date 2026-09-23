//! The tachyon-side grant an organization owner needs to issue and remove
//! API keys that carry repository access (see
//! `library_api_key_issuer_policy_id`).

use tachyon_sdk::auth::{
    AttachUserPolicyInput, AuthApp, ExecutorAction, MultiTenancyAction,
    PolicyId,
};
use value_object::TenantId;

/// Attach the issuer policy to the calling user in the organization's
/// tenant. Callers check `library:ManageRepoPolicy` first: the grant lets
/// its holder attach policies to service accounts, which is only an
/// owner's to do.
///
/// Granted on use rather than when someone becomes an owner, so owners of
/// organizations that predate the policy need no backfill. The attachment
/// is idempotent upstream. A caller that is not a user (a key acting on
/// its own behalf) gets nothing: the grant is for people. `policy_id` is
/// `None` where the environment does not configure the policy.
pub(crate) async fn grant_api_key_issuer(
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
            "LIBRARY_API_KEY_ISSUER_POLICY_ID is not set; skipping the api key issuer grant"
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
