-- TiDB supports only one added CHECK constraint per ALTER TABLE statement.
ALTER TABLE property_values
    ADD CONSTRAINT chk_property_values_value_json
        CHECK (JSON_VALID(value));
