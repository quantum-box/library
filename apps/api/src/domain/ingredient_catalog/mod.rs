//! COM-860: common ingredient master.
//!
//! A catalog ties together three Library repos in one org: ingredients,
//! nutrient definitions, and ingredient × nutrient values. People edit
//! them as ordinary Library data. Publishing validates the three repos
//! together and freezes them into an immutable release, which is what
//! Field and other consumers reference by ingredient key + release ID.
//!
//! Store-specific data (purchase prices, private recipes) never enters a
//! catalog: the draft schema has no field for it and consumers only read.

mod snapshot;

// The value rules live in a shared crate so the COM-861 importer
// (`library food import`) applies exactly the rules publishing checks.
pub use ingredient_notation::{NormalizedDecimal, NutrientValueStatus};
pub use snapshot::*;

#[cfg(test)]
pub(crate) use snapshot::fixtures;

use chrono::{DateTime, Utc};
use derive_getters::Getters;
use util::macros::*;
use value_object::TenantId;

use super::RepoId;

def_id!(IngredientCatalogId, "icat_");
def_id!(IngredientReleaseId, "irel_");

#[derive(Debug, Clone, PartialEq, Eq, Getters)]
pub struct IngredientCatalog {
    id: IngredientCatalogId,
    /// The org that owns the catalog and its three repos.
    tenant_id: TenantId,
    /// URL-safe name, unique within the org, e.g. `food-composition`.
    catalog_key: String,
    name: String,
    ingredient_repo_id: RepoId,
    nutrient_repo_id: RepoId,
    value_repo_id: RepoId,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl IngredientCatalog {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: IngredientCatalogId,
        tenant_id: TenantId,
        catalog_key: String,
        name: String,
        ingredient_repo_id: RepoId,
        nutrient_repo_id: RepoId,
        value_repo_id: RepoId,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            tenant_id,
            catalog_key,
            name,
            ingredient_repo_id,
            nutrient_repo_id,
            value_repo_id,
            created_at,
            updated_at,
        }
    }

    pub fn create(
        tenant_id: &TenantId,
        catalog_key: &str,
        name: &str,
        ingredient_repo_id: &RepoId,
        nutrient_repo_id: &RepoId,
        value_repo_id: &RepoId,
    ) -> errors::Result<Self> {
        let key = catalog_key.trim();
        let valid_key = !key.is_empty()
            && key.len() <= 64
            && key.bytes().all(|b| {
                b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'
            });
        if !valid_key {
            return Err(errors::Error::invalid(format!(
                "catalog_key must be 1-64 characters of [a-z0-9-]: {catalog_key:?}"
            )));
        }
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 255 {
            return Err(errors::Error::invalid(
                "catalog name must be 1-255 characters",
            ));
        }
        let repos = [ingredient_repo_id, nutrient_repo_id, value_repo_id];
        if repos[0] == repos[1]
            || repos[0] == repos[2]
            || repos[1] == repos[2]
        {
            return Err(errors::Error::invalid(
                "ingredient, nutrient and value repos must be different repos",
            ));
        }

        let now = Utc::now();
        Ok(Self::new(
            IngredientCatalogId::default(),
            tenant_id.clone(),
            key.to_string(),
            name.to_string(),
            ingredient_repo_id.clone(),
            nutrient_repo_id.clone(),
            value_repo_id.clone(),
            now,
            now,
        ))
    }

    pub fn repo_ids(&self) -> [&RepoId; 3] {
        [
            &self.ingredient_repo_id,
            &self.nutrient_repo_id,
            &self.value_repo_id,
        ]
    }
}

/// Where a release's data came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseSource {
    /// Dataset identifier, e.g. `mext-sfct-2023`.
    pub source_id: String,
    /// Edition label within the dataset, including applied errata,
    /// e.g. `2023+errata-2026-03-27`.
    pub source_release: String,
    pub source_url: Option<String>,
    pub source_retrieved_at: Option<DateTime<Utc>>,
    pub notes: Option<String>,
}

