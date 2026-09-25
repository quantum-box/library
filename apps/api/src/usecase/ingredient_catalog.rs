//! COM-860: common ingredient master usecases.
//!
//! Writes (register a catalog, publish a release) need `library:UpdateRepo`,
//! the same action that edits the draft repos. Reads follow the draft
//! repos' visibility: anyone can read a catalog whose three repos are
//! public, and a private repo needs the usual repo read permission.
//! Consumers such as Field only read; nothing here lets them write back.

use std::collections::BTreeMap;
use std::fmt::Debug;
use std::sync::Arc;

use database_manager::{
    domain::{Data, Property},
    usecase::FindAllPropertiesInputData,
};
use tachyon_sdk::auth::{
    AuthApp, CheckPolicyInput, ExecutorAction, MultiTenancyAction,
};
use value_object::{TenantId, MAX_PAGE_SIZE};

use crate::domain::{
    DraftCatalog, DraftIngredient, DraftNutrient, DraftValue,
    IngredientCatalog, IngredientCatalogRepository, IngredientRelease,
    IngredientReleaseId, IngredientSearch, NutrientValueStatus,
    ReleaseSnapshot, ReleaseSource, ReleasedIngredient, ReleasedNutrient,
    ReleasedValue, Repo, RepoRepository,
};
use crate::usecase::{
    authorize_private_repo_read, GetOrganizationByUsernameQuery,
    GetRepoByUsernameQuery,
};

/// Property names the draft repos use. The ingredient's source name is
/// the data name of the ingredient record; the nutrient's display name is
/// the data name of the nutrient record.
pub mod draft_schema {
    pub const INGREDIENT_KEY: &str = "ingredient_key";
    pub const SOURCE_FOOD_CODE: &str = "source_food_code";
    pub const STANDARD_NAME: &str = "standard_name";
    pub const READING: &str = "reading";
    pub const ALIASES: &str = "aliases";
    pub const CATEGORY_CODE: &str = "category_code";
    pub const CATEGORY_NAME: &str = "category_name";
    pub const PART: &str = "part";
    pub const COOKING_STATE: &str = "cooking_state";
    pub const SKIN_BONE: &str = "skin_bone";
    pub const REFUSE_RATE: &str = "refuse_rate";
    pub const ATTRIBUTE_REVIEW_STATUS: &str = "attribute_review_status";

    pub const NUTRIENT_KEY: &str = "nutrient_key";
    pub const UNIT: &str = "unit";
    pub const BASIS: &str = "basis";
    pub const METHOD: &str = "method";
    pub const DISPLAY_ORDER: &str = "display_order";
    pub const DEFAULT_DISPLAY: &str = "default_display";

    pub const VALUE_STATUS: &str = "value_status";
    pub const AMOUNT: &str = "amount";
    pub const RAW_NOTATION: &str = "raw_notation";
}

/// Upper bound on draft records read from one repo, so a runaway repo
/// cannot keep a publish request paging forever.
const MAX_DRAFT_RECORDS: usize = 500_000;

// ==================== Ports ====================

#[async_trait::async_trait]
pub trait CreateIngredientCatalogInputPort: Debug + Send + Sync {
    async fn execute<'a>(
        &self,
        input: CreateIngredientCatalogInputData<'a>,
    ) -> errors::Result<IngredientCatalog>;
}

#[derive(Debug)]
pub struct CreateIngredientCatalogInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    pub org_username: String,
    pub catalog_key: String,
    pub name: String,
    pub ingredient_repo: String,
    pub nutrient_repo: String,
    pub value_repo: String,
}

#[async_trait::async_trait]
pub trait PublishIngredientReleaseInputPort: Debug + Send + Sync {
    /// Returns the release and whether it was created by this call. A
    /// retry with identical content returns the existing release.
    async fn execute<'a>(
        &self,
        input: PublishIngredientReleaseInputData<'a>,
    ) -> errors::Result<(IngredientRelease, bool)>;
}

#[derive(Debug)]
pub struct PublishIngredientReleaseInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    pub org_username: String,
    pub catalog_key: String,
    pub source: ReleaseSource,
}

