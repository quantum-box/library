-- Reject malformed canonical envelopes at the storage boundary. This remains
-- expand-only: legacy value0..value50 columns are unchanged.
ALTER TABLE property_values
    ADD CONSTRAINT chk_property_values_type_key
        CHECK (REGEXP_LIKE(
            type_key,
            '^[a-z][a-z0-9]*(_[a-z0-9]+)*$',
            'c'
        )),
    ADD CONSTRAINT chk_property_values_type_version
        CHECK (type_version > 0),
    ADD CONSTRAINT chk_property_values_encoding_version
        CHECK (value_encoding_version > 0),
    ADD CONSTRAINT chk_property_values_value_json
        CHECK (JSON_VALID(value));
