use std::fmt::Debug;

use crate::{DatabaseId, PropertyId};
use derive_getters::Getters;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};
use util::macros::*;
use value_object::*;

def_id!(RelationId, "rl_");

/// Legacy projection retained while existing callers migrate to
/// [`RelationDefinition`]. New schema mutations persist a RelationDefinition
/// directly in the Property-schema unit of work.
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

/// Maximum number of records allowed at one side of a Relation.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Display,
    EnumString,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum RelationCardinality {
    One,
    #[default]
    Many,
}

/// Behavior when a record referenced by a Relation is deleted.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Display,
    EnumString,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum RelationOnDelete {
    #[default]
    Restrict,
    Nullify,
}

/// Canonical definition of a Relation Property.
///
/// The source Property fixes the forward orientation. An optional inverse
/// Property points back to this same definition; no mirrored definition is
/// created. `forward_cardinality` limits targets per source record, while
/// `reverse_cardinality` limits sources per target record. RelationEdge
/// persistence is deliberately a separate concern.
#[derive(Debug, Clone, PartialEq, Eq, Getters)]
pub struct RelationDefinition {
    id: RelationId,
    tenant_id: TenantId,
    source_database_id: DatabaseId,
    source_property_id: PropertyId,
    target_database_id: DatabaseId,
    forward_cardinality: RelationCardinality,
    reverse_cardinality: RelationCardinality,
    inverse_property_id: Option<PropertyId>,
    on_target_delete: RelationOnDelete,
}

impl RelationDefinition {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: &RelationId,
        tenant_id: &TenantId,
        source_database_id: &DatabaseId,
        source_property_id: &PropertyId,
        target_database_id: &DatabaseId,
        forward_cardinality: RelationCardinality,
        reverse_cardinality: RelationCardinality,
        inverse_property_id: Option<&PropertyId>,
        on_target_delete: RelationOnDelete,
    ) -> Self {
        Self {
            id: id.clone(),
            tenant_id: tenant_id.clone(),
            source_database_id: source_database_id.clone(),
            source_property_id: source_property_id.clone(),
            target_database_id: target_database_id.clone(),
            forward_cardinality,
            reverse_cardinality,
            inverse_property_id: inverse_property_id.cloned(),
            on_target_delete,
        }
    }

    /// Compatibility semantics for definitions created from the legacy
    /// `TypeRelation { database_id }` configuration.
    pub fn legacy_default(
        id: &RelationId,
        tenant_id: &TenantId,
        source_database_id: &DatabaseId,
        source_property_id: &PropertyId,
        target_database_id: &DatabaseId,
    ) -> Self {
        Self::new(
            id,
            tenant_id,
            source_database_id,
            source_property_id,
            target_database_id,
            RelationCardinality::Many,
            RelationCardinality::Many,
            None,
            RelationOnDelete::Restrict,
        )
    }
}

/// Read port for Relation definitions.
///
/// Definition writes stay inside the Property-schema unit of work so a field
/// can never be committed without its Relation definition.
#[async_trait::async_trait]
pub trait RelationDefinitionRepository:
    Debug + Send + Sync + 'static
{
    async fn find_by_source_property(
        &self,
        tenant_id: &TenantId,
        source_database_id: &DatabaseId,
        source_property_id: &PropertyId,
    ) -> errors::Result<Option<RelationDefinition>>;

    async fn find_all_by_source_database(
        &self,
        tenant_id: &TenantId,
        source_database_id: &DatabaseId,
    ) -> errors::Result<Vec<RelationDefinition>>;

    async fn find_all_by_target_database(
        &self,
        tenant_id: &TenantId,
        target_database_id: &DatabaseId,
    ) -> errors::Result<Vec<RelationDefinition>>;
}

/// Compatibility read port for the pre-RelationDefinition projection.
#[async_trait::async_trait]
pub trait RelationRepository: Debug + Send + Sync {
    async fn find_all_by_database(
        &self,
        database_id: &DatabaseId,
        tenant_id: &TenantId,
    ) -> errors::Result<Vec<Relation>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_definitions_have_explicit_safe_defaults() {
        let definition = RelationDefinition::legacy_default(
            &RelationId::default(),
            &TenantId::default(),
            &DatabaseId::default(),
            &PropertyId::default(),
            &DatabaseId::default(),
        );

        assert_eq!(
            *definition.forward_cardinality(),
            RelationCardinality::Many
        );
        assert_eq!(
            *definition.reverse_cardinality(),
            RelationCardinality::Many
        );
        assert_eq!(
            *definition.on_target_delete(),
            RelationOnDelete::Restrict
        );
        assert!(definition.inverse_property_id().is_none());
    }

    #[test]
    fn self_relations_keep_one_canonical_orientation() {
        let database_id = DatabaseId::default();
        let property_id = PropertyId::default();
        let inverse_property_id = PropertyId::default();
        let definition = RelationDefinition::new(
            &RelationId::default(),
            &TenantId::default(),
            &database_id,
            &property_id,
            &database_id,
            RelationCardinality::One,
            RelationCardinality::Many,
            Some(&inverse_property_id),
            RelationOnDelete::Nullify,
        );

        assert_eq!(definition.source_database_id(), &database_id);
        assert_eq!(definition.target_database_id(), &database_id);
        assert_eq!(definition.source_property_id(), &property_id);
        assert_eq!(
            definition.inverse_property_id(),
            &Some(inverse_property_id)
        );
    }
}