/// Read access to published releases of one catalog.
#[async_trait::async_trait]
pub trait ReadIngredientCatalogInputPort: Debug + Send + Sync {
    async fn list_releases<'a>(
        &self,
        target: CatalogTarget<'a>,
    ) -> errors::Result<(IngredientCatalog, Vec<IngredientRelease>)>;

    async fn get_release<'a>(
        &self,
        target: CatalogTarget<'a>,
        release_id: &str,
    ) -> errors::Result<(IngredientRelease, Vec<ReleasedNutrient>)>;

    /// Returns one page, the total number of matches, and the page used.
    async fn search_ingredients<'a>(
        &self,
        target: CatalogTarget<'a>,
        release_id: &str,
        search: IngredientSearch,
    ) -> errors::Result<(Vec<ReleasedIngredient>, u64, IngredientSearch)>;

    async fn get_ingredient<'a>(
        &self,
        target: CatalogTarget<'a>,
        release_id: &str,
        ingredient_key: &str,
    ) -> errors::Result<ReleasedIngredientDetail>;
}

#[derive(Debug, Clone, Copy)]
pub struct CatalogTarget<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    pub org_username: &'a str,
    pub catalog_key: &'a str,
}

/// One ingredient of a release with a value for every nutrient of the
/// release. Nutrients without a stored value are `NotListed`.
#[derive(Debug, Clone)]
pub struct ReleasedIngredientDetail {
    pub release: IngredientRelease,
    pub ingredient: ReleasedIngredient,
    pub values: Vec<(ReleasedNutrient, ReleasedValue)>,
}

// ==================== Shared lookups ====================

#[derive(Debug, Clone)]
struct CatalogLookup {
    get_org_by_username: Arc<dyn GetOrganizationByUsernameQuery>,
    get_repo_by_username: Arc<dyn GetRepoByUsernameQuery>,
    repo_repo: Arc<dyn RepoRepository>,
    catalogs: Arc<dyn IngredientCatalogRepository>,
    auth: Arc<dyn AuthApp>,
}

impl CatalogLookup {
    async fn org_id(&self, org_username: &str) -> errors::Result<TenantId> {
        let org = self
            .get_org_by_username
            .execute(&org_username.parse()?)
            .await?
            .ok_or_else(|| errors::Error::not_found("organization"))?;
        Ok(org.id().clone())
    }

    async fn repo_in_org(
        &self,
        org_username: &str,
        org_id: &TenantId,
        repo_username: &str,
    ) -> errors::Result<Repo> {
        let repo = self
            .get_repo_by_username
            .execute(&org_username.parse()?, &repo_username.parse()?)
            .await?
            .ok_or_else(|| {
                errors::Error::not_found(format!("repo {repo_username}"))
            })?;
        ensure_repo_in_org(&repo, org_id)?;
        Ok(repo)
    }

    async fn catalog(
        &self,
        org_username: &str,
        catalog_key: &str,
    ) -> errors::Result<(IngredientCatalog, Vec<Repo>)> {
        let org_id = self.org_id(org_username).await?;
        let catalog = self
            .catalogs
            .get_catalog_by_key(&org_id, catalog_key)
            .await?
            .ok_or_else(|| {
                errors::Error::not_found("ingredient catalog")
            })?;

        let mut repos = Vec::with_capacity(3);
        for repo_id in catalog.repo_ids() {
            let repo = self
                .repo_repo
                .get_by_id(&org_id, repo_id)
                .await?
                .ok_or_else(|| {
                    errors::Error::not_found(format!(
                        "draft repo {repo_id} of the ingredient catalog"
                    ))
                })?;
            ensure_repo_in_org(&repo, &org_id)?;
            repos.push(repo);
        }
        Ok((catalog, repos))
    }

    /// Readable when every draft repo is readable by the caller.
    async fn authorize_read(
        &self,
        executor: &dyn ExecutorAction,
        multi_tenancy: &dyn MultiTenancyAction,
        repos: &[Repo],
    ) -> errors::Result<()> {
        for repo in repos.iter().filter(|r| r.is_private()) {
            if executor.is_none() {
                return Err(errors::Error::permission_denied(
                    "Access denied",
                ));
            }
            authorize_private_repo_read(
                self.auth.as_ref(),
                executor,
                multi_tenancy,
                repo.id().as_ref(),
            )
            .await?;
        }
        Ok(())
    }

    async fn authorize_write(
        &self,
        executor: &dyn ExecutorAction,
        multi_tenancy: &dyn MultiTenancyAction,
    ) -> errors::Result<()> {
        self.auth
            .check_policy(&CheckPolicyInput {
                executor,
                multi_tenancy,
                action: "library:UpdateRepo",
            })
            .await
    }
}

