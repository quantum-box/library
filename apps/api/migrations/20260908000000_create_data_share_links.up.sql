-- Read-only share links for a single document in a private repo.
--
-- A private repo is otherwise unreachable without a Tachyon identity:
-- `VisibilityService::check_access` refuses an anonymous caller before
-- any policy is consulted. A share link is the one exception, and it is
-- deliberately the narrowest one -- it names exactly one `data_id`, so a
-- leaked link exposes that document and nothing else in the repo.
--
-- The row stores only the SHA-256 of the token. The secret itself is
-- returned once, at creation, and is unrecoverable afterwards: a dump of
-- this table cannot be turned back into working links.
--
-- No CHARSET/COLLATE clause, for the reasons recorded at the top of
-- 20260830000000_create_translations.up.sql: an explicit collation on
-- `repo_id` makes MySQL reject the foreign key to `repos`, and a binary
-- collation would make sqlx decode these columns as bytes.
CREATE TABLE IF NOT EXISTS `data_share_links` (
    `id`         VARCHAR(29)  NOT NULL COMMENT 'Share link ID (sl_)',
    `token_hash` CHAR(64)     NOT NULL
                 COMMENT 'Lowercase hex SHA-256 of the secret; the secret is never stored',
    `repo_id`    VARCHAR(29)  NOT NULL COMMENT 'Repo ID (rp_)',
    -- Wider than the 31 characters `data_` + ULID needs, matching the
    -- width `sync_states.data_id` was widened to in
    -- 20260115000000_expand_sync_states_data_id.up.sql.
    `data_id`    VARCHAR(64)  NOT NULL COMMENT 'Library data ID (data_)',
    `name`       VARCHAR(255) NULL
                 COMMENT 'Label the creator gave the link, for telling several apart',
    `created_by` VARCHAR(64)  NULL
                 COMMENT 'Executor ID that minted the link; NULL once that identity is gone',
    `created_at` TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    -- Revocation is a timestamp rather than a DELETE so that a revoked
    -- link stays listable: "this link no longer works" is the answer the
    -- owner needs, and a missing row cannot give it.
    `revoked_at` TIMESTAMP    NULL DEFAULT NULL,
    PRIMARY KEY (`id`),
    -- The lookup on the read path is by hash alone, and it must be
    -- unique: two rows sharing a hash would make the token ambiguous.
    UNIQUE KEY `uk_data_share_links_token_hash` (`token_hash`),
    KEY `idx_data_share_links_data` (`repo_id`, `data_id`),
    CONSTRAINT `fk_data_share_links_repo` FOREIGN KEY (`repo_id`)
        REFERENCES `repos` (`id`) ON DELETE CASCADE
)
COMMENT='Unguessable read-only links to one document in a private repo';
