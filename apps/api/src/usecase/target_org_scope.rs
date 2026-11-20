use tachyon_sdk::auth::MultiTenancyAction;
use value_object::TenantId;

use crate::domain::LIBRARY_TENANT;

/// Reject a request whose authorization tenant names an organization other
/// than the one it acts on.
///
/// On GraphQL the authorization tenant is the caller's `x-operator-id`
/// header, while the organization acted on comes from the arguments. A
/// policy that passes in the caller's own organization must not reach
/// another organization's repositories, data or profile. The two tenants a
/// request may be authorized in are the target organization itself (MCP and
/// v2 send it) and Library's platform tenant, which the v1 web client sends
/// by default and where repository-scoped grants are attached. A request
/// with no operator is left to the policy check.
pub(crate) fn ensure_scope_targets_org(
    scope: &dyn MultiTenancyAction,
    organization_id: &TenantId,
) -> errors::Result<()> {
    // Without an operator the request is not authorized in any
    // organization's tenant, so there is no other organization to confuse.
    let Some(operator) = scope.operator_id() else {
        return Ok(());
    };
    if &operator == organization_id || operator == *LIBRARY_TENANT {
        return Ok(());
    }
    Err(errors::Error::forbidden(
        "The organization does not match the authorization tenant",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tachyon_sdk::auth::MultiTenancy;

    fn tenant(suffix: char) -> TenantId {
        format!("tn_01hjryxysgey07h5jz5wagqj0{suffix}")
            .parse()
            .unwrap()
    }

    #[test]
    fn another_organization_tenant_is_rejected() {
        let scope = MultiTenancy::new(None, Some(tenant('a')));
        let err =
            ensure_scope_targets_org(&scope, &tenant('b')).unwrap_err();
        assert!(matches!(err, errors::Error::Forbidden { .. }), "{err:?}");
    }

    #[test]
    fn the_target_organization_tenant_is_accepted() {
        let scope = MultiTenancy::new(None, Some(tenant('b')));
        ensure_scope_targets_org(&scope, &tenant('b')).unwrap();
    }

    #[test]
    fn a_scope_without_an_operator_is_left_to_the_policy_check() {
        let scope = MultiTenancy::new(None, None);
        ensure_scope_targets_org(&scope, &tenant('b')).unwrap();
    }

    #[test]
    fn the_platform_tenant_is_accepted() {
        let scope = MultiTenancy::new(None, Some(LIBRARY_TENANT.clone()));
        ensure_scope_targets_org(&scope, &tenant('b')).unwrap();
    }
}
