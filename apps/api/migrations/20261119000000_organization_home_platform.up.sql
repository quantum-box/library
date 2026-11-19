-- The tachyon platform an organization's tenant belongs to. Library-created
-- organizations live on Library's platform; organizations adopted from
-- tachyon keep their own. NULL marks rows stored before the platform was
-- recorded: library-api fills them on first use (B1 in the
-- library-org-platform-authority design), then this becomes NOT NULL.
-- `platform_id` is unrelated: it is the Library partition and always
-- Library's tenant.
ALTER TABLE library.organizations
ADD COLUMN home_platform_id VARCHAR(29) NULL;
