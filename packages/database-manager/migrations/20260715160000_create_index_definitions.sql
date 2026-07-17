-- Declarative IndexDefinition control plane. This migration creates no value
-- projection and intentionally does not infer definitions from legacy
-- `fields.is_indexed` or `indexes` rows.

-- RelationDefinition ids need a tenant/database-leading candidate key before
-- they can be referenced without weakening the Database BC scope.
-- Resume safely if TiDB committed this key before a later statement failed.
SET @library_relation_identity_key_ddl = IF(
    (
        SELECT COUNT(*)
        FROM information_schema.STATISTICS
        WHERE TABLE_SCHEMA = DATABASE()
          AND TABLE_NAME = 'relationships'
          AND INDEX_NAME = 'uq_relationships_tenant_object_id'
    ) = 0,
    'ALTER TABLE relationships ADD CONSTRAINT uq_relationships_tenant_object_id UNIQUE (tenant_id, object_id, id)',
    'SELECT 1'
);
PREPARE library_relation_identity_key_stmt
    FROM @library_relation_identity_key_ddl;
EXECUTE library_relation_identity_key_stmt;
DEALLOCATE PREPARE library_relation_identity_key_stmt;

CREATE TABLE IF NOT EXISTS index_definitions (
    id VARCHAR(29) NOT NULL,
    tenant_id VARCHAR(29) NOT NULL,
    database_id VARCHAR(29) NOT NULL,
    property_id VARCHAR(31) NULL,
    relation_id VARCHAR(31) NULL,
    policy VARCHAR(9) NOT NULL DEFAULT 'NONE',
    is_unique BOOLEAN NOT NULL DEFAULT FALSE,
    definition_version SMALLINT UNSIGNED NOT NULL DEFAULT 1,
    generation BIGINT UNSIGNED NOT NULL DEFAULT 1,
    projection_state VARCHAR(8) NOT NULL DEFAULT 'DISABLED',
    created_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
        ON UPDATE CURRENT_TIMESTAMP(6),

    PRIMARY KEY (id),
    CONSTRAINT uq_index_definitions_tenant_database_id
        UNIQUE (tenant_id, database_id, id),
    CONSTRAINT uq_index_definitions_property_target
        UNIQUE (tenant_id, database_id, property_id),
    CONSTRAINT uq_index_definitions_relation_target
        UNIQUE (tenant_id, database_id, relation_id),

    -- MySQL and TiDB prohibit CHECK constraints from reading columns used by
    -- ON DELETE CASCADE foreign keys. The domain constructor and fail-closed
    -- repository decoder enforce exactly one target; keep CASCADE here so
    -- Property and Relation lifecycle deletion remains atomic.
    CONSTRAINT chk_index_definitions_policy CHECK (
        CAST(policy AS BINARY) IN ('NONE', 'EXACT', 'RANGE', 'FULL_TEXT')
    ),
    CONSTRAINT chk_index_definitions_unique_policy CHECK (
        is_unique = FALSE
        OR CAST(policy AS BINARY) IN ('EXACT', 'RANGE')
    ),
    CONSTRAINT chk_index_definitions_version CHECK (
        definition_version > 0
    ),
    CONSTRAINT chk_index_definitions_generation CHECK (generation > 0),
    CONSTRAINT chk_index_definitions_projection_state CHECK (
        CAST(projection_state AS BINARY) IN (
            'DISABLED', 'PENDING', 'BUILDING', 'READY', 'FAILED'
        )
    ),
    CONSTRAINT chk_index_definitions_policy_projection CHECK (
        (
            CAST(policy AS BINARY) = 'NONE'
            AND CAST(projection_state AS BINARY) = 'DISABLED'
        )
        OR (
            CAST(policy AS BINARY) <> 'NONE'
            AND CAST(projection_state AS BINARY) <> 'DISABLED'
        )
    ),

    CONSTRAINT fk_index_definitions_tenant_database
        FOREIGN KEY (tenant_id, database_id)
        REFERENCES objects (tenant_id, id)
        ON DELETE CASCADE,
    CONSTRAINT fk_index_definitions_property_target
        FOREIGN KEY (tenant_id, database_id, property_id)
        REFERENCES fields (tenant_id, object_id, id)
        ON DELETE CASCADE,
    CONSTRAINT fk_index_definitions_relation_target
        FOREIGN KEY (tenant_id, database_id, relation_id)
        REFERENCES relationships (tenant_id, object_id, id)
        ON DELETE CASCADE,

    INDEX idx_index_definitions_tenant_database_state
        (tenant_id, database_id, projection_state, id),
    INDEX idx_index_definitions_tenant_policy
        (tenant_id, policy, database_id, id)
);
