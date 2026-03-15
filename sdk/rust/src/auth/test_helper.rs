use super::executor::{Executor, MultiTenancy};
use super::types::TenantId;

/// Default test tenant ID used across test fixtures.
pub const TEST_TENANT_ID: &str = "tn_01hjryxysgey07h5jz5wagqj0m";

pub fn create_test_executor() -> Executor {
    Executor::SystemUser
}

pub fn create_test_multi_tenancy() -> MultiTenancy {
    let tenant_id = TenantId::new(TEST_TENANT_ID)
        .expect("test tenant ID should be valid");
    MultiTenancy::new_operator(tenant_id)
}
