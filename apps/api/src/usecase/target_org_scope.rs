use tachyon_sdk::auth::{MultiTenancy, MultiTenancyAction};
use value_object::TenantId;

use crate::domain::LIBRARY_TENANT;

/// The scope an action on `organization_id` is authorized in.
///
/// On GraphQL and REST the caller names an authorization tenant in the
/// `x-operator-id` header, while the organization acted on comes from
/// the arguments or the path. Every Library user holds the unscoped
/// `LibraryUserPolicy` in Library's platform tenant, so a policy check
/// evaluated there authorizes anyone for any organization. Actions on an
/// organization, its repositories and everything inside them are
/// therefore always evaluated in that organization's own tenant:
///
/// - a header naming the organization itself is kept as it is;
/// - a header naming the platform tenant (what the v1 and v2 clients send
///   by default), or no header at all, is moved to the organization;
/// - a header naming any other organization is rejected, so a policy
///   that passes in the caller's own organization never reaches another.
pub(crate) fn authorization_scope_for_org(
    scope: &dyn MultiTenancyAction,
    organization_id: &TenantId,
) -> errors::Result<MultiTenancy> {
    match scope.operator_id() {
        Some(operator)
            if &operator != organization_id
                && operator != *LIBRARY_TENANT =>
        {
            Err(errors::Error::forbidden(
                "The organization does not match the authorization tenant",
            ))
        }
        // The platform is carried over as it is, absence included. A
        // scope without one is how an organization adopted from another
        // platform is addressed (`LibraryOrg::for_organization`): tachyon
        // resolves the platform from the operator. Filling in Library's
        // platform there made every check on such an organization fail
        // with 403 "Upstream authorization rejected".
        _ => Ok(MultiTenancy::new(
            scope.platform_id(),
            Some(organization_id.clone()),
        )),
    }
}

/// Whether the caller named Library's platform tenant as the
/// authorization tenant. Repository grants made through the v1 client
/// were attached there with a repository resource scope.
pub(crate) fn header_names_platform_tenant(
    scope: &dyn MultiTenancyAction,
) -> bool {
    scope.operator_id().as_ref() == Some(&*LIBRARY_TENANT)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tenant(suffix: char) -> TenantId {
        format!("tn_01hjryxysgey07h5jz5wagqj0{suffix}")
            .parse()
            .unwrap()
    }

    #[test]
    fn another_organization_tenant_is_rejected() {
        let scope = MultiTenancy::new(None, Some(tenant('a')));
        let err =
            authorization_scope_for_org(&scope, &tenant('b')).unwrap_err();
        assert!(matches!(err, errors::Error::Forbidden { .. }), "{err:?}");
    }

    #[test]
    fn the_target_organization_tenant_is_kept() {
        let scope = MultiTenancy::new(Some(tenant('p')), Some(tenant('b')));
        let scoped =
            authorization_scope_for_org(&scope, &tenant('b')).unwrap();
        assert_eq!(scoped.operator_id(), Some(tenant('b')));
        assert_eq!(scoped.platform_id(), Some(tenant('p')));
    }

    #[test]
    fn a_scope_without_an_operator_moves_to_the_organization() {
        let scope = MultiTenancy::new(None, None);
        let scoped =
            authorization_scope_for_org(&scope, &tenant('b')).unwrap();
        assert_eq!(scoped.operator_id(), Some(tenant('b')));
        assert_eq!(scoped.platform_id(), None);
    }

    /// An organization adopted from another platform is addressed with no
    /// platform, so tachyon resolves it from the operator. Library's
    /// platform there is refused upstream.
    #[test]
    fn an_organization_on_another_platform_keeps_no_platform() {
        let scope = MultiTenancy::new(None, Some(tenant('b')));
        let scoped =
            authorization_scope_for_org(&scope, &tenant('b')).unwrap();
        assert_eq!(scoped.operator_id(), Some(tenant('b')));
        assert_eq!(scoped.platform_id(), None);
    }

    #[test]
    fn the_platform_tenant_moves_to_the_organization() {
        let scope = MultiTenancy::new(
            Some(LIBRARY_TENANT.clone()),
            Some(LIBRARY_TENANT.clone()),
        );
        let scoped =
            authorization_scope_for_org(&scope, &tenant('b')).unwrap();
        assert_eq!(scoped.operator_id(), Some(tenant('b')));
        assert!(header_names_platform_tenant(&scope));
        assert!(!header_names_platform_tenant(&scoped));
    }
}
