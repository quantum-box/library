ALTER TABLE relationships
    ADD CONSTRAINT chk_relationships_owned_inverse
        CHECK (inverse_owned = FALSE OR inverse_field_id IS NOT NULL);
