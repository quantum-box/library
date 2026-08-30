-- Per-repo glossary: terms whose translation the owner fixes.
--
-- Matters more here than it would with a large model. The pipeline runs
-- on a small, cheap model, and small models are markedly worse at
-- inferring that a word is a product name, an internal codename or a
-- term of art rather than an ordinary noun. On a public repo that
-- inconsistency is what a reader sees.
--
-- No CHARSET/COLLATE clause, for the reasons given in
-- 20260830000000_create_translations.up.sql: an explicit collation
-- breaks the foreign key to `repos`, and a binary collation would make
-- sqlx decode these columns as bytes.
CREATE TABLE IF NOT EXISTS `repo_glossary_terms` (
    `repo_id`     VARCHAR(29)  NOT NULL COMMENT 'Repo ID (rp_)',
    `term`        VARCHAR(255) NOT NULL
                  COMMENT 'Source term as written by the author',
    `target_lang` VARCHAR(16)  NOT NULL DEFAULT '*'
                  COMMENT 'BCP-47 tag, or `*` for every target language',
    `translation` VARCHAR(255) NOT NULL
                  COMMENT 'What the term must become; equal to `term` to leave it alone',
    `created_at`  TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    `updated_at`  TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP
                  ON UPDATE CURRENT_TIMESTAMP,
    -- `*` rather than NULL as the every-language sentinel: MySQL will
    -- not take a nullable column in a primary key, and a unique index
    -- treats NULLs as distinct, which would let duplicates through.
    -- `*` is not a valid language tag, so it cannot collide with one.
    PRIMARY KEY (`repo_id`, `term`, `target_lang`),
    CONSTRAINT `fk_repo_glossary_terms_repo` FOREIGN KEY (`repo_id`)
        REFERENCES `repos` (`id`) ON DELETE CASCADE
)
COMMENT='Owner-fixed term translations, injected into the model prompt';
