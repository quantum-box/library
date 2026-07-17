ALTER TABLE relationships
    ADD CONSTRAINT uq_relationships_tenant_source_field
        UNIQUE (tenant_id, object_id, field_id);
