use super::executor::{Executor, MultiTenancy};
use super::types::TenantId;

pub fn create_test_executor() -> Executor {
    Executor::SystemUser
}

pub fn create_test_multi_tenancy() -> MultiTenancy {
    MultiTenancy::new_operator(TenantId::default())
}
