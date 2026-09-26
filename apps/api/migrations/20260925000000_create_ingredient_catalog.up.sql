-- COM-860: common ingredient master (food composition).
--
-- Ingredients, nutrient definitions and values are edited as ordinary
-- Library data in three repos. These tables hold what publishing freezes:
-- the catalog that names the three repos, and one immutable copy of every
-- row per release. The application has no UPDATE or DELETE path for the
-- release tables, so a published release keeps returning the same values
-- after the drafts change.
--
-- No foreign key to `repos`: deleting a draft repo must not be blocked
-- by, or cascade into, releases that consumers still reference.
--
-- Like the translation tables, no table names a CHARSET or COLLATE; see
-- 20260830000000_create_translations.up.sql for why.

CREATE TABLE IF NOT EXISTS `ingredient_catalogs` (
    `id`                 VARCHAR(32)  NOT NULL COMMENT 'Catalog ID (icat_)',
    `tenant_id`          VARCHAR(29)  NOT NULL COMMENT 'Owning org (tn_)',
    `catalog_key`        VARCHAR(64)  NOT NULL COMMENT 'URL-safe key, unique per org',
    `name`               VARCHAR(255) NOT NULL,
    `ingredient_repo_id` VARCHAR(29)  NOT NULL COMMENT 'Draft ingredients (rp_)',
    `nutrient_repo_id`   VARCHAR(29)  NOT NULL COMMENT 'Draft nutrient definitions (rp_)',
    `value_repo_id`      VARCHAR(29)  NOT NULL COMMENT 'Draft ingredient x nutrient values (rp_)',
    `created_at`         TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    `updated_at`         TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    PRIMARY KEY (`id`),
    UNIQUE KEY `uk_ingredient_catalogs_tenant_key` (`tenant_id`, `catalog_key`)
)
COMMENT='COM-860: an ingredient catalog and its three draft repos';

CREATE TABLE IF NOT EXISTS `ingredient_releases` (
    `id`                  VARCHAR(32)   NOT NULL COMMENT 'Release ID (irel_)',
    `tenant_id`           VARCHAR(29)   NOT NULL,
    `catalog_id`          VARCHAR(32)   NOT NULL,
    `source_id`           VARCHAR(128)  NOT NULL COMMENT 'Dataset, e.g. mext-sfct-2023',
    `source_release`      VARCHAR(128)  NOT NULL COMMENT 'Edition incl. errata',
    `source_url`          VARCHAR(2048) NULL,
    `source_retrieved_at` TIMESTAMP     NULL,
    `notes`               TEXT          NULL,
    `schema_version`      INT UNSIGNED  NOT NULL,
    `content_hash`        VARCHAR(80)   NOT NULL COMMENT 'sha256:<hex> of the frozen rows',
    `ingredient_count`    INT UNSIGNED  NOT NULL,
    `nutrient_count`      INT UNSIGNED  NOT NULL,
    `value_count`         INT UNSIGNED  NOT NULL,
    `private_repo_mask`   TINYINT UNSIGNED NOT NULL COMMENT 'Private draft repos at publish: bit 0 ingredients, bit 1 nutrients, bit 2 values',
    `published_by`        VARCHAR(64)   NOT NULL,
    `published_at`        TIMESTAMP     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (`id`),
    UNIQUE KEY `uk_ingredient_releases_source`
        (`tenant_id`, `catalog_id`, `source_id`, `source_release`),
    KEY `idx_ingredient_releases_catalog` (`tenant_id`, `catalog_id`, `published_at`),
    CONSTRAINT `fk_ingredient_releases_catalog` FOREIGN KEY (`catalog_id`)
        REFERENCES `ingredient_catalogs` (`id`)
)
COMMENT='COM-860: immutable published release of an ingredient catalog';

CREATE TABLE IF NOT EXISTS `ingredient_release_nutrients` (
    `release_id`      VARCHAR(32)  NOT NULL,
    `nutrient_key`    VARCHAR(64)  NOT NULL COMMENT 'e.g. ENERC_KCAL, PROT-',
    `name`            VARCHAR(255) NOT NULL,
    `unit`            VARCHAR(255) NOT NULL,
    `basis`           VARCHAR(255) NOT NULL COMMENT 'e.g. per 100 g edible portion',
    `method`          VARCHAR(255) NULL,
    `display_order`   INT          NOT NULL,
    `default_display` BOOLEAN      NOT NULL,
    PRIMARY KEY (`release_id`, `nutrient_key`),
    CONSTRAINT `fk_ingredient_release_nutrients_release` FOREIGN KEY (`release_id`)
        REFERENCES `ingredient_releases` (`id`)
)
COMMENT='COM-860: nutrient definitions frozen in a release';

