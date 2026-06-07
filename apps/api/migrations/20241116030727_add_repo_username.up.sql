-- Add up migration script here
ALTER TABLE library.repos
ADD COLUMN IF NOT EXISTS username VARCHAR(255) NOT NULL DEFAULT '';
UPDATE library.repos SET username = name WHERE username = '';
ALTER TABLE library.repos MODIFY COLUMN username VARCHAR(255) NOT NULL;


-- add org_id
ALTER TABLE library.repos
ADD COLUMN IF NOT EXISTS org_id VARCHAR(29) NOT NULL DEFAULT '';

-- org_id left as empty default; tachyon_apps_auth is a separate DB not available here
