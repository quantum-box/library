use std::fmt::Debug;

use derive_getters::Getters;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};
use util::macros::*;
use value_object::TenantId;

use crate::{
    DatabaseId, IndexCapabilities, PropertyDefinition, PropertyId,
    RelationDefinition, RelationId, ResolvedPropertyConfig,
};

def_id!(IndexDefinitionId, "ix_");

pub const INDEX_DEFINITION_VERSION_V1: IndexDefinitionVersion =
    IndexDefinitionVersion::new_unchecked(1);
pub const INDEX_GENERATION_V1: IndexGeneration =
    IndexGeneration::new_unchecked(1);

/// Physical strategy requested for one logical Property or Relation index.
///
/// `Range` implies equality lookup and ordering for scalar handlers. `None`
/// is an explicit disabled declaration; it is not inferred from the legacy
/// `fields.is_indexed` flag.
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
pub enum IndexPolicy {
    #[default]
    None,
    Exact,
    Range,
    FullText,
}

/// Lifecycle of the rebuildable physical projection for this definition.
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
pub enum IndexProjectionState {
    #[default]
    Disabled,
    Pending,
    Building,
    Ready,
    Failed,
}

impl IndexProjectionState {
    fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Pending, Self::Building)
                | (Self::Failed, Self::Building)
                | (Self::Building, Self::Ready)
                | (Self::Building, Self::Failed)
        )
    }
}

/// Version of the IndexDefinition storage/application contract.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize,
)]
#[serde(transparent)]
pub struct IndexDefinitionVersion(u16);

