ALTER TABLE relationships
    ADD COLUMN generation BIGINT UNSIGNED NOT NULL DEFAULT 1
        AFTER definition_version;
