ALTER TABLE relationships
    ADD CONSTRAINT chk_relationships_generation
        CHECK (generation > 0);