impl ReleaseSource {
    pub fn validate(&self) -> errors::Result<()> {
        let token = |field: &str, s: &str| {
            let valid = !s.is_empty()
                && s.len() <= 128
                && s.bytes().all(|b| {
                    b.is_ascii_alphanumeric() || b"._+-".contains(&b)
                });
            if valid {
                Ok(())
            } else {
                Err(errors::Error::invalid(format!(
                    "{field} must be 1-128 characters of [A-Za-z0-9._+-]: {s:?}"
                )))
            }
        };
        token("source_id", &self.source_id)?;
        token("source_release", &self.source_release)?;
        if self.source_url.as_ref().is_some_and(|u| u.len() > 2048) {
            return Err(errors::Error::invalid("source_url is too long"));
        }
        if self
            .notes
            .as_ref()
            .is_some_and(|n| n.chars().count() > 2000)
        {
            return Err(errors::Error::invalid("notes is too long"));
        }
        Ok(())
    }
}

/// Metadata of a published release. The frozen rows are stored with it.
#[derive(Debug, Clone, PartialEq, Eq, Getters)]
pub struct IngredientRelease {
    id: IngredientReleaseId,
    tenant_id: TenantId,
    catalog_id: IngredientCatalogId,
    source: ReleaseSource,
    schema_version: u32,
    content_hash: String,
    ingredient_count: u32,
    nutrient_count: u32,
    value_count: u32,
    published_by: String,
    published_at: DateTime<Utc>,
}

impl IngredientRelease {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: IngredientReleaseId,
        tenant_id: TenantId,
        catalog_id: IngredientCatalogId,
        source: ReleaseSource,
        schema_version: u32,
        content_hash: String,
        ingredient_count: u32,
        nutrient_count: u32,
        value_count: u32,
        published_by: String,
        published_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            tenant_id,
            catalog_id,
            source,
            schema_version,
            content_hash,
            ingredient_count,
            nutrient_count,
            value_count,
            published_by,
            published_at,
        }
    }

    pub fn publish(
        catalog: &IngredientCatalog,
        source: ReleaseSource,
        snapshot: &ReleaseSnapshot,
        published_by: &str,
    ) -> errors::Result<Self> {
        source.validate()?;
        let count = |n: usize| {
            u32::try_from(n).map_err(|_| {
                errors::Error::invalid("release is too large to publish")
            })
        };
        Ok(Self::new(
            IngredientReleaseId::default(),
            catalog.tenant_id().clone(),
            catalog.id().clone(),
            source,
            SNAPSHOT_SCHEMA_VERSION,
            snapshot.content_hash().to_string(),
            count(snapshot.ingredients().len())?,
            count(snapshot.nutrients().len())?,
            count(snapshot.values().len())?,
            published_by.to_string(),
            Utc::now(),
        ))
    }
}

/// Filters for searching the ingredients of one release.
#[derive(Debug, Clone, Default)]
pub struct IngredientSearch {
    /// Substring matched against original name, standard name, reading
    /// and aliases. Matching an alias never picks a state for the
    /// caller: raw and boiled onions both match `玉ねぎ`.
    pub text: Option<String>,
    pub category_code: Option<String>,
    pub cooking_state: Option<String>,
    pub page: u32,
    pub page_size: u32,
}

