use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Debug;

use derive_getters::Getters;
use value_object::TenantId;

use crate::{
    DataId, DatabaseId, RelationCardinality, RelationDefinition, RelationId,
};

/// Complete Database/record identity used at either side of a RelationEdge.
///
/// A DataId is globally unique today, but carrying its DatabaseId prevents
/// that implementation detail from weakening the bounded-context contract.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Getters)]
pub struct RecordReference {
    database_id: DatabaseId,
    data_id: DataId,
}

impl RecordReference {
    pub fn new(database_id: &DatabaseId, data_id: &DataId) -> Self {
        Self {
            database_id: database_id.clone(),
            data_id: data_id.clone(),
        }
    }
}

/// One canonical, forward-oriented edge.
///
/// Inverse Properties read the same row through the backlink index. Self
/// Relations and self loops are valid and therefore have no special marker.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Getters)]
pub struct RelationEdge {
    tenant_id: TenantId,
    relation_id: RelationId,
    source: RecordReference,
    target: RecordReference,
}

impl RelationEdge {
    pub fn new(
        definition: &RelationDefinition,
        source: RecordReference,
        target: RecordReference,
    ) -> errors::Result<Self> {
        definition.ensure_writable()?;
        Self::restore(
            definition.tenant_id(),
            definition.id(),
            source,
            target,
            definition,
        )
    }

    /// Restore an edge while checking the complete persisted scope against
    /// its canonical RelationDefinition. Future definition versions remain
    /// readable because this path performs no mutation.
    pub fn restore(
        tenant_id: &TenantId,
        relation_id: &RelationId,
        source: RecordReference,
        target: RecordReference,
        definition: &RelationDefinition,
    ) -> errors::Result<Self> {
        if tenant_id != definition.tenant_id() {
            return Err(errors::Error::invalid(
                "RelationEdge tenant does not match RelationDefinition",
            ));
        }
        if relation_id != definition.id() {
            return Err(errors::Error::invalid(
                "RelationEdge id does not match RelationDefinition",
            ));
        }
        if source.database_id() != definition.source_database_id() {
            return Err(errors::Error::invalid(
                "RelationEdge source Database does not match RelationDefinition",
            ));
        }
        if target.database_id() != definition.target_database_id() {
            return Err(errors::Error::invalid(
                "RelationEdge target Database does not match RelationDefinition",
            ));
        }
        Ok(Self {
            tenant_id: tenant_id.clone(),
            relation_id: relation_id.clone(),
            source,
            target,
        })
    }
}

/// An unordered, definition-scoped set of Relation edges.
///
/// The constructor rejects duplicate logical identities and cardinality
/// violations visible in the supplied set. A later writer UoW must serialize
/// and re-check the complete persisted set before it changes any rows.
#[derive(Debug, Clone, PartialEq, Eq, Getters)]
pub struct RelationEdgeSet {
    definition: RelationDefinition,
    edges: Vec<RelationEdge>,
}

impl RelationEdgeSet {
    pub fn new(
        definition: &RelationDefinition,
        mut edges: Vec<RelationEdge>,
    ) -> errors::Result<Self> {
        let mut identities = BTreeSet::new();
        let mut targets_by_source =
            BTreeMap::<RecordReference, BTreeSet<RecordReference>>::new();
        let mut sources_by_target =
            BTreeMap::<RecordReference, BTreeSet<RecordReference>>::new();

        for edge in &edges {
            RelationEdge::restore(
                edge.tenant_id(),
                edge.relation_id(),
                edge.source().clone(),
                edge.target().clone(),
                definition,
            )?;

            let identity = (edge.source().clone(), edge.target().clone());
            if !identities.insert(identity) {
                return Err(errors::Error::invalid(
                    "duplicate RelationEdge logical identity",
                ));
            }
            targets_by_source
                .entry(edge.source().clone())
                .or_default()
                .insert(edge.target().clone());
            sources_by_target
                .entry(edge.target().clone())
                .or_default()
                .insert(edge.source().clone());
        }

        if *definition.forward_cardinality() == RelationCardinality::One
            && targets_by_source.values().any(|targets| targets.len() > 1)
        {
            return Err(errors::Error::conflict(
                "Relation forward cardinality ONE was exceeded",
            ));
        }
        if *definition.reverse_cardinality() == RelationCardinality::One
            && sources_by_target.values().any(|sources| sources.len() > 1)
        {
            return Err(errors::Error::conflict(
                "Relation reverse cardinality ONE was exceeded",
            ));
        }

        // Relation v1 is an unordered set. Keep one canonical in-memory order
        // so equality, parity, and later mutation planning are deterministic.
        edges.sort();
        Ok(Self {
            definition: definition.clone(),
            edges,
        })
    }
}

