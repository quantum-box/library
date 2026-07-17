ALTER TABLE relationships
    ADD CONSTRAINT fk_relationships_tenant_source_property_restrict
        FOREIGN KEY (tenant_id, object_id, field_id)
        REFERENCES fields (tenant_id, object_id, id)
        ON DELETE RESTRICT;