#[async_trait::async_trait]
pub trait IngredientCatalogRepository:
    std::marker::Send + Sync + std::fmt::Debug
{
    /// Unique-key collisions surface as `conflict`.
    async fn insert_catalog(
        &self,
        catalog: &IngredientCatalog,
    ) -> errors::Result<()>;

    async fn get_catalog_by_key(
        &self,
        tenant_id: &TenantId,
        catalog_key: &str,
    ) -> errors::Result<Option<IngredientCatalog>>;

    /// Write the release and every frozen row in one transaction. There
    /// is deliberately no update or delete for releases.
    async fn insert_release(
        &self,
        release: &IngredientRelease,
        snapshot: &ReleaseSnapshot,
    ) -> errors::Result<()>;

    async fn find_release_by_source(
        &self,
        tenant_id: &TenantId,
        catalog_id: &IngredientCatalogId,
        source_id: &str,
        source_release: &str,
    ) -> errors::Result<Option<IngredientRelease>>;

    async fn get_release(
        &self,
        tenant_id: &TenantId,
        catalog_id: &IngredientCatalogId,
        release_id: &IngredientReleaseId,
    ) -> errors::Result<Option<IngredientRelease>>;

    /// Newest first.
    async fn list_releases(
        &self,
        tenant_id: &TenantId,
        catalog_id: &IngredientCatalogId,
    ) -> errors::Result<Vec<IngredientRelease>>;

    /// Returns one page and the total number of matches.
    async fn search_ingredients(
        &self,
        release: &IngredientRelease,
        search: &IngredientSearch,
    ) -> errors::Result<(Vec<ReleasedIngredient>, u64)>;

    async fn get_ingredient(
        &self,
        release: &IngredientRelease,
        ingredient_key: &str,
    ) -> errors::Result<Option<ReleasedIngredient>>;

    async fn list_values(
        &self,
        release: &IngredientRelease,
        ingredient_key: &str,
    ) -> errors::Result<Vec<ReleasedValue>>;

    async fn list_nutrients(
        &self,
        release: &IngredientRelease,
    ) -> errors::Result<Vec<ReleasedNutrient>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tenant() -> TenantId {
        "tn_01hjryxysgey07h5jz5wagqj0m".parse().unwrap()
    }

    fn repos() -> (RepoId, RepoId, RepoId) {
        (RepoId::default(), RepoId::default(), RepoId::default())
    }

    #[test]
    fn catalog_key_is_url_safe() {
        let (i, n, v) = repos();
        assert!(IngredientCatalog::create(
            &tenant(),
            "food-composition",
            "食品成分",
            &i,
            &n,
            &v
        )
        .is_ok());
        for bad in ["", "Food", "food_composition", "食品"] {
            assert!(
                IngredientCatalog::create(&tenant(), bad, "x", &i, &n, &v)
                    .is_err(),
                "{bad}"
            );
        }
    }

    #[test]
    fn catalog_repos_must_differ() {
        let (i, n, _) = repos();
        assert!(IngredientCatalog::create(&tenant(), "c", "c", &i, &n, &i)
            .is_err());
    }

    fn source(release: &str) -> ReleaseSource {
        ReleaseSource {
            source_id: "mext-sfct-2023".into(),
            source_release: release.into(),
            source_url: None,
            source_retrieved_at: None,
            notes: None,
        }
    }

    #[test]
    fn release_records_counts_and_hash() {
        let (i, n, v) = repos();
        let catalog =
            IngredientCatalog::create(&tenant(), "c", "c", &i, &n, &v)
                .unwrap();
        let snapshot =
            ReleaseSnapshot::build(&fixtures::onion_catalog()).unwrap();

        let release = IngredientRelease::publish(
            &catalog,
            source("2023+errata-2026-03-27"),
            &snapshot,
            "us_x",
        )
        .unwrap();

        assert!(release.id().as_str().starts_with("irel_"));
        assert_eq!(release.content_hash(), snapshot.content_hash());
        assert_eq!(*release.ingredient_count(), 2);
        assert_eq!(*release.nutrient_count(), 4);
        assert_eq!(*release.value_count(), 6);
        assert_eq!(*release.schema_version(), SNAPSHOT_SCHEMA_VERSION);
    }

    #[test]
    fn release_source_labels_are_validated() {
        assert!(source("2023").validate().is_ok());
        assert!(source("").validate().is_err());
        assert!(source("2023 errata").validate().is_err());
    }
}
