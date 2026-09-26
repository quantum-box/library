//! COM-860: SQLx implementation of IngredientCatalogRepository.
//!
//! Catalogs and releases are tenant-scoped; release rows are reached only
//! through a release that was already loaded for the caller's tenant.
//! Releases are insert-only: nothing here updates or deletes them.

use std::collections::BTreeMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use sqlx::{FromRow, MySql, QueryBuilder};

use crate::domain::{
    AttributeReviewStatus, IngredientCatalog, IngredientCatalogId,
    IngredientCatalogRepository, IngredientRelease, IngredientReleaseId,
    IngredientSearch, NormalizedDecimal, NutrientValueStatus,
    ReleaseSnapshot, ReleaseSource, ReleasedIngredient, ReleasedNutrient,
    ReleasedValue,
};
use value_object::TenantId;

/// Rows per multi-row INSERT. Keeps each statement well under
/// `max_allowed_packet` for a full food composition table.
const INSERT_CHUNK: usize = 500;

fn invalid<E: std::fmt::Display>(e: E) -> errors::Error {
    errors::Error::invalid(e.to_string())
}

fn db_error(e: sqlx::Error) -> errors::Error {
    match e {
        sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
            errors::Error::conflict(format!(
                "ingredient catalog unique constraint violated: {db_err}"
            ))
        }
        e => errors::Error::internal_server_error(e),
    }
}

#[derive(Debug, FromRow)]
struct CatalogRow {
    id: String,
    tenant_id: String,
    catalog_key: String,
    name: String,
    ingredient_repo_id: String,
    nutrient_repo_id: String,
    value_repo_id: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<CatalogRow> for IngredientCatalog {
    type Error = errors::Error;

    fn try_from(row: CatalogRow) -> Result<Self, Self::Error> {
        Ok(IngredientCatalog::new(
            row.id.parse().map_err(invalid)?,
            row.tenant_id.parse().map_err(invalid)?,
            row.catalog_key,
            row.name,
            row.ingredient_repo_id.parse().map_err(invalid)?,
            row.nutrient_repo_id.parse().map_err(invalid)?,
            row.value_repo_id.parse().map_err(invalid)?,
            row.created_at,
            row.updated_at,
        ))
    }
}

#[derive(Debug, FromRow)]
struct ReleaseRow {
    id: String,
    tenant_id: String,
    catalog_id: String,
    source_id: String,
    source_release: String,
    source_url: Option<String>,
    source_retrieved_at: Option<DateTime<Utc>>,
    notes: Option<String>,
    schema_version: u32,
    content_hash: String,
    ingredient_count: u32,
    nutrient_count: u32,
    value_count: u32,
    private_repo_mask: u8,
    published_by: String,
    published_at: DateTime<Utc>,
}

impl TryFrom<ReleaseRow> for IngredientRelease {
    type Error = errors::Error;

    fn try_from(row: ReleaseRow) -> Result<Self, Self::Error> {
        Ok(IngredientRelease::new(
            row.id.parse().map_err(invalid)?,
            row.tenant_id.parse().map_err(invalid)?,
            row.catalog_id.parse().map_err(invalid)?,
            ReleaseSource {
                source_id: row.source_id,
                source_release: row.source_release,
                source_url: row.source_url,
                source_retrieved_at: row.source_retrieved_at,
                notes: row.notes,
            },
            row.schema_version,
            row.content_hash,
            row.ingredient_count,
            row.nutrient_count,
            row.value_count,
            row.private_repo_mask,
            row.published_by,
            row.published_at,
        ))
    }
}

#[derive(Debug, FromRow)]
struct ItemRow {
    ingredient_key: String,
    source_food_code: String,
    original_name: String,
    standard_name: Option<String>,
    reading: Option<String>,
    category_code: Option<String>,
    category_name: Option<String>,
    part: Option<String>,
    cooking_state: Option<String>,
    skin_bone: Option<String>,
    refuse_rate: Option<String>,
    attribute_review_status: String,
}

impl TryFrom<ItemRow> for ReleasedIngredient {
    type Error = errors::Error;