/// Tenant-scoped, read-only port for forward and inverse/backlink views.
///
/// Mutations deliberately do not belong to this expand contract. They will be
/// introduced only as part of the cleanup-aware Record Unit of Work.
#[async_trait::async_trait]
pub trait RelationEdgeRepository: Debug + Send + Sync + 'static {
    async fn find_forward(
        &self,
        tenant_id: &TenantId,
        definition: &RelationDefinition,
        source_data_id: &DataId,
    ) -> errors::Result<RelationEdgeSet>;

    async fn find_backlinks(
        &self,
        tenant_id: &TenantId,
        definition: &RelationDefinition,
        target_data_id: &DataId,
    ) -> errors::Result<RelationEdgeSet>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        PropertyId, RelationDefinitionVersion, RelationGeneration,
        RelationOnDelete,
    };

    fn definition(
        tenant_id: &TenantId,
        source_database_id: &DatabaseId,
        target_database_id: &DatabaseId,
        forward: RelationCardinality,
        reverse: RelationCardinality,
    ) -> RelationDefinition {
        RelationDefinition::new(
            &RelationId::default(),
            tenant_id,
            source_database_id,
            &PropertyId::default(),
            target_database_id,
            forward,
            reverse,
            None,
            RelationOnDelete::Restrict,
        )
        .expect("valid definition")
    }

    fn edge(
        definition: &RelationDefinition,
        source_data_id: &DataId,
        target_data_id: &DataId,
    ) -> RelationEdge {
        RelationEdge::new(
            definition,
            RecordReference::new(
                definition.source_database_id(),
                source_data_id,
            ),
            RecordReference::new(
                definition.target_database_id(),
                target_data_id,
            ),
        )
        .expect("valid edge")
    }

    #[test]
    fn self_relation_and_self_loop_are_valid() {
        let tenant_id = TenantId::default();
        let database_id = DatabaseId::default();
        let data_id = DataId::default();
        let definition = definition(
            &tenant_id,
            &database_id,
            &database_id,
            RelationCardinality::One,
            RelationCardinality::One,
        );

        let set = RelationEdgeSet::new(
            &definition,
            vec![edge(&definition, &data_id, &data_id)],
        )
        .expect("a self loop is one canonical edge");
        assert_eq!(set.edges().len(), 1);
    }

    #[test]
    fn edge_scope_must_match_the_definition() {
        let tenant_id = TenantId::default();
        let source_database_id = DatabaseId::default();
        let target_database_id = DatabaseId::default();
        let definition = definition(
            &tenant_id,
            &source_database_id,
            &target_database_id,
            RelationCardinality::Many,
            RelationCardinality::Many,
        );
        let source =
            RecordReference::new(&source_database_id, &DataId::default());
        let wrong_target = RecordReference::new(
            &DatabaseId::default(),
            &DataId::default(),
        );

        assert!(
            RelationEdge::new(&definition, source, wrong_target).is_err()
        );
        assert!(
            RelationEdge::restore(
                &TenantId::default(),
                definition.id(),
                RecordReference::new(
                    &source_database_id,
                    &DataId::default(),
                ),
                RecordReference::new(
                    &target_database_id,
                    &DataId::default(),
                ),
                &definition,
            )
            .is_err()
        );
    }

    #[test]
    fn set_rejects_duplicate_logical_edges() {
        let tenant_id = TenantId::default();
        let definition = definition(
            &tenant_id,
            &DatabaseId::default(),
            &DatabaseId::default(),
            RelationCardinality::Many,
            RelationCardinality::Many,
        );
        let edge =
            edge(&definition, &DataId::default(), &DataId::default());

        assert!(
            RelationEdgeSet::new(&definition, vec![edge.clone(), edge])
                .is_err()
        );
    }

    #[test]
    fn set_enforces_both_cardinality_directions() {
        let tenant_id = TenantId::default();
        let source_database_id = DatabaseId::default();
        let target_database_id = DatabaseId::default();
        let source_a = DataId::default();
        let source_b = DataId::default();
        let target_a = DataId::default();
        let target_b = DataId::default();

        let forward_one = definition(
            &tenant_id,
            &source_database_id,
            &target_database_id,
            RelationCardinality::One,
            RelationCardinality::Many,
        );
        assert!(
            RelationEdgeSet::new(
                &forward_one,
                vec![
                    edge(&forward_one, &source_a, &target_a),
                    edge(&forward_one, &source_a, &target_b),
                ],
            )
            .is_err()
        );

        let reverse_one = definition(
            &tenant_id,
            &source_database_id,
            &target_database_id,
            RelationCardinality::Many,
            RelationCardinality::One,
        );
        assert!(
            RelationEdgeSet::new(
                &reverse_one,
                vec![
                    edge(&reverse_one, &source_a, &target_a),
                    edge(&reverse_one, &source_b, &target_a),
                ],
            )
            .is_err()
        );
    }

    #[test]
    fn edges_remain_readable_for_future_definition_versions() {
        let tenant_id = TenantId::default();
        let source_database_id = DatabaseId::default();
        let target_database_id = DatabaseId::default();
        let definition = RelationDefinition::restore(
            &RelationId::default(),
            &tenant_id,
            &source_database_id,
            &PropertyId::default(),
            &target_database_id,
            RelationCardinality::Many,
            RelationCardinality::Many,
            None,
            false,
            RelationOnDelete::Restrict,
            RelationDefinitionVersion::new(2).expect("future version"),
            RelationGeneration::new(1).expect("valid generation"),
        )
        .expect("future definition is readable");

        let source = RecordReference::new(
            definition.source_database_id(),
            &DataId::default(),
        );
        let target = RecordReference::new(
            definition.target_database_id(),
            &DataId::default(),
        );
        assert!(
            RelationEdge::new(&definition, source.clone(), target.clone())
                .is_err(),
            "future definitions must not create edges"
        );
        let restored = RelationEdge::restore(
            definition.tenant_id(),
            definition.id(),
            source,
            target,
            &definition,
        )
        .expect("future edge remains readable");
        let set = RelationEdgeSet::new(&definition, vec![restored]).expect(
            "read construction must not require a writable definition",
        );
        assert_eq!(set.edges().len(), 1);
    }

    #[test]
    fn set_equality_is_independent_of_input_order() {
        let tenant_id = TenantId::default();
        let definition = definition(
            &tenant_id,
            &DatabaseId::default(),
            &DatabaseId::default(),
            RelationCardinality::Many,
            RelationCardinality::Many,
        );
        let first =
            edge(&definition, &DataId::default(), &DataId::default());
        let second =
            edge(&definition, &DataId::default(), &DataId::default());

        let ordered = RelationEdgeSet::new(
            &definition,
            vec![first.clone(), second.clone()],
        )
        .expect("valid set");
        let reversed =
            RelationEdgeSet::new(&definition, vec![second, first])
                .expect("valid set");
        assert_eq!(ordered, reversed);
    }
}
