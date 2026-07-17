ALTER TABLE relationships
    ADD CONSTRAINT chk_relationships_reverse_cardinality
        CHECK (reverse_cardinality IN ('ONE', 'MANY'));
