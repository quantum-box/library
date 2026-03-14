-- Add up migration script here
ALTER TABLE repos ADD COLUMN org_username VARCHAR(255) NOT NULL;