    fn try_from(row: ItemRow) -> Result<Self, Self::Error> {
        Ok(ReleasedIngredient {
            ingredient_key: row.ingredient_key,
            source_food_code: row.source_food_code,
            original_name: row.original_name,
            standard_name: row.standard_name,
            reading: row.reading,
            aliases: Vec::new(),
            category_code: row.category_code,
            category_name: row.category_name,
            part: row.part,
            cooking_state: row.cooking_state,
            skin_bone: row.skin_bone,
            refuse_rate: row
                .refuse_rate
                .as_deref()
                .map(NormalizedDecimal::parse)
                .transpose()?,
            attribute_review_status: AttributeReviewStatus::parse(
                &row.attribute_review_status,
            )?,
        })
    }
}

#[derive(Debug, FromRow)]
struct AliasRow {
    ingredient_key: String,
    alias: String,
}

#[derive(Debug, FromRow)]
struct NutrientRow {
    nutrient_key: String,
    name: String,
    unit: String,
    basis: String,
    method: Option<String>,
    display_order: i32,
    default_display: bool,
}

impl From<NutrientRow> for ReleasedNutrient {
    fn from(row: NutrientRow) -> Self {
        Self {
            nutrient_key: row.nutrient_key,
            name: row.name,
            unit: row.unit,
            basis: row.basis,
            method: row.method,
            display_order: row.display_order,
            default_display: row.default_display,
        }
    }
}

#[derive(Debug, FromRow)]
struct ValueRow {
    ingredient_key: String,
    nutrient_key: String,
    value_status: String,
    amount: Option<String>,
    raw_notation: Option<String>,
}

impl TryFrom<ValueRow> for ReleasedValue {
    type Error = errors::Error;

    fn try_from(row: ValueRow) -> Result<Self, Self::Error> {
        Ok(ReleasedValue {
            ingredient_key: row.ingredient_key,
            nutrient_key: row.nutrient_key,
            status: row.value_status.parse::<NutrientValueStatus>()?,
            amount: row
                .amount
                .as_deref()
                .map(NormalizedDecimal::parse)
                .transpose()?,
            raw_notation: row.raw_notation,
        })
    }
}

const CATALOG_COLUMNS: &str = "`id`, `tenant_id`, `catalog_key`, `name`, \
    `ingredient_repo_id`, `nutrient_repo_id`, `value_repo_id`, \
    `created_at`, `updated_at`";

const RELEASE_COLUMNS: &str = "`id`, `tenant_id`, `catalog_id`, \
    `source_id`, `source_release`, `source_url`, `source_retrieved_at`, \
    `notes`, `schema_version`, `content_hash`, `ingredient_count`, \
    `nutrient_count`, `value_count`, `private_repo_mask`, \
    `published_by`, `published_at`";

/// Item columns. Aliases are loaded in a separate bounded query.
const ITEM_SELECT: &str =
    "SELECT i.`ingredient_key`, i.`source_food_code`, \
    i.`original_name`, i.`standard_name`, i.`reading`, i.`category_code`, \
    i.`category_name`, i.`part`, i.`cooking_state`, i.`skin_bone`, \
    i.`refuse_rate`, i.`attribute_review_status` \
    FROM `ingredient_release_items` i";

/// Escape `%`, `_` and `\` so user text is matched literally by LIKE.
fn like_contains(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len() + 2);
    escaped.push('%');
    for c in text.chars() {
        if matches!(c, '%' | '_' | '\\') {
            escaped.push('\\');
        }
        escaped.push(c);
    }
    escaped.push('%');
    escaped
}