CREATE TABLE IF NOT EXISTS `ingredient_release_items` (
    `release_id`              VARCHAR(32)  NOT NULL,
    `ingredient_key`          VARCHAR(64)  NOT NULL COMMENT 'Stable across releases',
    `source_food_code`        VARCHAR(64)  NOT NULL COMMENT 'Text: keeps leading zeros',
    `original_name`           VARCHAR(255) NOT NULL COMMENT 'Food name as in the source',
    `standard_name`           VARCHAR(255) NULL,
    `reading`                 VARCHAR(255) NULL,
    `category_code`           VARCHAR(255) NULL,
    `category_name`           VARCHAR(255) NULL,
    `part`                    VARCHAR(255) NULL,
    `cooking_state`           VARCHAR(32)  NULL,
    `skin_bone`               VARCHAR(255) NULL,
    `refuse_rate`             VARCHAR(38)  NULL COMMENT 'Normalized decimal percent; not store food loss',
    `attribute_review_status` VARCHAR(16)  NOT NULL,
    PRIMARY KEY (`release_id`, `ingredient_key`),
    UNIQUE KEY `uk_ingredient_release_items_food_code` (`release_id`, `source_food_code`),
    KEY `idx_ingredient_release_items_category` (`release_id`, `category_code`),
    KEY `idx_ingredient_release_items_state` (`release_id`, `cooking_state`),
    CONSTRAINT `fk_ingredient_release_items_release` FOREIGN KEY (`release_id`)
        REFERENCES `ingredient_releases` (`id`)
)
COMMENT='COM-860: ingredients frozen in a release';

CREATE TABLE IF NOT EXISTS `ingredient_release_aliases` (
    `release_id`     VARCHAR(32)  NOT NULL,
    `ingredient_key` VARCHAR(64)  NOT NULL,
    `alias`          VARCHAR(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_bin NOT NULL,
    PRIMARY KEY (`release_id`, `ingredient_key`, `alias`),
    CONSTRAINT `fk_ingredient_release_aliases_item` FOREIGN KEY (`release_id`, `ingredient_key`)
        REFERENCES `ingredient_release_items` (`release_id`, `ingredient_key`)
)
COMMENT='COM-860: curated aliases frozen in a release, used for search';

CREATE TABLE IF NOT EXISTS `ingredient_release_values` (
    `release_id`     VARCHAR(32)  NOT NULL,
    `ingredient_key` VARCHAR(64)  NOT NULL,
    `nutrient_key`   VARCHAR(64)  NOT NULL,
    `value_status`   VARCHAR(32)  NOT NULL COMMENT 'measured, estimated, zero, trace, not_measured, ...',
    `amount`         VARCHAR(38)  NULL COMMENT 'Normalized decimal; NULL for trace and not measured',
    `raw_notation`   VARCHAR(255) NULL COMMENT 'Cell text as in the source',
    PRIMARY KEY (`release_id`, `ingredient_key`, `nutrient_key`),
    CONSTRAINT `fk_ingredient_release_values_item` FOREIGN KEY (`release_id`, `ingredient_key`)
        REFERENCES `ingredient_release_items` (`release_id`, `ingredient_key`),
    CONSTRAINT `fk_ingredient_release_values_nutrient` FOREIGN KEY (`release_id`, `nutrient_key`)
        REFERENCES `ingredient_release_nutrients` (`release_id`, `nutrient_key`),
    CONSTRAINT `chk_ingredient_release_values_amount` CHECK (
        (`value_status` IN ('measured', 'estimated', 'zero', 'estimated_zero') AND `amount` IS NOT NULL)
        OR (`value_status` IN ('trace', 'estimated_trace', 'not_measured') AND `amount` IS NULL)
    )
)
COMMENT='COM-860: ingredient x nutrient values frozen in a release';
