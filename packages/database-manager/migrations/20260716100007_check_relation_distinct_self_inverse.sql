ALTER TABLE relationships
    ADD CONSTRAINT chk_relationships_distinct_self_inverse
        CHECK (
            inverse_field_id IS NULL
            OR object_id <> target_object_id
            OR inverse_field_id <> field_id
        );