/// `RepoRepository::get_by_id` is platform-scoped, so the org has to be
/// checked here: a catalog must never read another org's repo.
fn ensure_repo_in_org(
    repo: &Repo,
    org_id: &TenantId,
) -> errors::Result<()> {
    if repo.organization_id() != org_id {
        return Err(errors::Error::not_found("repo in this organization"));
    }
    Ok(())
}

// ==================== Create catalog ====================

#[derive(Debug, Clone)]
pub struct CreateIngredientCatalog {
    lookup: CatalogLookup,
}

impl CreateIngredientCatalog {
    pub fn new(
        get_org_by_username: Arc<dyn GetOrganizationByUsernameQuery>,
        get_repo_by_username: Arc<dyn GetRepoByUsernameQuery>,
        repo_repo: Arc<dyn RepoRepository>,
        catalogs: Arc<dyn IngredientCatalogRepository>,
        auth: Arc<dyn AuthApp>,
    ) -> Arc<Self> {
        Arc::new(Self {
            lookup: CatalogLookup {
                get_org_by_username,
                get_repo_by_username,
                repo_repo,
                catalogs,
                auth,
            },
        })
    }
}

#[async_trait::async_trait]
impl CreateIngredientCatalogInputPort for CreateIngredientCatalog {
    #[tracing::instrument(
        name = "CreateIngredientCatalog::execute",
        skip_all
    )]
    async fn execute<'a>(
        &self,
        input: CreateIngredientCatalogInputData<'a>,
    ) -> errors::Result<IngredientCatalog> {
        self.lookup
            .authorize_write(input.executor, input.multi_tenancy)
            .await?;
        let org = input.org_username.as_str();
        let org_id = self.lookup.org_id(org).await?;
        let ingredient_repo = self
            .lookup
            .repo_in_org(org, &org_id, &input.ingredient_repo)
            .await?;
        let nutrient_repo = self
            .lookup
            .repo_in_org(org, &org_id, &input.nutrient_repo)
            .await?;
        let value_repo = self
            .lookup
            .repo_in_org(org, &org_id, &input.value_repo)
            .await?;

        let catalog = IngredientCatalog::create(
            &org_id,
            &input.catalog_key,
            &input.name,
            ingredient_repo.id(),
            nutrient_repo.id(),
            value_repo.id(),
        )?;
        self.lookup.catalogs.insert_catalog(&catalog).await?;
        Ok(catalog)
    }
}

// ==================== Publish ====================

#[derive(Debug, Clone)]
pub struct PublishIngredientRelease {
    lookup: CatalogLookup,
    database: Arc<database_manager::App>,
}

impl PublishIngredientRelease {
    pub fn new(
        get_org_by_username: Arc<dyn GetOrganizationByUsernameQuery>,
        get_repo_by_username: Arc<dyn GetRepoByUsernameQuery>,
        repo_repo: Arc<dyn RepoRepository>,
        catalogs: Arc<dyn IngredientCatalogRepository>,
        auth: Arc<dyn AuthApp>,
        database: Arc<database_manager::App>,
    ) -> Arc<Self> {
        Arc::new(Self {
            lookup: CatalogLookup {
                get_org_by_username,
                get_repo_by_username,
                repo_repo,
                catalogs,
                auth,
            },
            database,
        })
    }

    /// Read every record of a repo's database, page by page.
    async fn read_all(
        &self,
        input: &PublishIngredientReleaseInputData<'_>,
        tenant_id: &TenantId,
        repo: &Repo,
    ) -> errors::Result<DraftRecords> {
        let database_id =
            repo.databases().first().cloned().ok_or_else(|| {
                errors::Error::application_logic_error(format!(
                    "repo {} has no database",
                    repo.username()
                ))
            })?;
        let properties = self
            .database
            .find_all_properties()
            .execute(FindAllPropertiesInputData {
                tenant_id: tenant_id.clone(),
                database_id: database_id.clone(),
            })
            .await?;

        let mut records = Vec::new();
        let mut page = 1;
        loop {
            let (data, paginator) = self
                .database
                .search_data()
                .execute(&database_manager::SearchDataInputData {
                    executor: input.executor,
                    multi_tenancy: input.multi_tenancy,
                    tenant_id,
                    database_id: Some(database_id.clone()),
                    query: "",
                    page: Some(page),
                    page_size: Some(MAX_PAGE_SIZE),
                })
                .await?;
            records.extend(data);
            if records.len() > MAX_DRAFT_RECORDS {
                return Err(errors::Error::invalid(format!(
                    "repo {} has more than {MAX_DRAFT_RECORDS} records",
                    repo.username()
                )));
            }
            if page >= paginator.total_pages {
                break;
            }
            page += 1;
        }
        // Offset paging can repeat a row if the repo is edited while we
        // read. Keep one copy per record; a repeated key still fails
        // validation instead of being silently merged.
        records.sort_by(|a, b| a.id().as_str().cmp(b.id().as_str()));
        records.dedup_by(|a, b| a.id() == b.id());

        Ok(DraftRecords {
            properties,
            records,
        })
    }
}

