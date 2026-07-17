ALTER TABLE relationships
    ADD CONSTRAINT chk_relationships_forward_cardinality_strict
        CHECK (CAST(forward_cardinality AS BINARY) IN ('ONE', 'MANY'));
