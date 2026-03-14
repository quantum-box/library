-- Add up migration script here
ALTER TABLE library.repos
ADD COLUMN username VARCHAR(255) NOT NULL DEFAULT '';
UPDATE library.repos SET username = name;
ALTER TABLE library.repos MODIFY COLUMN username VARCHAR(255) NOT NULL;


-- add org_id
ALTER TABLE library.repos
ADD COLUMN org_id VARCHAR(29) NOT NULL DEFAULT '';

-- Update org_id with parent_tenant_id from tachyon_app_auth.tenants
UPDATE library.repos
SET org_id = (
    SELECT parent_tenant_id
    FROM tachyon_apps_auth.tenants
    WHERE tachyon_apps_auth.tenants.id = library.repos.id
);

-- REMOVED: This DELETE statement was causing issues by removing tenant data
-- during migration. The operation was a one-time cleanup that should not
-- be re-executed on fresh databases.
-- DELETE FROM tachyon_apps_auth.tenants
-- WHERE id IN (SELECT id FROM library.repos);
