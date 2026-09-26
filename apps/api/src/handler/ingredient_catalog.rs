//! COM-860: REST API for the common ingredient master.
//!
//! Consumers pin an ingredient by `ingredient_key` plus `release_id`: the
//! same pair always returns the same values, however the draft repos are
//! edited afterwards. Amounts are decimal strings (never floats), and
//! every value carries a `value_status` so `Tr`, `-` and `0` stay distinct.

use std::sync::Arc;

use axum::{
    extract::{Extension, Path as AxumPath, Query},
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::app::LibraryApp;
use crate::domain::{
    IngredientCatalog, IngredientRelease, IngredientSearch, ReleaseSource,
    ReleasedIngredient, ReleasedNutrient, ReleasedValue,
};
use crate::handler::library_executor_extractor::LibraryExecutor;
use crate::usecase::{
    CatalogTarget, CreateIngredientCatalogInputData, LibraryOrg,
    PublishIngredientReleaseInputData,
};

fn target<'a>(
    executor: &'a LibraryExecutor,
    library_org: &'a LibraryOrg,
    org: &'a str,
    catalog: &'a str,
) -> CatalogTarget<'a> {
    CatalogTarget {
        executor,
        multi_tenancy: library_org,
        org_username: org,
        catalog_key: catalog,
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateIngredientCatalogRequest {
    /// URL-safe key, unique within the org, e.g. `food-composition`.
    pub catalog_key: String,
    pub name: String,
    /// Username of the repo holding ingredient records.
    pub ingredient_repo: String,
    /// Username of the repo holding nutrient definitions.
    pub nutrient_repo: String,
    /// Username of the repo holding ingredient x nutrient values.
    pub value_repo: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct IngredientCatalogResponse {
    pub id: String,
    pub catalog_key: String,
    pub name: String,
    pub ingredient_repo_id: String,
    pub nutrient_repo_id: String,
    pub value_repo_id: String,
    pub created_at: String,
}

impl From<&IngredientCatalog> for IngredientCatalogResponse {
    fn from(c: &IngredientCatalog) -> Self {
        Self {
            id: c.id().to_string(),
            catalog_key: c.catalog_key().clone(),
            name: c.name().clone(),
            ingredient_repo_id: c.ingredient_repo_id().to_string(),
            nutrient_repo_id: c.nutrient_repo_id().to_string(),
            value_repo_id: c.value_repo_id().to_string(),
            created_at: c.created_at().to_rfc3339(),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PublishIngredientReleaseRequest {
    /// Dataset identifier, e.g. `mext-sfct-2023`.
    pub source_id: String,
    /// Edition label including applied errata, e.g.
    /// `2023+errata-2026-03-27`. A new label is needed for new content.
    pub source_release: String,
    pub source_url: Option<String>,
    /// RFC 3339 time the source files were downloaded.
    pub source_retrieved_at: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct IngredientReleaseResponse {
    pub id: String,
    pub catalog_id: String,
    pub source_id: String,
    pub source_release: String,
    pub source_url: Option<String>,
    pub source_retrieved_at: Option<String>,
    pub notes: Option<String>,
    pub schema_version: u32,
    /// `sha256:<hex>` over the frozen rows.
    pub content_hash: String,
    pub ingredient_count: u32,
    pub nutrient_count: u32,
    pub value_count: u32,
    pub published_by: String,
    pub published_at: String,
}

impl From<&IngredientRelease> for IngredientReleaseResponse {
    fn from(r: &IngredientRelease) -> Self {
        let s = r.source();
        Self {
            id: r.id().to_string(),
            catalog_id: r.catalog_id().to_string(),
            source_id: s.source_id.clone(),
            source_release: s.source_release.clone(),
            source_url: s.source_url.clone(),
            source_retrieved_at: s
                .source_retrieved_at
                .map(|t| t.to_rfc3339()),
            notes: s.notes.clone(),
            schema_version: *r.schema_version(),
            content_hash: r.content_hash().clone(),
            ingredient_count: *r.ingredient_count(),
            nutrient_count: *r.nutrient_count(),
            value_count: *r.value_count(),
            published_by: r.published_by().clone(),
            published_at: r.published_at().to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PublishIngredientReleaseResponse {
    pub release: IngredientReleaseResponse,
    /// False when an identical release already existed (safe retry).
    pub created: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct IngredientReleaseListResponse {
    pub catalog: IngredientCatalogResponse,
    /// Newest first.
    pub releases: Vec<IngredientReleaseResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct NutrientResponse {
    pub nutrient_key: String,
    pub name: String,
    pub unit: String,
    pub basis: String,
    pub method: Option<String>,
    pub display_order: i32,
    pub default_display: bool,
}

impl From<&ReleasedNutrient> for NutrientResponse {
    fn from(n: &ReleasedNutrient) -> Self {
        Self {
            nutrient_key: n.nutrient_key.clone(),
            name: n.name.clone(),
            unit: n.unit.clone(),
            basis: n.basis.clone(),
            method: n.method.clone(),
            display_order: n.display_order,
            default_display: n.default_display,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct IngredientReleaseDetailResponse {
    pub release: IngredientReleaseResponse,
    /// In display order.
    pub nutrients: Vec<NutrientResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct IngredientResponse {
    pub ingredient_key: String,
    /// Food number in the source; text so leading zeros survive.
    pub source_food_code: String,
    /// Food name exactly as in the source.
    pub original_name: String,
    pub standard_name: Option<String>,
    pub reading: Option<String>,
    pub aliases: Vec<String>,
    pub category_code: Option<String>,
    pub category_name: Option<String>,
    pub part: Option<String>,
    pub cooking_state: Option<String>,
    pub skin_bone: Option<String>,
    /// Source refuse rate (%) for edible-portion conversion. Not a store
    /// food-loss rate.
    pub refuse_rate: Option<String>,
    /// Whether parsed attributes were checked by a person.
    pub attribute_review_status: String,
}

impl From<&ReleasedIngredient> for IngredientResponse {
    fn from(i: &ReleasedIngredient) -> Self {
        Self {
            ingredient_key: i.ingredient_key.clone(),
            source_food_code: i.source_food_code.clone(),
            original_name: i.original_name.clone(),
            standard_name: i.standard_name.clone(),
            reading: i.reading.clone(),
            aliases: i.aliases.clone(),
            category_code: i.category_code.clone(),
            category_name: i.category_name.clone(),
            part: i.part.clone(),
            cooking_state: i.cooking_state.clone(),
            skin_bone: i.skin_bone.clone(),
            refuse_rate: i.refuse_rate.as_ref().map(|r| r.to_string()),
            attribute_review_status: i
                .attribute_review_status
                .as_str()
                .to_string(),
        }
    }
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct SearchIngredientsQuery {
    /// Substring of the source name, standard name, reading or an alias.
    pub q: Option<String>,
    pub category_code: Option<String>,
    /// e.g. `raw`, `boiled`. Alias matches do not pick a state for you.
    pub cooking_state: Option<String>,
    /// 1-origin. Defaults to 1.
    pub page: Option<u32>,
    /// 1..=100. Defaults to 20.
    pub page_size: Option<u32>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct IngredientSearchResponse {
    pub release_id: String,
    pub ingredients: Vec<IngredientResponse>,
    pub page: u32,
    pub page_size: u32,
    pub total_items: u64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct NutrientValueResponse {
    pub nutrient: NutrientResponse,
    /// measured, estimated, zero, estimated_zero, trace, estimated_trace,
    /// not_measured, or not_listed (no value in this release).
    pub value_status: String,
    /// Normalized decimal string. Null for trace, not measured and not
    /// listed: those are not zero.
    pub amount: Option<String>,
    /// Cell text as in the source, when recorded.
    pub raw_notation: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct IngredientDetailResponse {
    pub release: IngredientReleaseResponse,
    pub ingredient: IngredientResponse,
    /// One entry per nutrient of the release, in display order.
    pub values: Vec<NutrientValueResponse>,
}

fn value_response(
    n: &ReleasedNutrient,
    v: &ReleasedValue,
) -> NutrientValueResponse {
    NutrientValueResponse {
        nutrient: NutrientResponse::from(n),
        value_status: v.status.as_str().to_string(),
        amount: v.amount.as_ref().map(|a| a.to_string()),
        raw_notation: v.raw_notation.clone(),
    }
}

/// `POST /v1beta/orgs/{org}/ingredient-catalogs`
#[utoipa::path(
    post,
    path = "/v1beta/orgs/{org}/ingredient-catalogs",
    params(("org" = String, Path, description = "Organization username")),
    request_body = CreateIngredientCatalogRequest,
    responses(
        (status = 200, description = "Catalog registered", body = IngredientCatalogResponse),
        (status = 403, description = "Caller may not update repos"),
        (status = 404, description = "Organization or a repo not found in the organization"),
        (status = 409, description = "catalog_key already exists"),
    ),
    tag = "ingredient-catalogs"
)]
#[axum::debug_handler]
pub async fn create_ingredient_catalog(
    AxumPath(org): AxumPath<String>,
    Extension(library_app): Extension<Arc<LibraryApp>>,
    executor: LibraryExecutor,
    library_org: LibraryOrg,
    Json(payload): Json<CreateIngredientCatalogRequest>,
) -> errors::Result<Json<IngredientCatalogResponse>> {
    let catalog = library_app
        .create_ingredient_catalog
        .execute(CreateIngredientCatalogInputData {
            executor: &executor,
            multi_tenancy: &library_org,
            org_username: org,
            catalog_key: payload.catalog_key,
            name: payload.name,
            ingredient_repo: payload.ingredient_repo,
            nutrient_repo: payload.nutrient_repo,
            value_repo: payload.value_repo,
        })
        .await?;
    Ok(Json(IngredientCatalogResponse::from(&catalog)))
}

/// `POST /v1beta/orgs/{org}/ingredient-catalogs/{catalog}/releases`
#[utoipa::path(
    post,
    path = "/v1beta/orgs/{org}/ingredient-catalogs/{catalog}/releases",
    params(
        ("org" = String, Path, description = "Organization username"),
        ("catalog" = String, Path, description = "Catalog key"),
    ),
    request_body = PublishIngredientReleaseRequest,
    responses(
        (status = 200, description = "Release published, or the identical existing release", body = PublishIngredientReleaseResponse),
        (status = 400, description = "Draft records failed validation; nothing was published"),
        (status = 403, description = "Caller may not update repos"),
        (status = 404, description = "Organization or catalog not found"),
        (status = 409, description = "source_release already published with different content"),
    ),
    tag = "ingredient-catalogs"
)]
#[axum::debug_handler]
pub async fn publish_ingredient_release(
    AxumPath((org, catalog)): AxumPath<(String, String)>,
    Extension(library_app): Extension<Arc<LibraryApp>>,
    executor: LibraryExecutor,
    library_org: LibraryOrg,
    Json(payload): Json<PublishIngredientReleaseRequest>,
) -> errors::Result<Json<PublishIngredientReleaseResponse>> {
    let source_retrieved_at = payload
        .source_retrieved_at
        .as_deref()
        .map(|t| {
            chrono::DateTime::parse_from_rfc3339(t)
                .map(|t| t.with_timezone(&chrono::Utc))
                .map_err(|e| {
                    errors::Error::invalid(format!(
                        "source_retrieved_at must be RFC 3339: {e}"
                    ))
                })
        })
        .transpose()?;
    let trimmed = |s: Option<String>| {
        s.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
    };
    let (release, created) = library_app
        .publish_ingredient_release
        .execute(PublishIngredientReleaseInputData {
            executor: &executor,
            multi_tenancy: &library_org,
            org_username: org,
            catalog_key: catalog,
            source: ReleaseSource {
                source_id: payload.source_id.trim().to_string(),
                source_release: payload.source_release.trim().to_string(),
                source_url: trimmed(payload.source_url),
                source_retrieved_at,
                notes: trimmed(payload.notes),
            },
        })
        .await?;
    Ok(Json(PublishIngredientReleaseResponse {
        release: IngredientReleaseResponse::from(&release),
        created,
    }))
}

/// `GET /v1beta/orgs/{org}/ingredient-catalogs/{catalog}/releases`
#[utoipa::path(
    get,
    path = "/v1beta/orgs/{org}/ingredient-catalogs/{catalog}/releases",
    params(
        ("org" = String, Path, description = "Organization username"),
        ("catalog" = String, Path, description = "Catalog key"),
    ),
    responses(
        (status = 200, description = "Catalog and its releases, newest first", body = IngredientReleaseListResponse),
        (status = 403, description = "A draft repo is private and the caller may not read it"),
        (status = 404, description = "Organization or catalog not found"),
    ),
    tag = "ingredient-catalogs"
)]
#[axum::debug_handler]
pub async fn list_ingredient_releases(
    AxumPath((org, catalog)): AxumPath<(String, String)>,
    Extension(library_app): Extension<Arc<LibraryApp>>,
    executor: LibraryExecutor,
    library_org: LibraryOrg,
) -> errors::Result<Json<IngredientReleaseListResponse>> {
    let (catalog, releases) = library_app
        .read_ingredient_catalog
        .list_releases(target(&executor, &library_org, &org, &catalog))
        .await?;
    Ok(Json(IngredientReleaseListResponse {
        catalog: IngredientCatalogResponse::from(&catalog),
        releases: releases
            .iter()
            .map(IngredientReleaseResponse::from)
            .collect(),
    }))
}

/// `GET /v1beta/orgs/{org}/ingredient-catalogs/{catalog}/releases/{release_id}`
#[utoipa::path(
    get,
    path = "/v1beta/orgs/{org}/ingredient-catalogs/{catalog}/releases/{release_id}",
    params(
        ("org" = String, Path, description = "Organization username"),
        ("catalog" = String, Path, description = "Catalog key"),
        ("release_id" = String, Path, description = "Release ID (irel_)"),
    ),
    responses(
        (status = 200, description = "Release metadata and its nutrient definitions", body = IngredientReleaseDetailResponse),
        (status = 403, description = "A draft repo is private and the caller may not read it"),
        (status = 404, description = "Organization, catalog or release not found"),
    ),
    tag = "ingredient-catalogs"
)]
#[axum::debug_handler]
pub async fn get_ingredient_release(
    AxumPath((org, catalog, release_id)): AxumPath<(
        String,
        String,
        String,
    )>,
    Extension(library_app): Extension<Arc<LibraryApp>>,
    executor: LibraryExecutor,
    library_org: LibraryOrg,
) -> errors::Result<Json<IngredientReleaseDetailResponse>> {
    let (release, nutrients) = library_app
        .read_ingredient_catalog
        .get_release(
            target(&executor, &library_org, &org, &catalog),
            &release_id,
        )
        .await?;
    Ok(Json(IngredientReleaseDetailResponse {
        release: IngredientReleaseResponse::from(&release),
        nutrients: nutrients.iter().map(NutrientResponse::from).collect(),
    }))
}

/// `GET /v1beta/orgs/{org}/ingredient-catalogs/{catalog}/releases/{release_id}/ingredients`
#[utoipa::path(
    get,
    path = "/v1beta/orgs/{org}/ingredient-catalogs/{catalog}/releases/{release_id}/ingredients",
    params(
        ("org" = String, Path, description = "Organization username"),
        ("catalog" = String, Path, description = "Catalog key"),
        ("release_id" = String, Path, description = "Release ID (irel_)"),
        SearchIngredientsQuery,
    ),
    responses(
        (status = 200, description = "One page of matching ingredients, ordered by source food code", body = IngredientSearchResponse),
        (status = 400, description = "Invalid paging"),
        (status = 403, description = "A draft repo is private and the caller may not read it"),
        (status = 404, description = "Organization, catalog or release not found"),
    ),
    tag = "ingredient-catalogs"
)]
#[axum::debug_handler]
pub async fn search_released_ingredients(
    AxumPath((org, catalog, release_id)): AxumPath<(
        String,
        String,
        String,
    )>,
    Query(query): Query<SearchIngredientsQuery>,
    Extension(library_app): Extension<Arc<LibraryApp>>,
    executor: LibraryExecutor,
    library_org: LibraryOrg,
) -> errors::Result<Json<IngredientSearchResponse>> {
    let (items, total, search) = library_app
        .read_ingredient_catalog
        .search_ingredients(
            target(&executor, &library_org, &org, &catalog),
            &release_id,
            IngredientSearch {
                text: query.q,
                category_code: query.category_code,
                cooking_state: query.cooking_state,
                page: query.page.unwrap_or(0),
                page_size: query.page_size.unwrap_or(0),
            },
        )
        .await?;
    Ok(Json(IngredientSearchResponse {
        release_id,
        ingredients: items.iter().map(IngredientResponse::from).collect(),
        page: search.page,
        page_size: search.page_size,
        total_items: total,
    }))
}

/// `GET /v1beta/orgs/{org}/ingredient-catalogs/{catalog}/releases/{release_id}/ingredients/{ingredient_key}`
#[utoipa::path(
    get,
    path = "/v1beta/orgs/{org}/ingredient-catalogs/{catalog}/releases/{release_id}/ingredients/{ingredient_key}",
    params(
        ("org" = String, Path, description = "Organization username"),
        ("catalog" = String, Path, description = "Catalog key"),
        ("release_id" = String, Path, description = "Release ID (irel_)"),
        ("ingredient_key" = String, Path, description = "Stable ingredient key"),
    ),
    responses(
        (status = 200, description = "The ingredient with a value entry for every nutrient of the release", body = IngredientDetailResponse),
        (status = 403, description = "A draft repo is private and the caller may not read it"),
        (status = 404, description = "Organization, catalog, release or ingredient not found"),
    ),
    tag = "ingredient-catalogs"
)]
#[axum::debug_handler]
pub async fn get_released_ingredient(
    AxumPath((org, catalog, release_id, ingredient_key)): AxumPath<(
        String,
        String,
        String,
        String,
    )>,
    Extension(library_app): Extension<Arc<LibraryApp>>,
    executor: LibraryExecutor,
    library_org: LibraryOrg,
) -> errors::Result<Json<IngredientDetailResponse>> {
    let detail = library_app
        .read_ingredient_catalog
        .get_ingredient(
            target(&executor, &library_org, &org, &catalog),
            &release_id,
            &ingredient_key,
        )
        .await?;
    Ok(Json(IngredientDetailResponse {
        release: IngredientReleaseResponse::from(&detail.release),
        ingredient: IngredientResponse::from(&detail.ingredient),
        values: detail
            .values
            .iter()
            .map(|(n, v)| value_response(n, v))
            .collect(),
    }))
}