/// Append the WHERE clause shared by the page query and the count query.
fn push_search_filters(
    qb: &mut QueryBuilder<'_, MySql>,
    release_id: &str,
    search: &IngredientSearch,
) {
    qb.push(" WHERE i.`release_id` = ")
        .push_bind(release_id.to_string());

    if let Some(text) = search.text.as_deref().map(str::trim) {
        if !text.is_empty() {
            let pattern = like_contains(text);
            qb.push(" AND (i.`original_name` LIKE ")
                .push_bind(pattern.clone())
                .push(" OR i.`standard_name` LIKE ")
                .push_bind(pattern.clone())
                .push(" OR i.`reading` LIKE ")
                .push_bind(pattern.clone())
                .push(
                    " OR EXISTS (SELECT 1 FROM `ingredient_release_aliases` a \
                     WHERE a.`release_id` = i.`release_id` \
                       AND a.`ingredient_key` = i.`ingredient_key` \
                       AND a.`alias` LIKE ",
                )
                .push_bind(pattern)
                .push("))");
        }
    }
    if let Some(code) = &search.category_code {
        qb.push(" AND i.`category_code` = ").push_bind(code.clone());
    }
    if let Some(state) = &search.cooking_state {
        qb.push(" AND i.`cooking_state` = ")
            .push_bind(state.clone());
    }
}

#[derive(Debug)]
pub struct IngredientCatalogRepositoryImpl {
    db: Arc<persistence::Db>,
}

impl IngredientCatalogRepositoryImpl {
    pub fn new(db: Arc<persistence::Db>) -> Self {
        Self { db }
    }

    async fn aliases_for_ingredients(
        &self,
        release_id: &str,
        ingredient_keys: &[String],
    ) -> errors::Result<BTreeMap<String, Vec<String>>> {
        let mut aliases = BTreeMap::<String, Vec<String>>::new();
        for chunk in ingredient_keys.chunks(INSERT_CHUNK) {
            if chunk.is_empty() {
                continue;
            }
            let mut qb = QueryBuilder::<MySql>::new(
                "SELECT `ingredient_key`, CAST(`alias` AS CHAR CHARACTER SET utf8mb4) AS `alias` \
                 FROM `ingredient_release_aliases` \
                 WHERE `release_id` = ",
            );
            qb.push_bind(release_id.to_string())
                .push(" AND `ingredient_key` IN (");
            let mut separated = qb.separated(", ");
            for key in chunk {
                separated.push_bind(key);
            }
            separated.push_unseparated(
                ") ORDER BY `ingredient_key` ASC, `alias` ASC",
            );
            let rows: Vec<AliasRow> = qb
                .build_query_as()
                .fetch_all(self.db.pool().as_ref())
                .await
                .map_err(db_error)?;
            for row in rows {
                aliases
                    .entry(row.ingredient_key)
                    .or_default()
                    .push(row.alias);
            }
        }
        Ok(aliases)
    }
}

#[async_trait::async_trait]
impl IngredientCatalogRepository for IngredientCatalogRepositoryImpl {
    async fn insert_catalog(
        &self,
        catalog: &IngredientCatalog,
    ) -> errors::Result<()> {
        sqlx::query(&format!(
            "INSERT INTO `ingredient_catalogs` ({CATALOG_COLUMNS}) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
        ))
        .bind(catalog.id().to_string())
        .bind(catalog.tenant_id().to_string())
        .bind(catalog.catalog_key())
        .bind(catalog.name())
        .bind(catalog.ingredient_repo_id().to_string())
        .bind(catalog.nutrient_repo_id().to_string())
        .bind(catalog.value_repo_id().to_string())
        .bind(catalog.created_at())
        .bind(catalog.updated_at())
        .execute(self.db.pool().as_ref())
        .await
        .map_err(db_error)?;
        Ok(())
    }

    async fn get_catalog_by_key(
        &self,
        tenant_id: &TenantId,
        catalog_key: &str,
    ) -> errors::Result<Option<IngredientCatalog>> {
        let row: Option<CatalogRow> = sqlx::query_as(&format!(
            "SELECT {CATALOG_COLUMNS} FROM `ingredient_catalogs` \
             WHERE `tenant_id` = ? AND `catalog_key` = ?"
        ))
        .bind(tenant_id.to_string())
        .bind(catalog_key)
        .fetch_optional(self.db.pool().as_ref())
        .await
        .map_err(db_error)?;
        row.map(IngredientCatalog::try_from).transpose()
    }

