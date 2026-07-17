ALTER TABLE relationships
    ADD CONSTRAINT chk_relationships_definition_version
        CHECK (definition_version > 0);
