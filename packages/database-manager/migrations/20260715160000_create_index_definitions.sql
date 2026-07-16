-- Declarative IndexDefinition control plane. This migration creates no value
-- projection and intentionally does not infer definitions from legacy
-- `fields.is_indexed` or `indexes` rows.

-- RelationDefinition ids need a tenant/database-leading candidate key before
-- they can be referenced without weakening the Database BC scope.
ALTER TABLE relationships
    ADD CONSTRAINT uq_relationships_tenant_object_id
        UNIQUE (tenant_id, object_id, id);

CREATE TABLE index_definitions (
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

    CONSTRAINT chk_index_definitions_exactly_one_target CHECK (
        (property_id IS NOT NULL AND relation_id IS NULL)
        OR (property_id IS NULL AND relation_id IS NOT NULL)
    ),
    CONSTRAINT chk_index_definitions_policy CHECK (
        CAST(policy AS BINARY) IN ('NONE', 'EXACT', 'RANGE', 'FULL_TEXT')
    ),
    CONSTRAINT chk_index_definitions_unique_policy CHECK (
        is_unique = FALSE
        OR CAST(policy AS BINARY) IN ('EXACT', 'RANGE')
    ),
    CONSTRAINT chk_index_definitions_relation_policy CHECK (
        relation_id IS NULL
        OR (
            CAST(policy AS BINARY) IN ('NONE', 'EXACT')
            AND is_unique = FALSE
        )
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
