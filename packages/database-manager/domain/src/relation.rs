use std::fmt::Debug;

use crate::{DatabaseId, PropertyId};
use derive_getters::Getters;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};
use util::macros::*;
use value_object::*;

def_id!(RelationId, "rl_");

pub const RELATION_DEFINITION_VERSION_V1: RelationDefinitionVersion =
    RelationDefinitionVersion::new_unchecked(1);
pub const RELATION_GENERATION_V1: RelationGeneration =
    RelationGeneration::new_unchecked(1);

/// Version of the persisted RelationDefinition contract.
///
/// A newer version remains readable, but this binary refuses to mutate it.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize,
)]
#[serde(transparent)]
pub struct RelationDefinitionVersion(u16);

impl RelationDefinitionVersion {
    pub fn new(value: u16) -> errors::Result<Self> {
        if value == 0 {
            return Err(errors::Error::invalid(
                "relation definition version must be greater than zero",
            ));
        }
        Ok(Self(value))
    }

    const fn new_unchecked(value: u16) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u16 {
        self.0
    }
}

impl<'de> Deserialize<'de> for RelationDefinitionVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = u16::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

/// Monotonic optimistic-concurrency token for RelationDefinition mutations.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize,
)]
#[serde(transparent)]
pub struct RelationGeneration(u64);

impl RelationGeneration {
    pub fn new(value: u64) -> errors::Result<Self> {
        if value == 0 {
            return Err(errors::Error::invalid(
                "relation generation must be greater than zero",
            ));
        }
        Ok(Self(value))
    }

    const fn new_unchecked(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }

    pub fn next(self) -> errors::Result<Self> {
        self.0.checked_add(1).map(Self).ok_or_else(|| {
            errors::Error::conflict("relation generation overflow")
        })
    }
}

impl<'de> Deserialize<'de> for RelationGeneration {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = u64::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

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
    inverse_property_owned: bool,
    on_target_delete: RelationOnDelete,
    definition_version: RelationDefinitionVersion,
    generation: RelationGeneration,
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
    ) -> errors::Result<Self> {
        Self::restore(
            id,
            tenant_id,
            source_database_id,
            source_property_id,
            target_database_id,
            forward_cardinality,
            reverse_cardinality,
            inverse_property_id,
            false,
            on_target_delete,
            RELATION_DEFINITION_VERSION_V1,
            RELATION_GENERATION_V1,
        )
    }

    /// Restore a persisted definition without silently upgrading its version.
    #[allow(clippy::too_many_arguments)]
    pub fn restore(
        id: &RelationId,
        tenant_id: &TenantId,
        source_database_id: &DatabaseId,
        source_property_id: &PropertyId,
        target_database_id: &DatabaseId,
        forward_cardinality: RelationCardinality,
        reverse_cardinality: RelationCardinality,
        inverse_property_id: Option<&PropertyId>,
        inverse_property_owned: bool,
        on_target_delete: RelationOnDelete,
        definition_version: RelationDefinitionVersion,
        generation: RelationGeneration,
    ) -> errors::Result<Self> {
        Self::validate_inverse(
            source_database_id,
            source_property_id,
            target_database_id,
            inverse_property_id,
            inverse_property_owned,
        )?;
        Ok(Self {
            id: id.clone(),
            tenant_id: tenant_id.clone(),
            source_database_id: source_database_id.clone(),
            source_property_id: source_property_id.clone(),
            target_database_id: target_database_id.clone(),
            forward_cardinality,
            reverse_cardinality,
            inverse_property_id: inverse_property_id.cloned(),
            inverse_property_owned,
            on_target_delete,
            definition_version,
            generation,
        })
    }

    fn validate_inverse(
        source_database_id: &DatabaseId,
        source_property_id: &PropertyId,
        target_database_id: &DatabaseId,
        inverse_property_id: Option<&PropertyId>,
        inverse_property_owned: bool,
    ) -> errors::Result<()> {
        if inverse_property_owned && inverse_property_id.is_none() {
            return Err(errors::Error::invalid(
                "an owned inverse Property id is required",
            ));
        }
        if source_database_id == target_database_id
            && inverse_property_id == Some(source_property_id)
        {
            return Err(errors::Error::invalid(
                "a self Relation inverse must be a distinct Property",
            ));
        }
        Ok(())
    }

    pub fn ensure_writable(&self) -> errors::Result<()> {
        if self.definition_version != RELATION_DEFINITION_VERSION_V1 {
            return Err(errors::Error::invalid(format!(
                "RelationDefinition version {} is read-only in this binary",
                self.definition_version.get()
            )));
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn reconfigure(
        &self,
        forward_cardinality: RelationCardinality,
        reverse_cardinality: RelationCardinality,
        inverse_property_id: Option<&PropertyId>,
        inverse_property_owned: bool,
        on_target_delete: RelationOnDelete,
    ) -> errors::Result<Self> {
        self.ensure_writable()?;
        Self::validate_inverse(
            &self.source_database_id,
            &self.source_property_id,
            &self.target_database_id,
            inverse_property_id,
            inverse_property_owned,
        )?;
        Ok(Self {
            forward_cardinality,
            reverse_cardinality,
            inverse_property_id: inverse_property_id.cloned(),
            inverse_property_owned,
            on_target_delete,
            generation: self.generation.next()?,
            ..self.clone()
        })
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
        .expect("a legacy RelationDefinition has no inverse invariant")
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
    async fn find_by_id(
        &self,
        tenant_id: &TenantId,
        source_database_id: &DatabaseId,
        relation_id: &RelationId,
    ) -> errors::Result<Option<RelationDefinition>>;

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
        )
        .expect("valid self Relation");

        assert_eq!(definition.source_database_id(), &database_id);
        assert_eq!(definition.target_database_id(), &database_id);
        assert_eq!(definition.source_property_id(), &property_id);
        assert_eq!(
            definition.inverse_property_id(),
            &Some(inverse_property_id)
        );
    }

    #[test]
    fn self_relation_rejects_the_source_property_as_its_inverse() {
        let database_id = DatabaseId::default();
        let property_id = PropertyId::default();
        let error = RelationDefinition::new(
            &RelationId::default(),
            &TenantId::default(),
            &database_id,
            &property_id,
            &database_id,
            RelationCardinality::Many,
            RelationCardinality::Many,
            Some(&property_id),
            RelationOnDelete::Restrict,
        )
        .expect_err("a self inverse must be a distinct Property");
        assert!(error.to_string().contains("distinct Property"));
    }
}
