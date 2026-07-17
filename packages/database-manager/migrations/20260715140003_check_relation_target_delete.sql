ALTER TABLE relationships
    ADD CONSTRAINT chk_relationships_on_target_delete
        CHECK (on_target_delete IN ('RESTRICT', 'NULLIFY'));
