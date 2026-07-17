ALTER TABLE relationships
    ADD CONSTRAINT chk_relationships_reverse_cardinality_strict
        CHECK (CAST(reverse_cardinality AS BINARY) IN ('ONE', 'MANY'));
