ALTER TABLE data
    ADD CONSTRAINT chk_data_record_version_nonzero
        CHECK (record_version > 0);
