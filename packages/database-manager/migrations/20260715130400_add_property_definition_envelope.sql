-- Add the canonical PropertyDefinition type/config envelope without changing
-- the legacy reader or writer. Existing rows remain valid with all three
-- columns NULL until a later dual-write/backfill rollout.
-- Add the columns before the constraints because TiDB cannot resolve columns
-- added earlier in the same multi-schema ALTER inside CHECK expressions.
-- Use dynamic DDL so the migration resumes after TiDB auto-commits a column
-- but remains compatible with MySQL, which lacks ADD COLUMN IF NOT EXISTS.
SET @library_property_type_key_ddl = IF(
    (
        SELECT COUNT(*)
        FROM information_schema.COLUMNS
        WHERE TABLE_SCHEMA = DATABASE()
          AND TABLE_NAME = 'fields'
          AND COLUMN_NAME = 'type_key'
    ) = 0,
    'ALTER TABLE fields ADD COLUMN type_key VARCHAR(64) NULL',
    'SELECT 1'
);
PREPARE library_property_column_stmt
    FROM @library_property_type_key_ddl;
EXECUTE library_property_column_stmt;
DEALLOCATE PREPARE library_property_column_stmt;

SET @library_property_type_version_ddl = IF(
    (
        SELECT COUNT(*)
        FROM information_schema.COLUMNS
        WHERE TABLE_SCHEMA = DATABASE()
          AND TABLE_NAME = 'fields'
          AND COLUMN_NAME = 'type_version'
    ) = 0,
    'ALTER TABLE fields ADD COLUMN type_version SMALLINT UNSIGNED NULL',
    'SELECT 1'
);
PREPARE library_property_column_stmt
    FROM @library_property_type_version_ddl;
EXECUTE library_property_column_stmt;
DEALLOCATE PREPARE library_property_column_stmt;

SET @library_property_type_config_ddl = IF(
    (
        SELECT COUNT(*)
        FROM information_schema.COLUMNS
        WHERE TABLE_SCHEMA = DATABASE()
          AND TABLE_NAME = 'fields'
          AND COLUMN_NAME = 'type_config'
    ) = 0,
    'ALTER TABLE fields ADD COLUMN type_config LONGTEXT NULL',
    'SELECT 1'
);
PREPARE library_property_column_stmt
    FROM @library_property_type_config_ddl;
EXECUTE library_property_column_stmt;
DEALLOCATE PREPARE library_property_column_stmt;

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
        );