struct DraftRecords {
    properties: Vec<Property>,
    records: Vec<Data>,
}

impl DraftRecords {
    /// Values of one record by property name, as text.
    fn fields(&self, data: &Data) -> BTreeMap<String, String> {
        self.properties
            .iter()
            .filter_map(|p| {
                data.get_property_data(p.id()).map(|pd| {
                    (p.name().trim().to_string(), pd.string_value())
                })
            })
            .collect()
    }
}

fn field(fields: &BTreeMap<String, String>, name: &str) -> Option<String> {
    fields
        .get(name)
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn draft_ingredients(draft: &DraftRecords) -> Vec<DraftIngredient> {
    use draft_schema::*;
    draft
        .records
        .iter()
        .map(|d| {
            let f = draft.fields(d);
            DraftIngredient {
                record_ref: d.id().to_string(),
                ingredient_key: field(&f, INGREDIENT_KEY)
                    .unwrap_or_default(),
                source_food_code: field(&f, SOURCE_FOOD_CODE)
                    .unwrap_or_default(),
                original_name: d.name().to_string(),
                standard_name: field(&f, STANDARD_NAME),
                reading: field(&f, READING),
                // Aliases keep their line breaks.
                aliases: f.get(ALIASES).cloned(),
                category_code: field(&f, CATEGORY_CODE),
                category_name: field(&f, CATEGORY_NAME),
                part: field(&f, PART),
                cooking_state: field(&f, COOKING_STATE),
                skin_bone: field(&f, SKIN_BONE),
                refuse_rate: field(&f, REFUSE_RATE),
                attribute_review_status: field(&f, ATTRIBUTE_REVIEW_STATUS),
            }
        })
        .collect()
}

fn draft_nutrients(draft: &DraftRecords) -> Vec<DraftNutrient> {
    use draft_schema::*;
    draft
        .records
        .iter()
        .map(|d| {
            let f = draft.fields(d);
            DraftNutrient {
                record_ref: d.id().to_string(),
                nutrient_key: field(&f, NUTRIENT_KEY).unwrap_or_default(),
                name: d.name().to_string(),
                unit: field(&f, UNIT).unwrap_or_default(),
                basis: field(&f, BASIS).unwrap_or_default(),
                method: field(&f, METHOD),
                display_order: field(&f, DISPLAY_ORDER),
                default_display: field(&f, DEFAULT_DISPLAY),
            }
        })
        .collect()
}

fn draft_values(draft: &DraftRecords) -> Vec<DraftValue> {
    use draft_schema::*;
    draft
        .records
        .iter()
        .map(|d| {
            let f = draft.fields(d);
            DraftValue {
                record_ref: d.id().to_string(),
                ingredient_key: field(&f, INGREDIENT_KEY)
                    .unwrap_or_default(),
                nutrient_key: field(&f, NUTRIENT_KEY).unwrap_or_default(),
                value_status: field(&f, VALUE_STATUS).unwrap_or_default(),
                amount: field(&f, AMOUNT),
                raw_notation: field(&f, RAW_NOTATION),
            }
        })
        .collect()
}

#[async_trait::async_trait]
impl PublishIngredientReleaseInputPort for PublishIngredientRelease {
    #[tracing::instrument(
        name = "PublishIngredientRelease::execute",
        skip_all
    )]
    async fn execute<'a>(
        &self,
        input: PublishIngredientReleaseInputData<'a>,
    ) -> errors::Result<(IngredientRelease, bool)> {
        self.lookup
            .authorize_write(input.executor, input.multi_tenancy)
            .await?;
        input.source.validate()?;
        let (catalog, repos) = self
            .lookup
            .catalog(&input.org_username, &input.catalog_key)
            .await?;
        let tenant_id = catalog.tenant_id().clone();

        let draft = DraftCatalog {
            ingredients: draft_ingredients(
                &self.read_all(&input, &tenant_id, &repos[0]).await?,
            ),
            nutrients: draft_nutrients(
                &self.read_all(&input, &tenant_id, &repos[1]).await?,
            ),
            values: draft_values(
                &self.read_all(&input, &tenant_id, &repos[2]).await?,
            ),
        };
        let snapshot = ReleaseSnapshot::build(&draft)?;

        let existing = self
            .lookup
            .catalogs
            .find_release_by_source(
                &tenant_id,
                catalog.id(),
                &input.source.source_id,
                &input.source.source_release,
            )
            .await?;
        if let Some(existing) = existing {
            if existing.content_hash() == snapshot.content_hash() {
                return Ok((existing, false));
            }
            return Err(errors::Error::conflict(format!(
                "release {}/{} is already published with different content; \
                 publish the change under a new source_release",
                input.source.source_id, input.source.source_release
            )));
        }

        let release = IngredientRelease::publish(
            &catalog,
            input.source,
            &snapshot,
            input.executor.get_id(),
        )?;
        self.lookup
            .catalogs
            .insert_release(&release, &snapshot)
            .await?;
        Ok((release, true))
    }
}

