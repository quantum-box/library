-- Expand-only storage for normalized Relation edges.
--
-- This migration intentionally creates no edge rows. Writers remain disabled
-- until every Relation/Record/Database deletion path removes or evaluates
-- edges in its cleanup-aware Unit of Work; otherwise these RESTRICT guards
-- will safely reject the legacy delete rather than orphan an edge.
-- MySQL commits the ALTER and CREATE as separate DDL statements. If execution
-- stops between them, use the partial-DDL repair in the rollout runbook before
-- asking sqlx to retry; a duplicate candidate-key ALTER is not idempotent.

-- The scoped definition FK below must bind both endpoints to the canonical
-- RelationDefinition. `relationships.id` remains globally unique, but this
-- tenant/source/target-leading candidate key makes the whole scope physical.
ALTER TABLE relationships
    ADD CONSTRAINT uq_relationships_edge_scope
        UNIQUE (tenant_id, object_id, id, target_object_id);

CREATE TABLE relation_edges (
    tenant_id VARCHAR(29) NOT NULL,
    source_database_id VARCHAR(29) NOT NULL,
    source_data_id VARCHAR(31) NOT NULL,
    relation_id VARCHAR(31) NOT NULL,
    target_database_id VARCHAR(29) NOT NULL,
    target_data_id VARCHAR(31) NOT NULL,
    created_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),

    -- Relation v1 is an unordered set. There is no synthetic edge id and no
    -- mirrored inverse row: this complete scope is the logical identity.
    PRIMARY KEY (
        tenant_id,
        source_database_id,
        source_data_id,
        relation_id,
        target_database_id,
        target_data_id
    ),

    CONSTRAINT fk_relation_edges_definition_scope
        FOREIGN KEY (
            tenant_id,
            source_database_id,
            relation_id,
            target_database_id
        )
        REFERENCES relationships (
            tenant_id,
            object_id,
            id,
            target_object_id
        )
        ON DELETE RESTRICT,

    CONSTRAINT fk_relation_edges_source_record
        FOREIGN KEY (
            tenant_id,
            source_database_id,
            source_data_id
        )
        REFERENCES data (tenant_id, object_id, id)
        ON DELETE RESTRICT,

    CONSTRAINT fk_relation_edges_target_record
        FOREIGN KEY (
            tenant_id,
            target_database_id,
            target_data_id
        )
        REFERENCES data (tenant_id, object_id, id)
        ON DELETE RESTRICT,

    INDEX idx_relation_edges_forward (
        tenant_id,
        source_database_id,
        relation_id,
        target_database_id,
        source_data_id,
        target_data_id
    ),

    INDEX idx_relation_edges_backlink (
        tenant_id,
        target_database_id,
        target_data_id,
        relation_id,
        source_database_id,
        source_data_id
    )
);
