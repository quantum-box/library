mod organization;
pub mod policy;
pub mod repo;
mod role;

pub use organization::*;
pub use policy::*;
pub use repo::*;
pub use role::*;

use once_cell::sync::Lazy;
use value_object::TenantId;

pub static LIBRARY_TENANT: Lazy<TenantId> = Lazy::new(|| {
    let tenant_id = std::env::var("LIBRARY_TENANT_ID")
        .unwrap_or("tn_01j702qf86pc2j35s0kv0gv3gy".to_string());
    // "tn_01HK2Y0K3QZZVMT5G66B6Y0E75"
    tenant_id.parse().unwrap()
});
