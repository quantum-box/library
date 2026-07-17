ALTER TABLE relationships
    ADD CONSTRAINT chk_relationships_on_target_delete_strict
        CHECK (CAST(on_target_delete AS BINARY) IN ('RESTRICT', 'NULLIFY'));
