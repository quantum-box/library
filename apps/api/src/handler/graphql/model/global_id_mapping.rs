use crate::domain;
use async_graphql::SimpleObject;
use chrono::{DateTime, Utc};

#[derive(SimpleObject, Debug, Clone)]
pub struct GlobalIdMapping {
    pub id: String,
    pub tenant_id: String,
    pub global_id: String,
    pub system: String,
    pub system_code: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<domain::GlobalIdMapping> for GlobalIdMapping {
    fn from(m: domain::GlobalIdMapping) -> Self {
        Self {
            id: m.id().to_string(),
            tenant_id: m.tenant_id().to_string(),
            global_id: m.global_id().to_string(),
            system: m.system().to_string(),
            system_code: m.system_code().to_string(),
            name: m.name().to_string(),
            created_at: *m.created_at(),
            updated_at: *m.updated_at(),
        }
    }
}
