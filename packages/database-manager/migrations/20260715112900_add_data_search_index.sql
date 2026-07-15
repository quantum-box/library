CREATE INDEX idx_data_tenant_object_name_id
    ON data (tenant_id, object_id, name, id);
