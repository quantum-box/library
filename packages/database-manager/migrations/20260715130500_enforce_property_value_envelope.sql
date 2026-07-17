-- Reject malformed canonical envelopes at the storage boundary. This remains
-- expand-only: legacy value0..value50 columns are unchanged.
ALTER TABLE property_values
    ADD CONSTRAINT chk_property_values_type_key
        CHECK (REGEXP_LIKE(
            type_key,
            '^[a-z][a-z0-9]*(_[a-z0-9]+)*$',
            'c'
        ));
