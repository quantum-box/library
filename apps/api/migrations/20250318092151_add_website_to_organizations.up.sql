-- Add website column to organizations table
ALTER TABLE library.organizations
ADD COLUMN website VARCHAR(2048) NULL;
