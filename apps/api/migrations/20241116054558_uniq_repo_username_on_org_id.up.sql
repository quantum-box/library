-- Add up migration script here
ALTER TABLE library.repos
ADD CONSTRAINT uniq_repo_username_on_org_id UNIQUE (username, org_id);
