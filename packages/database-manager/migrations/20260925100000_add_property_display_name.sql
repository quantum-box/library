ALTER TABLE fields
    ADD COLUMN field_display_name VARCHAR(255) NOT NULL DEFAULT '';

UPDATE fields
SET field_display_name = field_name
WHERE field_display_name = '';
