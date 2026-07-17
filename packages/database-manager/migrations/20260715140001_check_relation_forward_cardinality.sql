ALTER TABLE relationships
    ADD CONSTRAINT chk_relationships_forward_cardinality
        CHECK (forward_cardinality IN ('ONE', 'MANY'));
