use super::*;
use serde::Serialize;
use std::fmt::Debug;
use util::macros::*;
use value_object::*;

#[derive(Getters, Debug, Clone, Eq, PartialEq, Serialize)]
pub struct Database {
    id: DatabaseId,
    tenant_id: TenantId,
    name: Text,
}

impl Database {
    pub fn new(id: &DatabaseId, tenant_id: &TenantId, name: &str) -> Self {
        let name = if name.is_empty() {
            "Untitled".to_string()
        } else {
            name.to_string()
        };
        Self {
            id: id.clone(),
            tenant_id: tenant_id.clone(),
            name: name.parse().unwrap(),
        }
    }

    pub fn change_name(&mut self, name: &Text) {
        self.name = name.clone();
    }
}

def_id!(DatabaseId, "db_");