    async fn insert_release(
        &self,
        release: &IngredientRelease,
        snapshot: &ReleaseSnapshot,
    ) -> errors::Result<()> {
        let release_id = release.id().to_string();
        let source = release.source();
        let mut tx = self.db.pool().begin().await.map_err(db_error)?;

        sqlx::query(&format!(
            "INSERT INTO `ingredient_releases` ({RELEASE_COLUMNS}) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        ))
        .bind(&release_id)
        .bind(release.tenant_id().to_string())
        .bind(release.catalog_id().to_string())
        .bind(&source.source_id)
        .bind(&source.source_release)
        .bind(&source.source_url)
        .bind(source.source_retrieved_at)
        .bind(&source.notes)
        .bind(release.schema_version())
        .bind(release.content_hash())
        .bind(release.ingredient_count())
        .bind(release.nutrient_count())
        .bind(release.value_count())
        .bind(release.private_repo_mask())
        .bind(release.published_by())
        .bind(release.published_at())
        .execute(&mut *tx)
        .await
        .map_err(db_error)?;

        for chunk in snapshot.nutrients().chunks(INSERT_CHUNK) {
            let mut qb = QueryBuilder::<MySql>::new(
                "INSERT INTO `ingredient_release_nutrients` \
                 (`release_id`, `nutrient_key`, `name`, `unit`, `basis`, \
                  `method`, `display_order`, `default_display`) ",
            );
            qb.push_values(chunk, |mut b, n| {
                b.push_bind(&release_id)
                    .push_bind(&n.nutrient_key)
                    .push_bind(&n.name)
                    .push_bind(&n.unit)
                    .push_bind(&n.basis)
                    .push_bind(&n.method)
                    .push_bind(n.display_order)
                    .push_bind(n.default_display);
            });
            qb.build().execute(&mut *tx).await.map_err(db_error)?;
        }

        for chunk in snapshot.ingredients().chunks(INSERT_CHUNK) {
            let mut qb = QueryBuilder::<MySql>::new(
                "INSERT INTO `ingredient_release_items` \
                 (`release_id`, `ingredient_key`, `source_food_code`, \
                  `original_name`, `standard_name`, `reading`, \
                  `category_code`, `category_name`, `part`, \
                  `cooking_state`, `skin_bone`, `refuse_rate`, \
                  `attribute_review_status`) ",
            );
            qb.push_values(chunk, |mut b, i| {
                b.push_bind(&release_id)
                    .push_bind(&i.ingredient_key)
                    .push_bind(&i.source_food_code)
                    .push_bind(&i.original_name)
                    .push_bind(&i.standard_name)
                    .push_bind(&i.reading)
                    .push_bind(&i.category_code)
                    .push_bind(&i.category_name)
                    .push_bind(&i.part)
                    .push_bind(&i.cooking_state)
                    .push_bind(&i.skin_bone)
                    .push_bind(
                        i.refuse_rate.as_ref().map(|r| r.to_string()),
                    )
                    .push_bind(i.attribute_review_status.as_str());
            });
            qb.build().execute(&mut *tx).await.map_err(db_error)?;
        }

        let aliases: Vec<(&str, &str)> = snapshot
            .ingredients()
            .iter()
            .flat_map(|i| {
                i.aliases
                    .iter()
                    .map(move |a| (i.ingredient_key.as_str(), a.as_str()))
            })
            .collect();
        for chunk in aliases.chunks(INSERT_CHUNK) {
            let mut qb = QueryBuilder::<MySql>::new(
                "INSERT INTO `ingredient_release_aliases` \
                 (`release_id`, `ingredient_key`, `alias`) ",
            );
            qb.push_values(chunk, |mut b, (key, alias)| {
                b.push_bind(&release_id).push_bind(*key).push_bind(*alias);
            });
            qb.build().execute(&mut *tx).await.map_err(db_error)?;
        }

        for chunk in snapshot.values().chunks(INSERT_CHUNK) {
            let mut qb = QueryBuilder::<MySql>::new(
                "INSERT INTO `ingredient_release_values` \
                 (`release_id`, `ingredient_key`, `nutrient_key`, \
                  `value_status`, `amount`, `raw_notation`) ",
            );
            qb.push_values(chunk, |mut b, v| {
                b.push_bind(&release_id)
                    .push_bind(&v.ingredient_key)
                    .push_bind(&v.nutrient_key)
                    .push_bind(v.status.as_str())
                    .push_bind(v.amount.as_ref().map(|a| a.to_string()))
                    .push_bind(&v.raw_notation);
            });
            qb.build().execute(&mut *tx).await.map_err(db_error)?;
        }

        tx.commit().await.map_err(db_error)?;
        Ok(())
    }

    async fn find_release_by_source(
        &self,
        tenant_id: &TenantId,
        catalog_id: &IngredientCatalogId,
        source_id: &str,
        source_release: &str,
    ) -> errors::Result<Option<IngredientRelease>> {
        let row: Option<ReleaseRow> = sqlx::query_as(&format!(
            "SELECT {RELEASE_COLUMNS} FROM `ingredient_releases` \
             WHERE `tenant_id` = ? AND `catalog_id` = ? \
               AND `source_id` = ? AND `source_release` = ?"
        ))
        .bind(tenant_id.to_string())
        .bind(catalog_id.to_string())
        .bind(source_id)
        .bind(source_release)
        .fetch_optional(self.db.pool().as_ref())
        .await
        .map_err(db_error)?;
        row.map(IngredientRelease::try_from).transpose()
    }

    async fn get_release(
        &self,
        tenant_id: &TenantId,
        catalog_id: &IngredientCatalogId,
        release_id: &IngredientReleaseId,
    ) -> errors::Result<Option<IngredientRelease>> {
        let row: Option<ReleaseRow> = sqlx::query_as(&format!(
            "SELECT {RELEASE_COLUMNS} FROM `ingredient_releases` \
             WHERE `tenant_id` = ? AND `catalog_id` = ? AND `id` = ?"
        ))
        .bind(tenant_id.to_string())
        .bind(catalog_id.to_string())
        .bind(release_id.to_string())
        .fetch_optional(self.db.pool().as_ref())
        .await
        .map_err(db_error)?;
        row.map(IngredientRelease::try_from).transpose()
    }

    async fn list_releases(
        &self,
        tenant_id: &TenantId,
        catalog_id: &IngredientCatalogId,
    ) -> errors::Result<Vec<IngredientRelease>> {
        let rows: Vec<ReleaseRow> = sqlx::query_as(&format!(
            "SELECT {RELEASE_COLUMNS} FROM `ingredient_releases` \
             WHERE `tenant_id` = ? AND `catalog_id` = ? \
             ORDER BY `published_at` DESC, `id` DESC"
        ))
        .bind(tenant_id.to_string())
        .bind(catalog_id.to_string())
        .fetch_all(self.db.pool().as_ref())
        .await
        .map_err(db_error)?;
        rows.into_iter().map(IngredientRelease::try_from).collect()
    }

    async fn search_ingredients(
        &self,
        release: &IngredientRelease,
        search: &IngredientSearch,
    ) -> errors::Result<(Vec<ReleasedIngredient>, u64)> {
        let release_id = release.id().to_string();
        let page = search.page.max(1);
        let page_size = search.page_size.max(1);

        let mut count_qb = QueryBuilder::<MySql>::new(
            "SELECT COUNT(*) FROM `ingredient_release_items` i",
        );
        push_search_filters(&mut count_qb, &release_id, search);
        let total: i64 = count_qb
            .build_query_scalar()
            .fetch_one(self.db.pool().as_ref())
            .await
            .map_err(db_error)?;

        let mut qb = QueryBuilder::<MySql>::new(ITEM_SELECT);
        push_search_filters(&mut qb, &release_id, search);
        qb.push(
            " ORDER BY i.`source_food_code` ASC, i.`ingredient_key` ASC",
        )
        .push(" LIMIT ")
        .push_bind(page_size)
        .push(" OFFSET ")
        .push_bind(u64::from(page - 1) * u64::from(page_size));
        let rows: Vec<ItemRow> = qb
            .build_query_as()
            .fetch_all(self.db.pool().as_ref())
            .await
            .map_err(db_error)?;

        let keys = rows
            .iter()
            .map(|row| row.ingredient_key.clone())
            .collect::<Vec<_>>();
        let mut aliases =
            self.aliases_for_ingredients(&release_id, &keys).await?;
        let mut items = rows
            .into_iter()
            .map(ReleasedIngredient::try_from)
            .collect::<errors::Result<Vec<_>>>()?;
        for item in &mut items {
            item.aliases =
                aliases.remove(&item.ingredient_key).unwrap_or_default();
        }
        Ok((items, u64::try_from(total).unwrap_or(0)))
    }

    async fn get_ingredient(
        &self,
        release: &IngredientRelease,
        ingredient_key: &str,
    ) -> errors::Result<Option<ReleasedIngredient>> {
        let row: Option<ItemRow> = sqlx::query_as(&format!(
            "{ITEM_SELECT} WHERE i.`release_id` = ? AND i.`ingredient_key` = ?"
        ))
        .bind(release.id().to_string())
        .bind(ingredient_key)
        .fetch_optional(self.db.pool().as_ref())
        .await
        .map_err(db_error)?;
        match row {
            Some(row) => {
                let mut ingredient = ReleasedIngredient::try_from(row)?;
                let keys = [ingredient.ingredient_key.clone()];
                let mut aliases = self
                    .aliases_for_ingredients(release.id().as_ref(), &keys)
                    .await?;
                ingredient.aliases = aliases
                    .remove(&ingredient.ingredient_key)
                    .unwrap_or_default();
                Ok(Some(ingredient))
            }
            None => Ok(None),
        }
    }

    async fn list_values(
        &self,
        release: &IngredientRelease,
        ingredient_key: &str,
    ) -> errors::Result<Vec<ReleasedValue>> {
        let rows: Vec<ValueRow> = sqlx::query_as(
            "SELECT `ingredient_key`, `nutrient_key`, `value_status`, \
                    `amount`, `raw_notation` \
             FROM `ingredient_release_values` \
             WHERE `release_id` = ? AND `ingredient_key` = ? \
             ORDER BY `nutrient_key` ASC",
        )
        .bind(release.id().to_string())
        .bind(ingredient_key)
        .fetch_all(self.db.pool().as_ref())
        .await
        .map_err(db_error)?;
        rows.into_iter().map(ReleasedValue::try_from).collect()
    }

    async fn list_nutrients(
        &self,
        release: &IngredientRelease,
    ) -> errors::Result<Vec<ReleasedNutrient>> {
        let rows: Vec<NutrientRow> = sqlx::query_as(
            "SELECT `nutrient_key`, `name`, `unit`, `basis`, `method`, \
                    `display_order`, `default_display` \
             FROM `ingredient_release_nutrients` \
             WHERE `release_id` = ? \
             ORDER BY `display_order` ASC, `nutrient_key` ASC",
        )
        .bind(release.id().to_string())
        .fetch_all(self.db.pool().as_ref())
        .await
        .map_err(db_error)?;
        Ok(rows.into_iter().map(ReleasedNutrient::from).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn like_pattern_matches_wildcards_literally() {
        assert_eq!(like_contains("玉ねぎ"), "%玉ねぎ%");
        assert_eq!(like_contains("100%_\\"), "%100\\%\\_\\\\%");
    }

    #[test]
    fn search_filters_bind_every_user_value() {
        let search = IngredientSearch {
            text: Some("玉ねぎ".into()),
            category_code: Some("06".into()),
            cooking_state: Some("raw".into()),
            page: 2,
            page_size: 20,
        };
        let mut qb = QueryBuilder::<MySql>::new("SELECT 1 FROM t i");
        push_search_filters(&mut qb, "irel_x", &search);
        let sql = qb.sql();
        assert!(!sql.contains("玉ねぎ"), "text must be bound: {sql}");
        assert!(!sql.contains("raw"), "state must be bound: {sql}");
        assert!(sql.contains("a.`alias` LIKE ?"), "{sql}");
        assert!(sql.contains("i.`cooking_state` = ?"), "{sql}");
    }

    #[test]
    fn blank_text_adds_no_name_filter() {
        let search = IngredientSearch {
            text: Some("  ".into()),
            ..Default::default()
        };
        let mut qb = QueryBuilder::<MySql>::new("SELECT 1 FROM t i");
        push_search_filters(&mut qb, "irel_x", &search);
        assert!(!qb.sql().contains("LIKE"));
    }
}

/// Requires MySQL with the library migrations applied:
/// `cargo run -p library-api --bin library_api_migrate dev`, then
/// `cargo test -p library-api --lib ingredient_catalog_db -- --ignored`.
#[cfg(test)]
mod ingredient_catalog_db_tests {
    use super::*;
    use crate::domain::{fixtures, DraftCatalog, RepoId};
    use persistence::test_helper::setup_test_db;

    async fn repository() -> IngredientCatalogRepositoryImpl {
        IngredientCatalogRepositoryImpl::new(setup_test_db("library").await)
    }

    fn source(release: &str) -> ReleaseSource {
        ReleaseSource {
            source_id: "mext-sfct-2023".into(),
            source_release: release.into(),
            source_url: Some("https://www.mext.go.jp/".into()),
            source_retrieved_at: None,
            notes: None,
        }
    }

    /// A fresh catalog in a fresh tenant, so tests never see each other.
    async fn publish(
        repo: &IngredientCatalogRepositoryImpl,
        draft: &DraftCatalog,
    ) -> (IngredientCatalog, IngredientRelease) {
        let catalog = IngredientCatalog::create(
            &TenantId::default(),
            "food-composition",
            "食品成分",
            &RepoId::default(),
            &RepoId::default(),
            &RepoId::default(),
        )
        .unwrap();
        repo.insert_catalog(&catalog).await.unwrap();
        let snapshot = ReleaseSnapshot::build(draft).unwrap();
        let release = IngredientRelease::publish(
            &catalog,
            source("2023"),
            &snapshot,
            "us_test",
        )
        .unwrap();
        repo.insert_release(&release, &snapshot).await.unwrap();
        (catalog, release)
    }

    fn search(
        text: Option<&str>,
        state: Option<&str>,
        page: u32,
        size: u32,
    ) -> IngredientSearch {
        IngredientSearch {
            text: text.map(Into::into),
            category_code: None,
            cooking_state: state.map(Into::into),
            page,
            page_size: size,
        }
    }

    #[tokio::test]
    #[ignore = "requires MySQL configured by DEV_DATABASE_URL"]
    async fn ingredient_catalog_db_round_trips_a_release() {
        let repo = repository().await;
        let (catalog, release) =
            publish(&repo, &fixtures::onion_catalog()).await;

        let stored = repo
            .get_release(catalog.tenant_id(), catalog.id(), release.id())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(stored.content_hash(), release.content_hash());
        assert_eq!(*stored.value_count(), 6);

        // Another tenant cannot see the release.
        assert!(repo
            .get_release(&TenantId::default(), catalog.id(), release.id())
            .await
            .unwrap()
            .is_none());

        // An alias that is in neither name finds both states.
        let (items, total) = repo
            .search_ingredients(
                &release,
                &search(Some("オニオン"), None, 1, 20),
            )
            .await
            .unwrap();
        assert_eq!(total, 1);
        assert_eq!(items[0].ingredient_key, "onion-raw");
        let (items, total) = repo
            .search_ingredients(
                &release,
                &search(Some("玉ねぎ"), None, 1, 20),
            )
            .await
            .unwrap();
        assert_eq!(total, 2);
        assert_eq!(items[0].source_food_code, "06153");

        // The state filter separates raw from boiled.
        let (items, _) = repo
            .search_ingredients(
                &release,
                &search(Some("玉ねぎ"), Some("boiled"), 1, 20),
            )
            .await
            .unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].ingredient_key, "onion-boiled");

        // LIKE wildcards in user text are literal.
        let (_, total) = repo
            .search_ingredients(&release, &search(Some("%"), None, 1, 20))
            .await
            .unwrap();
        assert_eq!(total, 0);

        // Values keep their meaning through the database.
        let values =
            repo.list_values(&release, "onion-boiled").await.unwrap();
        let prot =
            values.iter().find(|v| v.nutrient_key == "PROT-").unwrap();
        assert_eq!(prot.status, NutrientValueStatus::Estimated);
        assert_eq!(prot.amount.as_ref().unwrap().as_str(), "0.1");
        let na = values.iter().find(|v| v.nutrient_key == "NA").unwrap();
        assert_eq!(na.status, NutrientValueStatus::Trace);
        assert_eq!(na.amount, None);

        // Reading everything back reproduces the published hash.
        let ingredient = repo
            .get_ingredient(&release, "onion-raw")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(ingredient.aliases, ["オニオン", "玉ねぎ"]);
        let mut ingredients = Vec::new();
        let mut all_values = Vec::new();
        for key in ["onion-boiled", "onion-raw"] {
            ingredients.push(
                repo.get_ingredient(&release, key).await.unwrap().unwrap(),
            );
            all_values
                .extend(repo.list_values(&release, key).await.unwrap());
        }
        let mut nutrients = repo.list_nutrients(&release).await.unwrap();
        nutrients.sort_by(|a, b| a.nutrient_key.cmp(&b.nutrient_key));
        all_values.sort_by(|a, b| {
            (&a.ingredient_key, &a.nutrient_key)
                .cmp(&(&b.ingredient_key, &b.nutrient_key))
        });
        assert_eq!(
            ReleaseSnapshot::compute_hash(
                *release.schema_version(),
                &ingredients,
                &nutrients,
                &all_values
            ),
            release.content_hash().as_str()
        );
    }

    #[tokio::test]
    #[ignore = "requires MySQL configured by DEV_DATABASE_URL"]
    async fn ingredient_catalog_db_pages_past_the_first_200() {
        let repo = repository().await;
        let mut draft = fixtures::onion_catalog();
        draft.values.clear();
        draft.ingredients = (0..205)
            .map(|n| {
                let mut i = fixtures::ingredient(
                    &format!("veg-{n:03}"),
                    &format!("{n:05}"),
                    &format!("野菜 {n}"),
                );
                i.aliases = Some("やさい".into());
                i
            })
            .collect();
        let (_, release) = publish(&repo, &draft).await;

        let (items, total) = repo
            .search_ingredients(
                &release,
                &search(Some("やさい"), None, 3, 100),
            )
            .await
            .unwrap();
        assert_eq!(total, 205);
        let keys: Vec<_> =
            items.iter().map(|i| i.ingredient_key.as_str()).collect();
        assert_eq!(
            keys,
            ["veg-200", "veg-201", "veg-202", "veg-203", "veg-204"]
        );
    }

    #[tokio::test]
    #[ignore = "requires MySQL configured by DEV_DATABASE_URL"]
    async fn ingredient_catalog_db_rejects_a_second_release_with_the_same_label(
    ) {
        let repo = repository().await;
        let (catalog, release) =
            publish(&repo, &fixtures::onion_catalog()).await;

        let snapshot =
            ReleaseSnapshot::build(&fixtures::onion_catalog()).unwrap();
        let again = IngredientRelease::publish(
            &catalog,
            source("2023"),
            &snapshot,
            "us_test",
        )
        .unwrap();
        let err = repo.insert_release(&again, &snapshot).await.unwrap_err();
        assert!(err.to_string().contains("unique"), "{err}");

        // The failed insert left nothing behind for the new release ID.
        assert!(repo
            .get_release(catalog.tenant_id(), catalog.id(), again.id())
            .await
            .unwrap()
            .is_none());
        let found = repo
            .find_release_by_source(
                catalog.tenant_id(),
                catalog.id(),
                "mext-sfct-2023",
                "2023",
            )
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found.id(), release.id());
    }
}
