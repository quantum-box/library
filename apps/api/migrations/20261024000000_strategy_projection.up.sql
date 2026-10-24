-- Dedicated tables deliberately have no database-manager record projection:
-- external payload must not enter anonymous search, export or offline sync.
CREATE TABLE IF NOT EXISTS library_pm_bindings (
  repo_id VARCHAR(64) NOT NULL,
  tenant_id VARCHAR(64) NOT NULL,
  subscription_id VARCHAR(255) NOT NULL,
  workspace_id VARCHAR(255) NOT NULL,
  generation BIGINT UNSIGNED NOT NULL,
  checkpoint BIGINT UNSIGNED NOT NULL DEFAULT 0,
  cursor_value TEXT NULL,
  writer VARCHAR(16) NOT NULL DEFAULT 'paused',
  PRIMARY KEY(repo_id),
  UNIQUE KEY subscription_owner(tenant_id, subscription_id)
);
CREATE TABLE IF NOT EXISTS library_pm_events (
  repo_id VARCHAR(64) NOT NULL,
  generation BIGINT UNSIGNED NOT NULL,
  sequence BIGINT UNSIGNED NOT NULL,
  event_id VARCHAR(255) NOT NULL,
  payload_hash CHAR(64) NOT NULL,
  PRIMARY KEY(repo_id,generation,sequence),
  UNIQUE KEY unique_event(repo_id,generation,event_id)
);
CREATE TABLE IF NOT EXISTS library_pm_records (
  repo_id VARCHAR(64) NOT NULL,
  resource_key CHAR(64) NOT NULL,
  id VARCHAR(64) NOT NULL,
  resource_ref JSON NOT NULL,
  projection JSON NULL,
  observed_at VARCHAR(64) NOT NULL,
  observed_revision BIGINT UNSIGNED NOT NULL,
  lifecycle VARCHAR(32) NOT NULL,
  local_note MEDIUMTEXT NOT NULL,
  document_ids JSON NOT NULL,
  legacy_data_id VARCHAR(64) NULL,
  PRIMARY KEY(repo_id, resource_key),
  UNIQUE KEY stable_record(repo_id,id)
);
CREATE TABLE IF NOT EXISTS library_pm_relations (
  repo_id VARCHAR(64) NOT NULL,
  source_key CHAR(64) NOT NULL,
  target_key CHAR(64) NOT NULL,
  relation_value JSON NOT NULL,
  PRIMARY KEY(repo_id,source_key,target_key)
);
CREATE TABLE IF NOT EXISTS library_document_releases (
  repo_id VARCHAR(64) NOT NULL,
  id VARCHAR(64) NOT NULL,
  data_id VARCHAR(64) NOT NULL,
  source_version VARCHAR(64) NOT NULL,
  title TEXT NOT NULL,
  body MEDIUMTEXT NOT NULL,
  published_by VARCHAR(255) NOT NULL,
  published_at VARCHAR(64) NOT NULL,
  revoked BOOLEAN NOT NULL DEFAULT FALSE,
  PRIMARY KEY(repo_id,id),
  UNIQUE KEY one_source_version(repo_id,data_id,source_version)
);
CREATE TABLE IF NOT EXISTS library_pm_writer_fences (
  repo_id VARCHAR(64) NOT NULL PRIMARY KEY,
  writer VARCHAR(16) NOT NULL DEFAULT 'legacy'
);
