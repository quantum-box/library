-- Add down migration script here
UPDATE library.repos
SET id = REPLACE(id, 'rp_', 'tn_')
WHERE id LIKE 'rp_%';
