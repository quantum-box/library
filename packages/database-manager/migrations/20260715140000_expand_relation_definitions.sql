-- `relationships` already stores one row per Relation Property. Keep its
-- physical name and legacy columns during the compatibility window while
-- making the row an explicit RelationDefinition control plane.
--
-- Existing definitions came from a multi-value CSV representation and had no
-- inverse or lifecycle configuration. Preserve those semantics as
-- Many/Many/Restrict with no inverse Property. Dynamic DDL lets this migration
-- resume if TiDB committed any columns before rejecting the original combined
-- ALTER statement.
SET @library_relation_forward_cardinality_ddl = IF(
    (
        SELECT COUNT(*)
        FROM information_schema.COLUMNS
        WHERE TABLE_SCHEMA = DATABASE()
          AND TABLE_NAME = 'relationships'
          AND COLUMN_NAME = 'forward_cardinality'
    ) = 0,
    'ALTER TABLE relationships ADD COLUMN forward_cardinality VARCHAR(4) NOT NULL DEFAULT ''MANY''',
    'SELECT 1'
);
PREPARE library_relation_column_stmt
    FROM @library_relation_forward_cardinality_ddl;
EXECUTE library_relation_column_stmt;
DEALLOCATE PREPARE library_relation_column_stmt;

SET @library_relation_reverse_cardinality_ddl = IF(
    (
        SELECT COUNT(*)
        FROM information_schema.COLUMNS
        WHERE TABLE_SCHEMA = DATABASE()
          AND TABLE_NAME = 'relationships'
          AND COLUMN_NAME = 'reverse_cardinality'
    ) = 0,
    'ALTER TABLE relationships ADD COLUMN reverse_cardinality VARCHAR(4) NOT NULL DEFAULT ''MANY''',
    'SELECT 1'
);
PREPARE library_relation_column_stmt
    FROM @library_relation_reverse_cardinality_ddl;
EXECUTE library_relation_column_stmt;
DEALLOCATE PREPARE library_relation_column_stmt;

SET @library_relation_inverse_field_ddl = IF(
    (
        SELECT COUNT(*)
        FROM information_schema.COLUMNS
        WHERE TABLE_SCHEMA = DATABASE()
          AND TABLE_NAME = 'relationships'
          AND COLUMN_NAME = 'inverse_field_id'
    ) = 0,
    'ALTER TABLE relationships ADD COLUMN inverse_field_id VARCHAR(31) NULL',
    'SELECT 1'
);
PREPARE library_relation_column_stmt
    FROM @library_relation_inverse_field_ddl;
EXECUTE library_relation_column_stmt;
DEALLOCATE PREPARE library_relation_column_stmt;

SET @library_relation_target_delete_ddl = IF(
    (
        SELECT COUNT(*)
        FROM information_schema.COLUMNS
        WHERE TABLE_SCHEMA = DATABASE()
          AND TABLE_NAME = 'relationships'
          AND COLUMN_NAME = 'on_target_delete'
    ) = 0,
    'ALTER TABLE relationships ADD COLUMN on_target_delete VARCHAR(8) NOT NULL DEFAULT ''RESTRICT''',
    'SELECT 1'
);
PREPARE library_relation_column_stmt
    FROM @library_relation_target_delete_ddl;
EXECUTE library_relation_column_stmt;
DEALLOCATE PREPARE library_relation_column_stmt;
