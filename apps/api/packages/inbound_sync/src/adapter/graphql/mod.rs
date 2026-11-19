//! GraphQL API for library sync.

mod mutation;
mod query;
mod types;

pub use mutation::*;
pub use query::*;
pub use types::*;

use async_graphql::{Context, Result};
use tachyon_sdk::auth::{
    AuthApp, CheckPolicyInput, ExecutorAction, MultiTenancyAction,
};
use value_object::TenantId;

/// Authorize an inbound sync GraphQL call and return the tenant it may act
/// on.
///
/// The operator tenant comes from the caller-supplied `x-operator-id`
/// header, so the policy check alone is not enough: an anonymous request
/// would otherwise be evaluated with Library's process credential, and a
/// caller could name a tenant they do not belong to. Every resolver that
/// reads or writes tenant-owned sync records goes through here and then
/// compares the record's tenant with the returned one.
pub(crate) async fn authorize_sync_tenant(
    ctx: &Context<'_>,
    auth: &dyn AuthApp,
    action: &'static str,
) -> Result<TenantId> {
    authorize_sync_tenant_for(ctx, auth, action, None).await
}

/// Like [`authorize_sync_tenant`], for resolvers that also name the tenant
/// in a `tenantId` argument.
///
/// GraphQL allows the `x-operator-id` header to be omitted; in that case the
/// argument names the tenant to authorize in (as `createRepo` does with its
/// destination). When both are present they must agree.
pub(crate) async fn authorize_sync_tenant_for(
    ctx: &Context<'_>,
    auth: &dyn AuthApp,
    action: &'static str,
    requested: Option<TenantId>,
) -> Result<TenantId> {
    let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
    let header = ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
    let scope = authorization_scope(header, requested)?;
    let tenant = scope.get_operator_id()?;
    ensure_tenant_member(executor, &tenant)?;
    auth.check_policy(&CheckPolicyInput {
        executor,
        multi_tenancy: &scope,
        action,
    })
    .await?;
    Ok(tenant)
}

fn authorization_scope(
    header: &tachyon_sdk::auth::MultiTenancy,
    requested: Option<TenantId>,
) -> errors::Result<tachyon_sdk::auth::MultiTenancy> {
    match (header.operator_id(), requested) {
        (Some(operator), Some(requested)) if operator != requested => {
            Err(errors::Error::forbidden(
                "tenantId does not match the authorized tenant",
            ))
        }
        (Some(_), _) => Ok(header.clone()),
        (None, Some(requested)) => {
            Ok(tachyon_sdk::auth::MultiTenancy::new(
                header.platform_id(),
                Some(requested),
            ))
        }
        (None, None) => {
            Err(errors::Error::business_logic("Operator ID is required"))
        }
    }
}

/// Membership-only guard for resolvers whose usecase performs its own policy
/// check. Rejects anonymous callers and tenants the caller does not belong
/// to before any upstream call is made.
pub(crate) fn require_sync_tenant_member(
    ctx: &Context<'_>,
) -> Result<TenantId> {
    let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
    let multi_tenancy = ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
    let tenant = multi_tenancy.get_operator_id()?;
    ensure_tenant_member(executor, &tenant)?;
    Ok(tenant)
}

fn ensure_tenant_member(
    executor: &dyn ExecutorAction,
    tenant: &TenantId,
) -> errors::Result<()> {
    if executor.is_none() {
        return Err(errors::Error::unauthorized(
            "Authentication is required for integration settings",
        ));
    }
    if !executor.has_tenant_id(tenant) {
        return Err(errors::Error::forbidden(
            "The caller does not belong to the requested tenant",
        ));
    }
    Ok(())
}

/// Return the provider config as JSON with every secret-bearing field
/// removed. Webhook signing keys must never be readable back through the
/// API; the endpoint keeps them server-side.
pub(crate) fn redacted_config_json<T: serde::Serialize>(
    config: &T,
) -> String {
    let mut value = match serde_json::to_value(config) {
        Ok(value) => value,
        Err(_) => return String::new(),
    };
    strip_secret_fields(&mut value);
    serde_json::to_string(&value).unwrap_or_default()
}

fn strip_secret_fields(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            map.retain(|key, _| {
                !key.to_ascii_lowercase().contains("secret")
            });
            map.values_mut().for_each(strip_secret_fields);
        }
        serde_json::Value::Array(items) => {
            items.iter_mut().for_each(strip_secret_fields);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tachyon_sdk::auth::MockExecutorAction;

    fn tenant() -> TenantId {
        "tn_01hjryxysgey07h5jz5wagqj0m".parse().unwrap()
    }

    #[test]
    fn anonymous_caller_is_rejected() {
        let mut executor = MockExecutorAction::new();
        executor.expect_is_none().return_const(true);
        let err = ensure_tenant_member(&executor, &tenant()).unwrap_err();
        assert!(
            matches!(err, errors::Error::Unauthorized { .. }),
            "{err:?}"
        );
    }

    #[test]
    fn caller_outside_the_tenant_is_rejected() {
        let mut executor = MockExecutorAction::new();
        executor.expect_is_none().return_const(false);
        executor.expect_has_tenant_id().return_const(false);
        let err = ensure_tenant_member(&executor, &tenant()).unwrap_err();
        assert!(matches!(err, errors::Error::Forbidden { .. }), "{err:?}");
    }

    #[test]
    fn tenant_member_is_accepted() {
        let mut executor = MockExecutorAction::new();
        executor.expect_is_none().return_const(false);
        executor.expect_has_tenant_id().return_const(true);
        ensure_tenant_member(&executor, &tenant()).unwrap();
    }

    fn other_tenant() -> TenantId {
        "tn_01hjryxysgey07h5jz5wagqj0n".parse().unwrap()
    }

    #[test]
    fn tenant_argument_is_used_when_the_header_has_no_operator() {
        let header = tachyon_sdk::auth::MultiTenancy::new(None, None);
        let scope = authorization_scope(&header, Some(tenant())).unwrap();
        assert_eq!(scope.get_operator_id().unwrap(), tenant());
    }

    #[test]
    fn tenant_argument_must_match_the_header_operator() {
        let header =
            tachyon_sdk::auth::MultiTenancy::new(None, Some(tenant()));
        let err =
            authorization_scope(&header, Some(other_tenant())).unwrap_err();
        assert!(matches!(err, errors::Error::Forbidden { .. }), "{err:?}");
        let scope = authorization_scope(&header, Some(tenant())).unwrap();
        assert_eq!(scope.get_operator_id().unwrap(), tenant());
    }

    #[test]
    fn missing_operator_and_argument_is_rejected() {
        let header = tachyon_sdk::auth::MultiTenancy::new(None, None);
        assert!(authorization_scope(&header, None).is_err());
    }

    #[test]
    fn linear_webhook_secret_is_not_serialized() {
        let config = inbound_sync_domain::ProviderConfig::Linear {
            team_id: Some("team".into()),
            project_id: None,
            webhook_secret: Some("lin_wh_dummy".into()),
        };
        let json = redacted_config_json(&config);
        assert!(!json.contains("lin_wh_dummy"), "{json}");
        assert!(!json.contains("webhook_secret"), "{json}");
        assert!(json.contains("team"), "{json}");
    }
}