// ==================== Read ====================

#[derive(Debug, Clone)]
pub struct ReadIngredientCatalog {
    lookup: CatalogLookup,
}

impl ReadIngredientCatalog {
    pub fn new(
        get_org_by_username: Arc<dyn GetOrganizationByUsernameQuery>,
        get_repo_by_username: Arc<dyn GetRepoByUsernameQuery>,
        repo_repo: Arc<dyn RepoRepository>,
        catalogs: Arc<dyn IngredientCatalogRepository>,
        auth: Arc<dyn AuthApp>,
    ) -> Arc<Self> {
        Arc::new(Self {
            lookup: CatalogLookup {
                get_org_by_username,
                get_repo_by_username,
                repo_repo,
                catalogs,
                auth,
            },
        })
    }

    async fn readable_catalog(
        &self,
        target: CatalogTarget<'_>,
    ) -> errors::Result<IngredientCatalog> {
        let (catalog, repos) = self
            .lookup
            .catalog(target.org_username, target.catalog_key)
            .await?;
        self.lookup
            .authorize_read(target.executor, target.multi_tenancy, &repos)
            .await?;
        Ok(catalog)
    }

    async fn readable_release(
        &self,
        target: CatalogTarget<'_>,
        release_id: &str,
    ) -> errors::Result<IngredientRelease> {
        let catalog = self.readable_catalog(target).await?;
        let release_id: IngredientReleaseId = release_id
            .parse()
            .map_err(|_| errors::Error::not_found("ingredient release"))?;
        self.lookup
            .catalogs
            .get_release(catalog.tenant_id(), catalog.id(), &release_id)
            .await?
            .ok_or_else(|| errors::Error::not_found("ingredient release"))
    }
}

#[async_trait::async_trait]
impl ReadIngredientCatalogInputPort for ReadIngredientCatalog {
    #[tracing::instrument(
        name = "ReadIngredientCatalog::list_releases",
        skip_all
    )]
    async fn list_releases<'a>(
        &self,
        target: CatalogTarget<'a>,
    ) -> errors::Result<(IngredientCatalog, Vec<IngredientRelease>)> {
        let catalog = self.readable_catalog(target).await?;
        let releases = self
            .lookup
            .catalogs
            .list_releases(catalog.tenant_id(), catalog.id())
            .await?;
        Ok((catalog, releases))
    }

    #[tracing::instrument(
        name = "ReadIngredientCatalog::get_release",
        skip_all
    )]
    async fn get_release<'a>(
        &self,
        target: CatalogTarget<'a>,
        release_id: &str,
    ) -> errors::Result<(IngredientRelease, Vec<ReleasedNutrient>)> {
        let release = self.readable_release(target, release_id).await?;
        let nutrients =
            self.lookup.catalogs.list_nutrients(&release).await?;
        Ok((release, nutrients))
    }

    #[tracing::instrument(
        name = "ReadIngredientCatalog::search_ingredients",
        skip_all
    )]
    async fn search_ingredients<'a>(
        &self,
        target: CatalogTarget<'a>,
        release_id: &str,
        search: IngredientSearch,
    ) -> errors::Result<(Vec<ReleasedIngredient>, u64, IngredientSearch)>
    {
        let search = normalize_search(search)?;
        let release = self.readable_release(target, release_id).await?;
        let (items, total) = self
            .lookup
            .catalogs
            .search_ingredients(&release, &search)
            .await?;
        Ok((items, total, search))
    }

    #[tracing::instrument(
        name = "ReadIngredientCatalog::get_ingredient",
        skip_all
    )]
    async fn get_ingredient<'a>(
        &self,
        target: CatalogTarget<'a>,
        release_id: &str,
        ingredient_key: &str,
    ) -> errors::Result<ReleasedIngredientDetail> {
        let release = self.readable_release(target, release_id).await?;
        let ingredient = self
            .lookup
            .catalogs
            .get_ingredient(&release, ingredient_key)
            .await?
            .ok_or_else(|| errors::Error::not_found("ingredient"))?;
        let nutrients =
            self.lookup.catalogs.list_nutrients(&release).await?;
        let values = self
            .lookup
            .catalogs
            .list_values(&release, ingredient_key)
            .await?;
        Ok(ReleasedIngredientDetail {
            values: pair_values(&ingredient, nutrients, values),
            release,
            ingredient,
        })
    }
}

