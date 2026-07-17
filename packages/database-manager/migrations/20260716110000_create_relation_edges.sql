-- Expand-only storage for normalized Relation edges.
--
-- This migration intentionally creates no edge rows. Writers remain disabled
-- until every Relation/Record/Database deletion path removes or evaluates
-- edges in its cleanup-aware Unit of Work; otherwise these RESTRICT guards
-- will safely reject the legacy delete rather than orphan an edge.
-- MySQL and TiDB commit the ALTER and CREATE as separate DDL statements, so
-- both operations must be safe when sqlx retries after a partial execution.

-- The scoped definition FK below must bind both endpoints to the canonical
-- RelationDefinition. `relationships.id` remains globally unique, but this
-- tenant/source/target-leading candidate key makes the whole scope physical.
SET @library_relation_edge_scope_key_ddl = IF(
    (
        SELECT COUNT(*)
        FROM information_schema.STATISTICS
        WHERE TABLE_SCHEMA = DATABASE()
          AND TABLE_NAME = 'relationships'
          AND INDEX_NAME = 'uq_relationships_edge_scope'
    ) = 0,
    'ALTER TABLE relationships ADD CONSTRAINT uq_relationships_edge_scope UNIQUE (tenant_id, object_id, id, target_object_id)',
    'SELECT 1'
);
PREPARE library_relation_edge_scope_key_stmt
    FROM @library_relation_edge_scope_key_ddl;
EXECUTE library_relation_edge_scope_key_stmt;
DEALLOCATE PREPARE library_relation_edge_scope_key_stmt;

CREATE TABLE IF NOT EXISTS relation_edges (
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
