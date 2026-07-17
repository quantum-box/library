ALTER TABLE relationships
    ADD COLUMN definition_version SMALLINT UNSIGNED NOT NULL DEFAULT 1
        AFTER on_target_delete;