/// Validate paging and trim filters before they reach SQL.
fn normalize_search(
    search: IngredientSearch,
) -> errors::Result<IngredientSearch> {
    let page = if search.page == 0 { 1 } else { search.page };
    let page_size = if search.page_size == 0 {
        value_object::DEFAULT_PAGE_SIZE
    } else {
        search.page_size
    };
    if !(1..=MAX_PAGE_SIZE).contains(&page_size) {
        return Err(errors::Error::invalid(format!(
            "page_size must be between 1 and {MAX_PAGE_SIZE}"
        )));
    }
    let trimmed = |s: Option<String>| {
        s.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
    };
    let text = trimmed(search.text);
    if text.as_ref().is_some_and(|t| t.chars().count() > 255) {
        return Err(errors::Error::invalid("search text is too long"));
    }
    Ok(IngredientSearch {
        text,
        category_code: trimmed(search.category_code),
        cooking_state: trimmed(search.cooking_state),
        page,
        page_size,
    })
}

/// One entry per nutrient of the release, in display order. A nutrient
/// with no stored value is reported as `NotListed`, never as 0.
fn pair_values(
    ingredient: &ReleasedIngredient,
    nutrients: Vec<ReleasedNutrient>,
    values: Vec<ReleasedValue>,
) -> Vec<(ReleasedNutrient, ReleasedValue)> {
    let mut by_key: BTreeMap<String, ReleasedValue> = values
        .into_iter()
        .map(|v| (v.nutrient_key.clone(), v))
        .collect();
    nutrients
        .into_iter()
        .map(|n| {
            let value =
                by_key.remove(&n.nutrient_key).unwrap_or_else(|| {
                    ReleasedValue {
                        ingredient_key: ingredient.ingredient_key.clone(),
                        nutrient_key: n.nutrient_key.clone(),
                        status: NutrientValueStatus::NotListed,
                        amount: None,
                        raw_notation: None,
                    }
                });
            (n, value)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::fixtures;

    #[test]
    fn missing_values_are_not_listed_not_zero() {
        let snapshot =
            ReleaseSnapshot::build(&fixtures::onion_catalog()).unwrap();
        let boiled = snapshot
            .ingredients()
            .iter()
            .find(|i| i.ingredient_key == "onion-boiled")
            .unwrap();
        let values: Vec<_> = snapshot
            .values()
            .iter()
            .filter(|v| v.ingredient_key == "onion-boiled")
            .cloned()
            .collect();

        let paired =
            pair_values(boiled, snapshot.nutrients().to_vec(), values);

        assert_eq!(paired.len(), snapshot.nutrients().len());
        let energy = paired
            .iter()
            .find(|(n, _)| n.nutrient_key == "ENERC_KCAL")
            .unwrap();
        assert_eq!(energy.1.status, NutrientValueStatus::NotListed);
        assert_eq!(energy.1.amount, None);
    }

    #[test]
    fn search_defaults_and_limits() {
        let s = normalize_search(IngredientSearch {
            text: Some("  玉ねぎ ".into()),
            cooking_state: Some(" ".into()),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(s.page, 1);
        assert_eq!(s.page_size, value_object::DEFAULT_PAGE_SIZE);
        assert_eq!(s.text.as_deref(), Some("玉ねぎ"));
        assert_eq!(s.cooking_state, None);

        assert!(normalize_search(IngredientSearch {
            page_size: MAX_PAGE_SIZE + 1,
            ..Default::default()
        })
        .is_err());
    }
}
