-- Derived translation storage for Library user data (Tier 1-3).
--
-- Expand-only: the original values in the database-manager schema are never
-- written by this feature. Every row here is a cache keyed by the hash of the
-- source text, so the whole set can be dropped and regenerated at any time.
--
-- None of these tables names a CHARSET or COLLATE, so every column
-- carries the database default. Two reasons, both learned the hard way:
-- `repos` was created without one, and giving `repo_id` an explicit
-- collation makes MySQL 8 reject the foreign key (errno 3780); and a
-- binary collation such as `ascii_bin` makes sqlx decode the column as
-- `Vec<u8>` rather than `String`, which infects every read site.
--
-- These tables live in the `library` database rather than
-- `tachyon_apps_database_manager` because translation is a Library product
-- concern (public repos, published languages) and because production keeps the
-- two as separate physical databases -- a row here has to be joinable with
-- `repos`, and cannot be joined with `data` or `fields` either way.

-- The set of languages a repo owner has declared for public reading.
--
-- This doubles as the allow-list for anonymous requests: a `lang` absent here
-- never enqueues a translation job, so a public endpoint cannot be used to
-- drive LLM spend.
CREATE TABLE IF NOT EXISTS `repo_published_languages` (
    `repo_id`    VARCHAR(29) NOT NULL COMMENT 'Repo ID (rp_)',
    `lang`       VARCHAR(16) NOT NULL
                 COMMENT 'BCP-47 language tag, normalized (ja, en, zh-Hans)',
    `enabled_at` TIMESTAMP   NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (`repo_id`, `lang`),
    CONSTRAINT `fk_repo_published_languages_repo` FOREIGN KEY (`repo_id`)
        REFERENCES `repos` (`id`) ON DELETE CASCADE
)
-- Deliberately no CHARSET/COLLATE clause; see the note at the top of
-- this file.
COMMENT='Languages a repo is published in; allow-list for anonymous reads';

-- One cached translation per (target, language).
--
-- No foreign key to `data` or `fields`: they live in the other physical
-- database. A row whose target disappears is harmless -- it is never read
-- again and can be swept lazily.
CREATE TABLE IF NOT EXISTS `translations` (
    `tenant_id`   VARCHAR(29) NOT NULL
                  COMMENT 'Tenant ID (tn_)',
    `scope`       VARCHAR(16) NOT NULL
                  COMMENT 'DATABASE / PROPERTY_DEF / SELECT_OPTION / RECORD_NAME / PROPERTY_VALUE',
    `target_id`   VARCHAR(72) NOT NULL
                  COMMENT 'Scope-dependent key; PROPERTY_VALUE uses "{data_id}:{property_id}"',
    `target_lang` VARCHAR(16) NOT NULL,
    `source_lang` VARCHAR(16) NULL
                  COMMENT 'NULL when detection could not settle it (Latin script)',
    `source_hash` CHAR(64)    NOT NULL
                  COMMENT 'SHA-256 of the source text; a mismatch means stale',
    `translated`  LONGTEXT    NULL
                  COMMENT 'NULL while PENDING or FAILED',
    `status`      VARCHAR(8)  NOT NULL
                  DEFAULT 'PENDING'
                  COMMENT 'FRESH / STALE / PENDING / FAILED',
    `model`       VARCHAR(64) NULL
                  COMMENT 'Model that produced this row; part of the ETag',
    `reviewed_by` VARCHAR(64) NULL
                  COMMENT 'Set once a human edited the translation; never auto-overwritten',
    `created_at`  TIMESTAMP   NOT NULL DEFAULT CURRENT_TIMESTAMP,
    `updated_at`  TIMESTAMP   NOT NULL DEFAULT CURRENT_TIMESTAMP
                  ON UPDATE CURRENT_TIMESTAMP,
    PRIMARY KEY (`tenant_id`, `scope`, `target_id`, `target_lang`),
    INDEX `idx_translations_progress` (`tenant_id`, `target_lang`, `status`),
    INDEX `idx_translations_scope` (`tenant_id`, `scope`, `target_lang`, `status`)
)
COMMENT='Derived translations of user data, keyed by source content hash';

-- Per-property override of the type-derived translatability default.
--
-- The property type decides the default (String/Markdown/RichText/Html are
-- candidates, Id/Date/Integer/Location/Relation/Image never are), but a String
-- column holding part numbers or person names must not be translated. Only the
-- schema owner can know that, so the override lives here rather than being
-- inferred.
CREATE TABLE IF NOT EXISTS `property_translation_settings` (
    `tenant_id`    VARCHAR(29) NOT NULL,
    `property_id`  VARCHAR(31) NOT NULL
                   COMMENT 'Property definition ID in the database-manager schema',
    `database_id`  VARCHAR(29) NOT NULL
                   COMMENT 'Denormalized for per-database sweeps',
    `translatable` BOOLEAN     NOT NULL,
    `created_at`   TIMESTAMP   NOT NULL DEFAULT CURRENT_TIMESTAMP,
    `updated_at`   TIMESTAMP   NOT NULL DEFAULT CURRENT_TIMESTAMP
                   ON UPDATE CURRENT_TIMESTAMP,
    PRIMARY KEY (`tenant_id`, `property_id`),
    INDEX `idx_property_translation_settings_database`
        (`tenant_id`, `database_id`)
)
COMMENT='Owner override of the type-derived translatable default';
