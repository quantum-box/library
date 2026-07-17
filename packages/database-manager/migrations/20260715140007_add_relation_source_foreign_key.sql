-- A RelationDefinition is owned by its source Property. Deleting that
-- Property removes only the definition; target Database deletion remains
-- restricted while a definition points at it.
ALTER TABLE relationships
    ADD CONSTRAINT fk_relationships_tenant_source_property
        FOREIGN KEY (tenant_id, object_id, field_id)
        REFERENCES fields (tenant_id, object_id, id)
        ON DELETE CASCADE;
