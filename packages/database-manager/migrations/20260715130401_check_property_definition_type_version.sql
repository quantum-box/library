-- TiDB supports only one added CHECK constraint per ALTER TABLE statement.
ALTER TABLE fields
    ADD CONSTRAINT ck_fields_property_definition_type_version
        CHECK (type_version IS NULL OR type_version > 0);
