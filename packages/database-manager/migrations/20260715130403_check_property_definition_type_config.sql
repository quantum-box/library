-- TiDB supports only one added CHECK constraint per ALTER TABLE statement.
ALTER TABLE fields
    ADD CONSTRAINT ck_fields_property_definition_type_config
        CHECK (type_config IS NULL OR JSON_VALID(type_config));
