use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Debug;

use derive_getters::Getters;
use value_object::TenantId;

use crate::{
    DataId, DatabaseId, RelationCardinality, RelationDefinition,
    RelationId, RelationOnDelete,
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

/// Deterministic replacement of one source Record's forward Relation value.
///
/// The adapter must pass the complete current forward set for `source` and
/// every persisted backlink for the requested targets while holding the
/// RelationDefinition row lock. That row is the serialization mutex shared
/// with cardinality reconfiguration and cleanup-aware Record deletion.
#[derive(Debug, Clone, PartialEq, Eq, Getters)]
pub struct RelationEdgeMutationPlan {
    definition: RelationDefinition,
    source: RecordReference,
    desired: RelationEdgeSet,
    insertions: Vec<RelationEdge>,
    deletions: Vec<RelationEdge>,
}

impl RelationEdgeMutationPlan {
    pub fn replace_forward(
        definition: &RelationDefinition,
        source: RecordReference,
        current_forward: &RelationEdgeSet,
        requested_target_backlinks: &RelationEdgeSet,
        requested_target_ids: Vec<DataId>,
    ) -> errors::Result<Self> {
        definition.ensure_writable()?;
        if current_forward.definition() != definition
            || requested_target_backlinks.definition() != definition
        {
            return Err(errors::Error::invalid(
                "RelationEdge mutation scope uses a different RelationDefinition",
            ));
        }
        if source.database_id() != definition.source_database_id() {
            return Err(errors::Error::invalid(
                "RelationEdge mutation source Database does not match RelationDefinition",
            ));
        }
        if current_forward
            .edges()
            .iter()
            .any(|edge| edge.source() != &source)
        {
            return Err(errors::Error::invalid(
                "RelationEdge mutation requires one complete forward source scope",
            ));
        }

        let requested_target_count = requested_target_ids.len();
        let requested_target_ids =
            requested_target_ids.into_iter().collect::<BTreeSet<_>>();
        let desired_edges = requested_target_ids
            .iter()
            .map(|target_data_id| {
                RelationEdge::new(
                    definition,
                    source.clone(),
                    RecordReference::new(
                        definition.target_database_id(),
                        target_data_id,
                    ),
                )
            })
            .collect::<errors::Result<Vec<_>>>()?;

        // Relation values are sets. Silently deduplicating a malformed
        // command would make its accepted event differ from the caller's
        // payload and hide a client bug.
        if requested_target_count != requested_target_ids.len() {
            return Err(errors::Error::invalid(
                "duplicate RelationEdge target",
            ));
        }
        let desired = RelationEdgeSet::new(definition, desired_edges)?;

        for edge in requested_target_backlinks.edges() {
            if !requested_target_ids.contains(edge.target().data_id()) {
                return Err(errors::Error::invalid(
                    "RelationEdge backlink scope contains an unrequested target",
                ));
            }
            if *definition.reverse_cardinality() == RelationCardinality::One
                && edge.source() != &source
            {
                return Err(errors::Error::conflict(
                    "Relation reverse cardinality ONE was exceeded",
                ));
            }
        }

        let current = current_forward
            .edges()
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let next = desired.edges().iter().cloned().collect::<BTreeSet<_>>();
        let insertions = next.difference(&current).cloned().collect();
        let deletions = current.difference(&next).cloned().collect();

        Ok(Self {
            definition: definition.clone(),
            source,
            desired,
            insertions,
            deletions,
        })
    }
}

/// One inbound Relation value that must remove the deleted target.
///
/// Keeping the definition with the edge gives the adapter the owning source
/// Property while preserving the canonical forward-only edge identity.
#[derive(Debug, Clone, PartialEq, Eq, Getters)]
pub struct RelationNullification {
    definition: RelationDefinition,
    edge: RelationEdge,
}

/// All Nullify actions for one source Record.
///
/// A deletion UoW increments each affected source RecordVersion once, even if
/// several Relation Properties on that Record reference the deleted target.
#[derive(Debug, Clone, PartialEq, Eq, Getters)]
pub struct RelationSourceNullification {
    source: RecordReference,
    nullifications: Vec<RelationNullification>,
}

/// Deterministic classification of every RelationEdge incident to a Record.
///
/// The caller supplies exactly one locked incident edge set per definition.
/// Edges sourced by the deleted Record (including self-loops) are unconditional
/// cleanup. Only inbound edges from another Record apply the definition's
/// Restrict/Nullify target-deletion policy.
#[derive(Debug, Clone, PartialEq, Eq, Getters)]
pub struct RelationTargetDeletionPlan {
    tenant_id: TenantId,
    target: RecordReference,
    outgoing_edges: Vec<RelationEdge>,
    restricting_inbound_edges: Vec<RelationEdge>,
    nullify_groups: Vec<RelationSourceNullification>,
}

impl RelationTargetDeletionPlan {
    pub fn new(
        tenant_id: &TenantId,
        target: &RecordReference,
        scopes: Vec<RelationEdgeSet>,
    ) -> errors::Result<Self> {
        let mut relation_ids = BTreeSet::new();
        let mut edge_identities = BTreeSet::new();
        let mut outgoing_edges = Vec::new();
        let mut restricting_inbound_edges = Vec::new();
        let mut nullifications_by_source =
            BTreeMap::<RecordReference, Vec<RelationNullification>>::new();

        for scope in scopes {
            let RelationEdgeSet { definition, edges } = scope;
            definition.ensure_writable()?;
            if definition.tenant_id() != tenant_id {
                return Err(errors::Error::invalid(
                    "Relation target-deletion scope belongs to another tenant",
                ));
            }
            if definition.source_database_id() != target.database_id()
                && definition.target_database_id() != target.database_id()
            {
                return Err(errors::Error::invalid(
                    "Relation target-deletion definition is not incident to the target Database",
                ));
            }
            if !relation_ids.insert(definition.id().clone()) {
                return Err(errors::Error::invalid(
                    "duplicate Relation target-deletion definition scope",
                ));
            }

            for edge in edges {
                if !edge_identities.insert(edge.clone()) {
                    return Err(errors::Error::invalid(
                        "duplicate Relation target-deletion edge",
                    ));
                }

                if edge.source() == target {
                    // A self-loop is both incoming and outgoing. Classifying it
                    // here first prevents a Record from restricting its own
                    // deletion or scheduling a Nullify for a source that will
                    // be removed in the same transaction.
                    outgoing_edges.push(edge);
                    continue;
                }
                if edge.target() != target {
                    return Err(errors::Error::invalid(
                        "Relation target-deletion scope contains a non-incident edge",
                    ));
                }

                match definition.on_target_delete() {
                    RelationOnDelete::Restrict => {
                        restricting_inbound_edges.push(edge);
                    }
                    RelationOnDelete::Nullify => {
                        let source = edge.source().clone();
                        nullifications_by_source
                            .entry(source)
                            .or_default()
                            .push(RelationNullification {
                                definition: definition.clone(),
                                edge,
                            });
                    }
                }
            }
        }

        outgoing_edges.sort();
        restricting_inbound_edges.sort();
        let nullify_groups = nullifications_by_source
            .into_iter()
            .map(|(source, mut nullifications)| {
                nullifications
                    .sort_by(|left, right| left.edge.cmp(&right.edge));
                RelationSourceNullification {
                    source,
                    nullifications,
                }
            })
            .collect();

        Ok(Self {
            tenant_id: tenant_id.clone(),
            target: target.clone(),
            outgoing_edges,
            restricting_inbound_edges,
            nullify_groups,
        })
    }

    pub fn is_restricted(&self) -> bool {
        !self.restricting_inbound_edges.is_empty()
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
        definition_with_policy(
            tenant_id,
            source_database_id,
            target_database_id,
            forward,
            reverse,
            RelationOnDelete::Restrict,
        )
    }

    fn definition_with_policy(
        tenant_id: &TenantId,
        source_database_id: &DatabaseId,
        target_database_id: &DatabaseId,
        forward: RelationCardinality,
        reverse: RelationCardinality,
        on_target_delete: RelationOnDelete,
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
            on_target_delete,
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
        let relation_definition = definition(
            &tenant_id,
            &database_id,
            &database_id,
            RelationCardinality::One,
            RelationCardinality::One,
        );

        let set = RelationEdgeSet::new(
            &relation_definition,
            vec![edge(&relation_definition, &data_id, &data_id)],
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

    #[test]
    fn replacement_plan_is_a_deterministic_set_diff() {
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
        let source_data_id = DataId::default();
        let source =
            RecordReference::new(&source_database_id, &source_data_id);
        let retained_target = DataId::default();
        let removed_target = DataId::default();
        let inserted_target = DataId::default();
        let current = RelationEdgeSet::new(
            &definition,
            vec![
                edge(&definition, &source_data_id, &removed_target),
                edge(&definition, &source_data_id, &retained_target),
            ],
        )
        .expect("current forward set");
        let backlinks = RelationEdgeSet::new(&definition, Vec::new())
            .expect("no occupied backlinks");

        let plan = RelationEdgeMutationPlan::replace_forward(
            &definition,
            source,
            &current,
            &backlinks,
            vec![inserted_target.clone(), retained_target.clone()],
        )
        .expect("valid replacement");

        assert_eq!(plan.insertions().len(), 1);
        assert_eq!(plan.deletions().len(), 1);
        assert_eq!(
            plan.insertions()[0].target().data_id(),
            &inserted_target
        );
        assert_eq!(plan.deletions()[0].target().data_id(), &removed_target);
        assert_eq!(plan.desired().edges().len(), 2);
    }

    #[test]
    fn replacement_rejects_duplicate_targets_instead_of_deduplicating() {
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
        let empty = RelationEdgeSet::new(&definition, Vec::new())
            .expect("empty edge scope");
        let target = DataId::default();

        assert!(
            RelationEdgeMutationPlan::replace_forward(
                &definition,
                source,
                &empty,
                &empty,
                vec![target.clone(), target],
            )
            .is_err()
        );
    }

    #[test]
    fn replacement_enforces_forward_and_reverse_one() {
        let tenant_id = TenantId::default();
        let source_database_id = DatabaseId::default();
        let target_database_id = DatabaseId::default();
        let source_data_id = DataId::default();
        let other_source_data_id = DataId::default();
        let first_target = DataId::default();
        let second_target = DataId::default();

        let forward_one = definition(
            &tenant_id,
            &source_database_id,
            &target_database_id,
            RelationCardinality::One,
            RelationCardinality::Many,
        );
        let empty = RelationEdgeSet::new(&forward_one, Vec::new())
            .expect("empty edge scope");
        assert!(
            RelationEdgeMutationPlan::replace_forward(
                &forward_one,
                RecordReference::new(&source_database_id, &source_data_id),
                &empty,
                &empty,
                vec![first_target.clone(), second_target],
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
        let empty = RelationEdgeSet::new(&reverse_one, Vec::new())
            .expect("empty edge scope");
        let occupied = RelationEdgeSet::new(
            &reverse_one,
            vec![edge(&reverse_one, &other_source_data_id, &first_target)],
        )
        .expect("one occupied backlink");
        assert!(
            RelationEdgeMutationPlan::replace_forward(
                &reverse_one,
                RecordReference::new(&source_database_id, &source_data_id),
                &empty,
                &occupied,
                vec![first_target],
            )
            .is_err()
        );
    }

    #[test]
    fn target_deletion_classifies_self_loops_and_groups_nullify_by_source()
    {
        let tenant_id = TenantId::default();
        let deleted_database_id = DatabaseId::default();
        let deleted_data_id = DataId::default();
        let deleted =
            RecordReference::new(&deleted_database_id, &deleted_data_id);
        let other_database_id = DatabaseId::default();
        let source_database_id = DatabaseId::default();
        let source_a = DataId::default();
        let source_b = DataId::default();

        let outgoing_definition = definition_with_policy(
            &tenant_id,
            &deleted_database_id,
            &other_database_id,
            RelationCardinality::Many,
            RelationCardinality::Many,
            RelationOnDelete::Restrict,
        );
        let outgoing = edge(
            &outgoing_definition,
            &deleted_data_id,
            &DataId::default(),
        );
        let self_definition = definition_with_policy(
            &tenant_id,
            &deleted_database_id,
            &deleted_database_id,
            RelationCardinality::Many,
            RelationCardinality::Many,
            RelationOnDelete::Restrict,
        );
        let self_loop =
            edge(&self_definition, &deleted_data_id, &deleted_data_id);
        let restrict_definition = definition_with_policy(
            &tenant_id,
            &source_database_id,
            &deleted_database_id,
            RelationCardinality::Many,
            RelationCardinality::Many,
            RelationOnDelete::Restrict,
        );
        let restrict = edge(
            &restrict_definition,
            &DataId::default(),
            &deleted_data_id,
        );
        let nullify_a_definition = definition_with_policy(
            &tenant_id,
            &source_database_id,
            &deleted_database_id,
            RelationCardinality::Many,
            RelationCardinality::Many,
            RelationOnDelete::Nullify,
        );
        let nullify_b_definition = definition_with_policy(
            &tenant_id,
            &source_database_id,
            &deleted_database_id,
            RelationCardinality::Many,
            RelationCardinality::Many,
            RelationOnDelete::Nullify,
        );
        let nullify_a =
            edge(&nullify_a_definition, &source_a, &deleted_data_id);
        let nullify_b =
            edge(&nullify_b_definition, &source_a, &deleted_data_id);
        let nullify_other =
            edge(&nullify_a_definition, &source_b, &deleted_data_id);

        let scopes = vec![
            RelationEdgeSet::new(
                &nullify_a_definition,
                vec![nullify_other.clone(), nullify_a.clone()],
            )
            .expect("nullify a scope"),
            RelationEdgeSet::new(
                &outgoing_definition,
                vec![outgoing.clone()],
            )
            .expect("outgoing scope"),
            RelationEdgeSet::new(
                &restrict_definition,
                vec![restrict.clone()],
            )
            .expect("restrict scope"),
            RelationEdgeSet::new(&self_definition, vec![self_loop.clone()])
                .expect("self scope"),
            RelationEdgeSet::new(
                &nullify_b_definition,
                vec![nullify_b.clone()],
            )
            .expect("nullify b scope"),
        ];
        let mut reversed = scopes.clone();
        reversed.reverse();

        let plan =
            RelationTargetDeletionPlan::new(&tenant_id, &deleted, scopes)
                .expect("deletion plan");
        let reversed_plan =
            RelationTargetDeletionPlan::new(&tenant_id, &deleted, reversed)
                .expect("input order does not affect the plan");

        assert_eq!(plan, reversed_plan);
        assert!(plan.is_restricted());
        assert_eq!(plan.outgoing_edges().len(), 2);
        assert!(plan.outgoing_edges().contains(&outgoing));
        assert!(plan.outgoing_edges().contains(&self_loop));
        assert_eq!(plan.restricting_inbound_edges(), &vec![restrict]);
        assert_eq!(plan.nullify_groups().len(), 2);
        let grouped = plan
            .nullify_groups()
            .iter()
            .find(|group| group.source().data_id() == &source_a)
            .expect("source A group");
        assert_eq!(grouped.nullifications().len(), 2);
        assert!(
            grouped
                .nullifications()
                .iter()
                .any(|item| item.edge() == &nullify_a)
        );
        assert!(
            grouped
                .nullifications()
                .iter()
                .any(|item| item.edge() == &nullify_b)
        );
    }

    #[test]
    fn target_deletion_rejects_non_incident_and_duplicate_scopes() {
        let tenant_id = TenantId::default();
        let source_database_id = DatabaseId::default();
        let target_database_id = DatabaseId::default();
        let relation_definition = definition(
            &tenant_id,
            &source_database_id,
            &target_database_id,
            RelationCardinality::Many,
            RelationCardinality::Many,
        );
        let unrelated = edge(
            &relation_definition,
            &DataId::default(),
            &DataId::default(),
        );
        let scope =
            RelationEdgeSet::new(&relation_definition, vec![unrelated])
                .expect("valid relation scope");
        let deleted =
            RecordReference::new(&target_database_id, &DataId::default());

        assert!(
            RelationTargetDeletionPlan::new(
                &tenant_id,
                &deleted,
                vec![scope.clone()]
            )
            .is_err()
        );
        assert!(
            RelationTargetDeletionPlan::new(
                &tenant_id,
                &deleted,
                vec![
                    RelationEdgeSet::new(&relation_definition, Vec::new())
                        .expect("empty scope"),
                    RelationEdgeSet::new(&relation_definition, Vec::new())
                        .expect("duplicate empty scope"),
                ]
            )
            .is_err()
        );

        let unrelated_definition = definition(
            &tenant_id,
            &DatabaseId::default(),
            &DatabaseId::default(),
            RelationCardinality::Many,
            RelationCardinality::Many,
        );
        assert!(
            RelationTargetDeletionPlan::new(
                &tenant_id,
                &deleted,
                vec![RelationEdgeSet::new(
                    &unrelated_definition,
                    Vec::new(),
                )
                .expect("empty unrelated scope")]
            )
            .is_err()
        );
    }
}