impl IndexDefinitionVersion {
    pub fn new(value: u16) -> errors::Result<Self> {
        if value == 0 {
            return Err(errors::Error::invalid(
                "index definition version must be greater than zero",
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

impl<'de> Deserialize<'de> for IndexDefinitionVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = u16::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

/// Monotonic optimistic-concurrency token for definition mutations.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize,
)]
#[serde(transparent)]
pub struct IndexGeneration(u64);

impl IndexGeneration {
    pub fn new(value: u64) -> errors::Result<Self> {
        if value == 0 {
            return Err(errors::Error::invalid(
                "index generation must be greater than zero",
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
            errors::Error::conflict("index generation overflow")
        })
    }
}

impl<'de> Deserialize<'de> for IndexGeneration {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = u64::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

/// Stable target identity. Property targets describe typed value indexes;
/// Relation targets reserve the reverse-lookup projection control plane.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IndexTarget {
    Property(PropertyId),
    Relation(RelationId),
}

/// Declarative index aggregate owned by the Database bounded context.
#[derive(Debug, Clone, PartialEq, Eq, Getters)]
pub struct IndexDefinition {
    id: IndexDefinitionId,
    tenant_id: TenantId,
    database_id: DatabaseId,
    target: IndexTarget,
    policy: IndexPolicy,
    unique: bool,
    definition_version: IndexDefinitionVersion,
    generation: IndexGeneration,
    projection_state: IndexProjectionState,
}

impl IndexDefinition {
    pub fn declare_for_property(
        id: &IndexDefinitionId,
        tenant_id: &TenantId,
        database_id: &DatabaseId,
        property: &PropertyDefinition,
        policy: IndexPolicy,
        unique: bool,
    ) -> errors::Result<Self> {
        Self::ensure_property_scope(tenant_id, database_id, property)?;
        Self::validate_property_policy(property, policy, unique)?;
        Self::new_v1(
            id,
            tenant_id,
            database_id,
            IndexTarget::Property(property.id().clone()),
            policy,
            unique,
        )
    }

    pub fn declare_for_relation(
        id: &IndexDefinitionId,
        tenant_id: &TenantId,
        database_id: &DatabaseId,
        relation: &RelationDefinition,
        policy: IndexPolicy,
        unique: bool,
    ) -> errors::Result<Self> {
        Self::ensure_relation_scope(tenant_id, database_id, relation)?;
        Self::validate_relation_policy(policy, unique)?;
        Self::new_v1(
            id,
            tenant_id,
            database_id,
            IndexTarget::Relation(relation.id().clone()),
            policy,
            unique,
        )
    }

    fn new_v1(
        id: &IndexDefinitionId,
        tenant_id: &TenantId,
        database_id: &DatabaseId,
        target: IndexTarget,
        policy: IndexPolicy,
        unique: bool,
    ) -> errors::Result<Self> {
        Self::restore(
            id,
            tenant_id,
            database_id,
            target,
            policy,
            unique,
            INDEX_DEFINITION_VERSION_V1,
            INDEX_GENERATION_V1,
            Self::initial_projection_state(policy),
        )
    }

    /// Restore a persisted definition without silently upgrading its version.
    #[allow(clippy::too_many_arguments)]
    pub fn restore(
        id: &IndexDefinitionId,
        tenant_id: &TenantId,
        database_id: &DatabaseId,
        target: IndexTarget,
        policy: IndexPolicy,
        unique: bool,
        definition_version: IndexDefinitionVersion,
        generation: IndexGeneration,
        projection_state: IndexProjectionState,
    ) -> errors::Result<Self> {
        match &target {
            IndexTarget::Property(_) => {
                // Property Type capability validation belongs to declaration
                // and reconfiguration, where the canonical Property is
                // available. Storage restoration can still enforce the
                // target-independent unique-policy invariant.
                Self::validate_general_policy(policy, unique)?;
            }
            IndexTarget::Relation(_) => {
                Self::validate_relation_policy(policy, unique)?;
            }
        }
        Self::validate_projection_state(policy, projection_state)?;
        Ok(Self {
            id: id.clone(),
            tenant_id: tenant_id.clone(),
            database_id: database_id.clone(),
            target,
            policy,
            unique,
            definition_version,
            generation,
            projection_state,
        })
    }

    pub fn reconfigure_for_property(
        &self,
        property: &PropertyDefinition,
        policy: IndexPolicy,
        unique: bool,
    ) -> errors::Result<Self> {
        self.ensure_writable()?;
        Self::ensure_property_scope(
            &self.tenant_id,
            &self.database_id,
            property,
        )?;
        if self.target != IndexTarget::Property(property.id().clone()) {
            return Err(errors::Error::invalid(
                "index definition target does not match Property",
            ));
        }
        Self::validate_property_policy(property, policy, unique)?;
        self.reconfigured(policy, unique)
    }

    pub fn reconfigure_for_relation(
        &self,
        relation: &RelationDefinition,
        policy: IndexPolicy,
        unique: bool,
    ) -> errors::Result<Self> {
        self.ensure_writable()?;
        Self::ensure_relation_scope(
            &self.tenant_id,
            &self.database_id,
            relation,
        )?;
        if self.target != IndexTarget::Relation(relation.id().clone()) {
            return Err(errors::Error::invalid(
                "index definition target does not match RelationDefinition",
            ));
        }
        Self::validate_relation_policy(policy, unique)?;
        self.reconfigured(policy, unique)
    }

    /// Revalidate the currently declared Property target before a projection
    /// worker advances lifecycle state.
    ///
    /// Capability decisions must not outlive the canonical PropertyDefinition
    /// that authorized them. In particular, an opaque, malformed, or
    /// legacy-mismatched definition cannot keep moving an old projection
    /// toward READY.
    pub fn validate_current_property_target(
        &self,
        property: &PropertyDefinition,
    ) -> errors::Result<()> {
        self.ensure_writable()?;
        Self::ensure_property_scope(
            &self.tenant_id,
            &self.database_id,
            property,
        )?;
        if self.target != IndexTarget::Property(property.id().clone()) {
            return Err(errors::Error::invalid(
                "index definition target does not match Property",
            ));
        }
        Self::validate_property_policy(property, self.policy, self.unique)
    }

    /// Revalidate the currently declared Relation target before a projection
    /// worker advances lifecycle state.
    pub fn validate_current_relation_target(
        &self,
        relation: &RelationDefinition,
    ) -> errors::Result<()> {
        self.ensure_writable()?;
        Self::ensure_relation_scope(
            &self.tenant_id,
            &self.database_id,
            relation,
        )?;
        if self.target != IndexTarget::Relation(relation.id().clone()) {
            return Err(errors::Error::invalid(
                "index definition target does not match RelationDefinition",
            ));
        }
        Self::validate_relation_policy(self.policy, self.unique)
    }

    fn reconfigured(
        &self,
        policy: IndexPolicy,
        unique: bool,
    ) -> errors::Result<Self> {
        if self.policy == policy && self.unique == unique {
            return Err(errors::Error::conflict(
                "index definition update is a no-op",
            ));
        }
        Ok(Self {
            policy,
            unique,
            generation: self.generation.next()?,
            projection_state: Self::initial_projection_state(policy),
            ..self.clone()
        })
    }

    pub fn transition_projection(
        &self,
        next: IndexProjectionState,
    ) -> errors::Result<Self> {
        self.ensure_writable()?;
        if !self.projection_state.can_transition_to(next) {
            return Err(errors::Error::conflict(format!(
                "invalid index projection transition {} -> {next}",
                self.projection_state
            )));
        }
        let generation = if next == IndexProjectionState::Building {
            self.generation.next()?
        } else {
            self.generation
        };
        Ok(Self {
            generation,
            projection_state: next,
            ..self.clone()
        })
    }

    pub fn ensure_writable(&self) -> errors::Result<()> {
        if self.definition_version != INDEX_DEFINITION_VERSION_V1 {
            return Err(errors::Error::not_supported(format!(
                "index definition version {} is read-only",
                self.definition_version.get()
            )));
        }
        Ok(())
    }

    fn ensure_property_scope(
        tenant_id: &TenantId,
        database_id: &DatabaseId,
        property: &PropertyDefinition,
    ) -> errors::Result<()> {
        if property.tenant_id() != tenant_id
            || property.database_id() != database_id
        {
            return Err(errors::Error::not_found("resource not found"));
        }
        Ok(())
    }

    fn ensure_relation_scope(
        tenant_id: &TenantId,
        database_id: &DatabaseId,
        relation: &RelationDefinition,
    ) -> errors::Result<()> {
        if relation.tenant_id() != tenant_id
            || relation.source_database_id() != database_id
        {
            return Err(errors::Error::not_found("resource not found"));
        }
        Ok(())
    }

    fn validate_property_policy(
        property: &PropertyDefinition,
        policy: IndexPolicy,
        unique: bool,
    ) -> errors::Result<()> {
        Self::validate_general_policy(policy, unique)?;
        let ResolvedPropertyConfig::Known(config) = property.config()
        else {
            return property.config().ensure_writable();
        };
        config.handler().validate_config(config)?;
        let capabilities = config.handler().index_capabilities();
        Self::validate_capabilities(capabilities, policy, unique)
    }

    fn validate_capabilities(
        capabilities: IndexCapabilities,
        policy: IndexPolicy,
        unique: bool,
    ) -> errors::Result<()> {
        let supported = match policy {
            IndexPolicy::None => true,
            IndexPolicy::Exact => capabilities.exact,
            IndexPolicy::Range => capabilities.range,
            IndexPolicy::FullText => capabilities.full_text,
        };
        if !supported {
            return Err(errors::Error::invalid(format!(
                "Property Type does not support {policy} indexing"
            )));
        }
        if unique && !capabilities.unique {
            return Err(errors::Error::invalid(
                "Property Type does not support unique indexing",
            ));
        }
        Ok(())
    }

    fn validate_relation_policy(
        policy: IndexPolicy,
        unique: bool,
    ) -> errors::Result<()> {
        Self::validate_general_policy(policy, unique)?;
        if !matches!(policy, IndexPolicy::None | IndexPolicy::Exact) {
            return Err(errors::Error::invalid(
                "RelationDefinition indexes support only NONE or EXACT",
            ));
        }
        if unique {
            return Err(errors::Error::invalid(
                "RelationDefinition reverse indexes cannot be unique",
            ));
        }
        Ok(())
    }

    fn validate_general_policy(
        policy: IndexPolicy,
        unique: bool,
    ) -> errors::Result<()> {
        if unique
            && !matches!(policy, IndexPolicy::Exact | IndexPolicy::Range)
        {
            return Err(errors::Error::invalid(
                "unique indexes require EXACT or RANGE policy",
            ));
        }
        Ok(())
    }

    fn initial_projection_state(
        policy: IndexPolicy,
    ) -> IndexProjectionState {
        match policy {
            IndexPolicy::None => IndexProjectionState::Disabled,
            _ => IndexProjectionState::Pending,
        }
    }

    fn validate_projection_state(
        policy: IndexPolicy,
        state: IndexProjectionState,
    ) -> errors::Result<()> {
        let valid = match policy {
            IndexPolicy::None => state == IndexProjectionState::Disabled,
            _ => state != IndexProjectionState::Disabled,
        };
        if !valid {
            return Err(errors::Error::invalid(
                "index policy and projection state are inconsistent",
            ));
        }
        Ok(())
    }
}

/// Scoped read/write output port for the IndexDefinition control plane.
#[async_trait::async_trait]
pub trait IndexDefinitionRepository: Debug + Send + Sync + 'static {
    async fn insert(
        &self,
        definition: &IndexDefinition,
    ) -> errors::Result<()>;

    async fn replace_if_generation(
        &self,
        definition: &IndexDefinition,
        expected_generation: IndexGeneration,
    ) -> errors::Result<()>;

    async fn transition_projection_if_generation(
        &self,
        definition: &IndexDefinition,
        expected_generation: IndexGeneration,
        expected_state: IndexProjectionState,
    ) -> errors::Result<()>;

    async fn find_by_id(
        &self,
        tenant_id: &TenantId,
        database_id: &DatabaseId,
        index_definition_id: &IndexDefinitionId,
    ) -> errors::Result<Option<IndexDefinition>>;

    async fn find_by_target(
        &self,
        tenant_id: &TenantId,
        database_id: &DatabaseId,
        target: &IndexTarget,
    ) -> errors::Result<Option<IndexDefinition>>;

    async fn find_all_by_database(
        &self,
        tenant_id: &TenantId,
        database_id: &DatabaseId,
    ) -> errors::Result<Vec<IndexDefinition>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        OpaquePropertyConfig, Property, PropertyType, PropertyTypeKey,
        PropertyTypeRef, PropertyTypeVersion, RelationCardinality,
        RelationOnDelete, TypeRelation,
    };

    fn property(
        tenant_id: &TenantId,
        database_id: &DatabaseId,
        property_type: PropertyType,
    ) -> PropertyDefinition {
        PropertyDefinition::from_property(&Property::new(
            &PropertyId::default(),
            tenant_id,
            database_id,
            "value",
            &property_type,
            false,
            0,
        ))
    }

    fn relation(
        tenant_id: &TenantId,
        database_id: &DatabaseId,
    ) -> RelationDefinition {
        RelationDefinition::new(
            &RelationId::default(),
            tenant_id,
            database_id,
            &PropertyId::default(),
            &DatabaseId::default(),
            RelationCardinality::Many,
            RelationCardinality::Many,
            None,
            RelationOnDelete::Restrict,
        )
    }

    #[test]
    fn property_capabilities_control_policy_and_uniqueness() {
        let tenant_id = TenantId::default();
        let database_id = DatabaseId::default();
        let string =
            property(&tenant_id, &database_id, PropertyType::String);
        assert!(
            IndexDefinition::declare_for_property(
                &IndexDefinitionId::default(),
                &tenant_id,
                &database_id,
                &string,
                IndexPolicy::Exact,
                true,
            )
            .is_ok()
        );
        assert!(
            IndexDefinition::declare_for_property(
                &IndexDefinitionId::default(),
                &tenant_id,
                &database_id,
                &string,
                IndexPolicy::Range,
                false,
            )
            .is_err()
        );

        let markdown =
            property(&tenant_id, &database_id, PropertyType::Markdown);
        assert!(
            IndexDefinition::declare_for_property(
                &IndexDefinitionId::default(),
                &tenant_id,
                &database_id,
                &markdown,
                IndexPolicy::FullText,
                false,
            )
            .is_ok()
        );
        assert!(
            IndexDefinition::declare_for_property(
                &IndexDefinitionId::default(),
                &tenant_id,
                &database_id,
                &markdown,
                IndexPolicy::Exact,
                false,
            )
            .is_err()
        );
        assert!(
            IndexDefinition::declare_for_property(
                &IndexDefinitionId::default(),
                &tenant_id,
                &database_id,
                &markdown,
                IndexPolicy::FullText,
                true,
            )
            .is_err()
        );
    }

    #[test]
    fn opaque_property_definition_has_no_index_capabilities() {
        let tenant_id = TenantId::default();
        let database_id = DatabaseId::default();
        let opaque = PropertyDefinition::new(
            &PropertyId::default(),
            &tenant_id,
            &database_id,
            "future",
            ResolvedPropertyConfig::Opaque(OpaquePropertyConfig {
                type_ref: PropertyTypeRef::new(
                    PropertyTypeKey::new("future_scalar")
                        .expect("type key"),
                    PropertyTypeVersion::new(9).expect("type version"),
                ),
                raw_config: serde_json::json!({"future": true}),
            }),
            false,
            0,
            None,
        );

        let error = IndexDefinition::declare_for_property(
            &IndexDefinitionId::default(),
            &tenant_id,
            &database_id,
            &opaque,
            IndexPolicy::Exact,
            false,
        )
        .expect_err("future types are read-only, not legacy-indexable");
        assert!(error.to_string().contains("NotSupported"));
    }

    #[test]
    fn relation_targets_are_exact_non_unique_only() {
        let tenant_id = TenantId::default();
        let database_id = DatabaseId::default();
        let relation = relation(&tenant_id, &database_id);
        assert!(
            IndexDefinition::declare_for_relation(
                &IndexDefinitionId::default(),
                &tenant_id,
                &database_id,
                &relation,
                IndexPolicy::Exact,
                false,
            )
            .is_ok()
        );
        assert!(
            IndexDefinition::declare_for_relation(
                &IndexDefinitionId::default(),
                &tenant_id,
                &database_id,
                &relation,
                IndexPolicy::Range,
                false,
            )
            .is_err()
        );
        assert!(
            IndexDefinition::declare_for_relation(
                &IndexDefinitionId::default(),
                &tenant_id,
                &database_id,
                &relation,
                IndexPolicy::Exact,
                true,
            )
            .is_err()
        );

        assert!(
            IndexDefinition::restore(
                &IndexDefinitionId::default(),
                &tenant_id,
                &database_id,
                IndexTarget::Relation(relation.id().clone()),
                IndexPolicy::Exact,
                true,
                INDEX_DEFINITION_VERSION_V1,
                INDEX_GENERATION_V1,
                IndexProjectionState::Pending,
            )
            .is_err()
        );
    }

    #[test]
    fn generation_and_projection_lifecycle_are_explicit() {
        let tenant_id = TenantId::default();
        let database_id = DatabaseId::default();
        let integer =
            property(&tenant_id, &database_id, PropertyType::Integer);
        let definition = IndexDefinition::declare_for_property(
            &IndexDefinitionId::default(),
            &tenant_id,
            &database_id,
            &integer,
            IndexPolicy::None,
            false,
        )
        .expect("disabled declaration");
        assert_eq!(
            *definition.projection_state(),
            IndexProjectionState::Disabled
        );

        let pending = definition
            .reconfigure_for_property(&integer, IndexPolicy::Range, true)
            .expect("range declaration");
        assert_eq!(pending.generation().get(), 2);
        assert_eq!(
            *pending.projection_state(),
            IndexProjectionState::Pending
        );
        let building = pending
            .transition_projection(IndexProjectionState::Building)
            .expect("start projection");
        assert_eq!(building.generation().get(), 3);
        let failed = building
            .transition_projection(IndexProjectionState::Failed)
            .expect("fail projection");
        assert_eq!(failed.generation(), building.generation());
        let retry = failed
            .transition_projection(IndexProjectionState::Building)
            .expect("retry projection");
        assert_eq!(retry.generation().get(), 4);
        let ready = retry
            .transition_projection(IndexProjectionState::Ready)
            .expect("complete projection");
        assert_eq!(*ready.projection_state(), IndexProjectionState::Ready);
        assert!(
            ready
                .transition_projection(IndexProjectionState::Failed)
                .is_err()
        );
    }

    #[test]
    fn target_scope_and_unknown_versions_fail_closed() {
        let tenant_id = TenantId::default();
        let database_id = DatabaseId::default();
        let foreign_property = property(
            &TenantId::default(),
            &database_id,
            PropertyType::String,
        );
        assert!(
            IndexDefinition::declare_for_property(
                &IndexDefinitionId::default(),
                &tenant_id,
                &database_id,
                &foreign_property,
                IndexPolicy::Exact,
                false,
            )
            .expect_err("foreign target must be hidden")
            .is_not_found()
        );

        let restored = IndexDefinition::restore(
            &IndexDefinitionId::default(),
            &tenant_id,
            &database_id,
            IndexTarget::Property(PropertyId::default()),
            IndexPolicy::Exact,
            false,
            IndexDefinitionVersion::new(2).expect("future version"),
            INDEX_GENERATION_V1,
            IndexProjectionState::Ready,
        )
        .expect("future definitions remain readable");
        assert!(restored.ensure_writable().is_err());
    }

    #[test]
    fn relation_property_capability_remains_distinct_from_relation_target()
    {
        let tenant_id = TenantId::default();
        let database_id = DatabaseId::default();
        let relation_property = property(
            &tenant_id,
            &database_id,
            PropertyType::Relation(
                TypeRelation::new(DatabaseId::default()),
            ),
        );
        let definition = IndexDefinition::declare_for_property(
            &IndexDefinitionId::default(),
            &tenant_id,
            &database_id,
            &relation_property,
            IndexPolicy::Exact,
            false,
        )
        .expect("forward multi-reference exact index");
        assert!(matches!(definition.target(), IndexTarget::Property(_)));
    }
}
