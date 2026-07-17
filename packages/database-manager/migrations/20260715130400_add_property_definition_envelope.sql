-- Add the canonical PropertyDefinition type/config envelope without changing
-- the legacy reader or writer. Existing rows remain valid with all three
-- columns NULL until a later dual-write/backfill rollout.
-- Add the columns before the constraints because TiDB cannot resolve columns
-- added earlier in the same multi-schema ALTER inside CHECK expressions.
ALTER TABLE fields
    ADD COLUMN type_key VARCHAR(64) NULL,
    ADD COLUMN type_version SMALLINT UNSIGNED NULL,
    ADD COLUMN type_config LONGTEXT NULL;

ALTER TABLE fields
    ADD CONSTRAINT ck_fields_property_definition_envelope_complete
        CHECK (
            (
                type_key IS NULL
                AND type_version IS NULL
                AND type_config IS NULL
            )
            OR
            (
                type_key IS NOT NULL
                AND type_version IS NOT NULL
                AND type_config IS NOT NULL
            )
        ),
    ADD CONSTRAINT ck_fields_property_definition_type_version
        CHECK (type_version IS NULL OR type_version > 0),
    ADD CONSTRAINT ck_fields_property_definition_type_key
        CHECK (
            type_key IS NULL
            OR REGEXP_LIKE(
                type_key,
                '^[a-z][a-z0-9]*(_[a-z0-9]+)*$',
                'c'
            )
        ),
    ADD CONSTRAINT ck_fields_property_definition_type_config
        CHECK (type_config IS NULL OR JSON_VALID(type_config));
