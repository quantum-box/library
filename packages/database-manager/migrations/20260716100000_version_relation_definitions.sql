-- Version RelationDefinition mutations without changing the legacy physical
-- table or its compatibility columns. Existing definitions become readable
-- V1/generation-1 rows. Only inverse Properties created by the new schema
-- mutation UoW are marked as owned; compatibility inverses remain detached
-- from lifecycle deletion.
ALTER TABLE relationships
    DROP FOREIGN KEY fk_relationships_tenant_source_property,
    DROP CHECK chk_relationships_forward_cardinality,
    DROP CHECK chk_relationships_reverse_cardinality,
    DROP CHECK chk_relationships_on_target_delete;

-- MySQL keeps the dropped foreign-key name reserved until the ALTER finishes,
-- so the RESTRICT replacement must be created in a separate statement.
ALTER TABLE relationships
    ADD CONSTRAINT chk_relationships_forward_cardinality
        CHECK (CAST(forward_cardinality AS BINARY) IN ('ONE', 'MANY')),
    ADD CONSTRAINT chk_relationships_reverse_cardinality
        CHECK (CAST(reverse_cardinality AS BINARY) IN ('ONE', 'MANY')),
    ADD CONSTRAINT chk_relationships_on_target_delete
        CHECK (CAST(on_target_delete AS BINARY) IN ('RESTRICT', 'NULLIFY')),
    ADD COLUMN inverse_owned BOOLEAN NOT NULL DEFAULT FALSE
        AFTER inverse_field_id,
    ADD COLUMN definition_version SMALLINT UNSIGNED NOT NULL DEFAULT 1
        AFTER on_target_delete,
    ADD COLUMN generation BIGINT UNSIGNED NOT NULL DEFAULT 1
        AFTER definition_version,
    ADD CONSTRAINT chk_relationships_owned_inverse
        CHECK (inverse_owned = FALSE OR inverse_field_id IS NOT NULL),
    ADD CONSTRAINT chk_relationships_distinct_self_inverse
        CHECK (
            inverse_field_id IS NULL
            OR object_id <> target_object_id
            OR inverse_field_id <> field_id
        ),
    ADD CONSTRAINT chk_relationships_definition_version
        CHECK (definition_version > 0),
    ADD CONSTRAINT chk_relationships_generation
        CHECK (generation > 0),
    ADD CONSTRAINT fk_relationships_tenant_source_property
        FOREIGN KEY (tenant_id, object_id, field_id)
        REFERENCES fields (tenant_id, object_id, id)
        ON DELETE RESTRICT;
