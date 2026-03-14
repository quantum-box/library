use std::fmt::Debug;

use crate::{DatabaseId, PropertyId};
use derive_getters::Getters;
use util::macros::*;
use value_object::*;

def_id!(RelationId, "rl_");

#[derive(Debug, Clone, Getters)]
pub struct Relation {
    id: RelationId,
    tenant_id: TenantId,

    database_id: DatabaseId,
    property_id: PropertyId,
    relation_id: usize,
    target_database_id: DatabaseId,
}

impl Relation {
    pub fn new(
        id: &RelationId,
        tenant_id: &TenantId,
        database_id: &DatabaseId,
        property_id: &PropertyId,
        relation_id: usize,
        target_database_id: &DatabaseId,
    ) -> Self {
        Self {
            id: id.clone(),
            tenant_id: tenant_id.clone(),
            database_id: database_id.clone(),
            property_id: property_id.clone(),
            relation_id,
            target_database_id: target_database_id.clone(),
        }
    }
}

#[async_trait::async_trait]
pub trait RelationRepository: Debug + Send + Sync {
    async fn insert(&self, entity: &Relation) -> errors::Result<()>;

    async fn find_all_by_database(
        &self,
        database_id: &DatabaseId,
        tenant_id: &TenantId,
    ) -> errors::Result<Vec<Relation>>;
}
