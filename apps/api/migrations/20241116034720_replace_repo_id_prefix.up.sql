-- Add up migration script here


UPDATE library.repos
SET id = REPLACE(id, 'tn_', 'rp_')
WHERE id LIKE 'tn_%';
