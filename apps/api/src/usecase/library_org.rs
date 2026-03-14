use std::fmt::Debug;
use std::str::FromStr;

use derive_new::new;
use tachyon_sdk::auth::MultiTenancyAction;
use value_object::{Identifier, OperatorId, PlatformId, TenantId};

use crate::domain::LIBRARY_TENANT;

/// LibraryOrg is a specialized implementation of MultiTenancyAction for library-api
///
/// Unlike the general MultiTenancy struct, LibraryOrg has a fixed platform ID (LIBRARY_TENANT)
/// and identifies tenants using organization usernames from URL paths.
#[derive(Debug, Clone, Default, new)]
pub struct LibraryOrg {
    /// The organization username from the URL path, stored as operator_id
    operator: Option<OperatorId>,
    /// The original organization username string
    org_username: Option<String>,
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
        }
    }

    /// Get the organization username
    pub fn org_username(&self) -> Option<&String> {
        self.org_username.as_ref()
    }
}

impl MultiTenancyAction for LibraryOrg {
    fn platform_id(&self) -> Option<PlatformId> {
        // Library API always uses the fixed LIBRARY_TENANT
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
    fn test_library_org_with_org() {
        let org_username = "test-org".to_string();
        let library_org = LibraryOrg::with_org(org_username.clone());
        assert_eq!(library_org.org_username(), Some(&org_username));
        assert!(library_org.operator.is_some());
    }
}
