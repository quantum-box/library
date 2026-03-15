use super::domain::{ServiceAccount, User};
use super::types::{OperatorId, PlatformId, TenantId, UserId};
use std::fmt::Debug;

// ─────────────── ExecutorAction trait ──────────────────

/// Trait representing an authenticated executor
/// (user, service account, or system).
#[cfg_attr(feature = "test", mockall::automock)]
pub trait ExecutorAction: Debug + Send + Sync + 'static {
    fn get_id(&self) -> &str;
    fn has_tenant_id(&self, tenant_id: &TenantId) -> bool;
    fn is_system_user(&self) -> bool;
    fn is_user(&self) -> bool;
    fn is_service_account(&self) -> bool;
    fn is_none(&self) -> bool;
    fn get_user_id(&self) -> errors::Result<UserId> {
        if self.get_id().is_empty() {
            Err(errors::Error::business_logic("User id is empty"))
        } else {
            Ok(self.get_id().parse()?)
        }
    }
}

// ──────────── MultiTenancyAction trait ─────────────────

/// Trait representing a multi-tenancy context for a
/// request.
#[cfg_attr(feature = "test", mockall::automock)]
pub trait MultiTenancyAction: Debug + Send + Sync + 'static {
    fn platform_id(&self) -> Option<PlatformId>;
    fn operator_id(&self) -> Option<OperatorId>;
    fn get_operator_id(&self) -> errors::Result<OperatorId>;
}

// ───────────────── Executor enum ───────────────────────

/// Concrete executor types.
#[derive(Debug, Clone)]
pub enum Executor {
    SystemUser,
    User(Box<User>),
    ServiceAccount(Box<ServiceAccount>),
    None,
}

impl ExecutorAction for Executor {
    fn get_id(&self) -> &str {
        match self {
            Self::SystemUser => "system",
            Self::User(u) => u.id.as_str(),
            Self::ServiceAccount(sa) => sa.id.as_str(),
            Self::None => "",
        }
    }

    fn has_tenant_id(&self, tenant_id: &TenantId) -> bool {
        match self {
            Self::SystemUser => true,
            Self::User(u) => u.tenants.iter().any(|t| t == tenant_id),
            Self::ServiceAccount(sa) => sa.tenant_id == *tenant_id,
            Self::None => false,
        }
    }

    fn is_system_user(&self) -> bool {
        matches!(self, Self::SystemUser)
    }

    fn is_user(&self) -> bool {
        matches!(self, Self::User(_))
    }

    fn is_service_account(&self) -> bool {
        matches!(self, Self::ServiceAccount(_))
    }

    fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }
}

// ──────────────── MultiTenancy struct ──────────────────

/// Concrete multi-tenancy context.
#[derive(Debug, Clone)]
pub struct MultiTenancy {
    platform: Option<PlatformId>,
    operator: Option<OperatorId>,
}

impl Default for MultiTenancy {
    fn default() -> Self {
        Self {
            platform: None,
            operator: None,
        }
    }
}

impl MultiTenancy {
    pub fn new(
        platform: Option<PlatformId>,
        operator: Option<OperatorId>,
    ) -> Self {
        Self { platform, operator }
    }

    pub fn new_operator(operator: OperatorId) -> Self {
        Self {
            platform: None,
            operator: Some(operator),
        }
    }

    pub fn new_platform(platform: PlatformId) -> Self {
        Self {
            platform: Some(platform),
            operator: None,
        }
    }
}

impl MultiTenancyAction for MultiTenancy {
    fn platform_id(&self) -> Option<PlatformId> {
        self.platform.clone()
    }

    fn operator_id(&self) -> Option<OperatorId> {
        self.operator.clone()
    }

    fn get_operator_id(&self) -> errors::Result<OperatorId> {
        self.operator.clone().ok_or_else(|| {
            errors::Error::business_logic("Operator ID is required")
        })
    }
}
