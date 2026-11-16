use std::fmt::Debug;
use std::str::FromStr;

use derive_new::new;
use tachyon_sdk::auth::MultiTenancyAction;
use value_object::{Identifier, OperatorId, PlatformId, TenantId};

use crate::domain::LIBRARY_TENANT;

/// LibraryOrg is a specialized implementation of MultiTenancyAction for library-api
///
/// Unlike the general MultiTenancy struct, LibraryOrg identifies tenants
/// using organization usernames from URL paths. Its platform is Library's
/// own (LIBRARY_TENANT), except for an organization adopted from another
/// platform, which carries no platform at all (see `on_other_platform`).
#[derive(Debug, Clone, Default, new)]
pub struct LibraryOrg {
    /// The organization username from the URL path, stored as operator_id
    operator: Option<OperatorId>,
    /// The original organization username string
    org_username: Option<String>,
    /// The organization's tenant lives on a platform other than Library's.
    #[new(default)]
    other_platform: bool,
}

impl LibraryOrg {
    /// Create a new LibraryOrg with the given organization username
    pub fn with_org(org_username: String) -> Self {
        // Try to convert the username to an OperatorAlias
        // For simplicity, we'll just create a new TenantId (which is the same as OperatorId)
        let operator = match Identifier::from_str(&org_username) {
            Ok(_) => Some(TenantId::default()),
            Err(_) => None,
        };

        Self {
            operator,
            org_username: Some(org_username),
            other_platform: false,
        }
    }

    /// Create a new LibraryOrg with the given organization username and operator ID
    pub fn with_org_and_operator(
        org_username: String,
        operator_id: OperatorId,
    ) -> Self {
        Self {
            operator: Some(operator_id),
            org_username: Some(org_username),
            other_platform: false,
        }
    }

    /// Get the organization username
    pub fn org_username(&self) -> Option<&String> {
        self.org_username.as_ref()
    }

    /// Mark the organization as adopted from another platform.
    ///
    /// An organization `seedLibraryTenant` adopted from tachyon keeps the
    /// platform it already had (for example the Quantum Box tenant on the
    /// TACHYON platform). tachyon-api rejects a request whose
    /// `x-platform-id` is not the operator's own platform, so such an
    /// organization must not claim Library's: it reports no platform and
    /// tachyon-api resolves the scope from the operator alone.
    pub fn on_other_platform(mut self) -> Self {
        self.other_platform = true;
        self
    }
}

impl MultiTenancyAction for LibraryOrg {
    fn platform_id(&self) -> Option<PlatformId> {
        if self.other_platform {
            return None;
        }
        Some(LIBRARY_TENANT.clone())
    }

    fn operator_id(&self) -> Option<OperatorId> {
        // Return the operator ID if we have one
        self.operator.clone()
    }

    /// Get the operator ID for the current request
    ///
    /// For library-api, we return the operator ID if available,
    /// otherwise we return the LIBRARY_TENANT as a fallback
    fn get_operator_id(&self) -> errors::Result<OperatorId> {
        if let Some(operator) = &self.operator {
            Ok(operator.clone())
        } else {
            // Fallback to LIBRARY_TENANT if no operator is available
            Ok(LIBRARY_TENANT.clone())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_library_org_platform_id() {
        let library_org = LibraryOrg::default();
        assert_eq!(library_org.platform_id(), Some(LIBRARY_TENANT.clone()));
    }

    #[test]
    fn test_library_org_operator_id() {
        // Default has no operator
        let library_org = LibraryOrg::default();
        assert_eq!(library_org.operator_id(), None);

        // With org should have an operator ID
        let org_username = "test-org".to_string();
        let library_org = LibraryOrg::with_org(org_username);
        assert!(library_org.operator_id().is_some());
    }

    #[test]
    fn test_library_org_get_operator_id() {
        // Default falls back to LIBRARY_TENANT
        let library_org = LibraryOrg::default();
        assert_eq!(
            library_org.get_operator_id().unwrap(),
            LIBRARY_TENANT.clone()
        );

        // With org returns the operator ID
        let org_username = "test-org".to_string();
        let library_org = LibraryOrg::with_org(org_username);
        assert!(library_org.get_operator_id().is_ok());
        assert_ne!(
            library_org.get_operator_id().unwrap(),
            LIBRARY_TENANT.clone()
        );
    }

    #[test]
    fn an_organization_on_another_platform_claims_no_platform() {
        let operator: OperatorId =
            "tn_01kxz0ytmhnab5vh53011cwctj".parse().unwrap();
        let org = LibraryOrg::with_org_and_operator(
            "quantumbox".to_string(),
            operator.clone(),
        )
        .on_other_platform();
        assert_eq!(org.platform_id(), None);
        assert_eq!(org.get_operator_id().unwrap(), operator);
    }

    #[test]
    fn test_library_org_with_org() {
        let org_username = "test-org".to_string();
        let library_org = LibraryOrg::with_org(org_username.clone());
        assert_eq!(library_org.org_username(), Some(&org_username));
        assert!(library_org.operator.is_some());
    }
}
