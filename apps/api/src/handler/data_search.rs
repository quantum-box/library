//! `GET /v1beta/repos/{org}/{repo}/data-search`: location, filter and text
//! search for map and listing clients.

use std::sync::Arc;

use axum::{
    extract::{Extension, Path as AxumPath, Query},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use database_manager::domain::{Data, Property};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tachyon_sdk::auth::ExecutorAction;
use utoipa::{IntoParams, ToSchema};
use value_object::OffsetPaginator;

use crate::app::LibraryApp;
use crate::handler::library_executor_extractor::LibraryExecutor;
use crate::handler::types::{
    DataResponse, PropertyDataResponse,
    PropertyDataValue as PropertyDataValueResponse,
};
use crate::usecase::data_search::{DataSearchInputData, DataSearchQuery};
use crate::usecase::library_client_url::data_url;
use crate::usecase::LibraryOrg;

/// How long a shared cache may serve a public response before revalidating.
/// Search results trail Library edits by at most this long in a CDN or
/// browser; the server side is never stale.
const PUBLIC_MAX_AGE_SECONDS: u32 = 60;
const PUBLIC_STALE_WHILE_REVALIDATE_SECONDS: u32 = 300;

/// Query parameters of `data-search`.
///
/// Parsed by hand from the raw pairs rather than through serde, because
/// `filter` repeats and the form decoder `Query` uses cannot collect a
/// repeated key.
#[derive(Debug, Default, Deserialize, ToSchema, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct DataSearchParams {
    /// Text to find. Every whitespace-separated term must appear in the
    /// record name, a text Property, or the key or label of a selected
    /// option. Case- and width-insensitive (`ＤＣＭ` matches `dcm`).
    pub q: Option<String>,
    /// Viewport as `minLng,minLat,maxLng,maxLat` (GeoJSON order). Records
    /// without a location are excluded.
    #[param(example = "141.20,42.95,141.50,43.15")]
    pub bbox: Option<String>,
    /// Latitude of the point to measure distance from. Requires `lng`.
    pub lat: Option<f64>,
    /// Longitude of the point to measure distance from. Requires `lat`.
    pub lng: Option<f64>,
    /// Only return records within this many meters of `lat`/`lng`.
    pub radius_m: Option<f64>,
    /// Key of the Location Property that places a record on the map.
    /// Defaults to the repo's first Location Property.
    pub location_property: Option<String>,
    /// Property filter, repeatable; all must hold. `key:value` matches a
    /// value (Select/MultiSelect options by id, key or label; Boolean
    /// `true`/`false`, where no value counts as `false`; Relation by
    /// record id; text exactly). `key:a|b` matches any of several.
    /// `key>=n` and `key<=n` compare Integer and `YYYY-MM-DD` Date values.
    #[param(example = "category:home_center")]
    pub filter: Option<Vec<String>>,
    /// Comma-separated record ids (at most 100). Fetches specific records
    /// under the same publication rules as the listing, e.g. a spot detail.
    pub ids: Option<String>,
    /// `distance` (default when `lat`/`lng` are given), `name` (default
    /// otherwise) or `updated` (newest first).
    pub sort: Option<String>,
    /// Include records whose `publication_status` is not `published`.
    /// Requires permission to edit the repo.
    #[serde(default)]
    pub include_unpublished: bool,
    /// Return rich text documents in full instead of as previews.
    #[serde(default)]
    pub include_body: bool,
    /// 1-origin page number. Defaults to 1.
    #[param(minimum = 1)]
    pub page: Option<u32>,
    /// Number of records per page. Defaults to 20 and is capped at 100.
    #[param(minimum = 1, maximum = 100)]
    pub page_size: Option<u32>,
}

impl DataSearchParams {
    fn from_pairs(pairs: &[(String, String)]) -> errors::Result<Self> {
        let mut params = Self::default();
        let mut filters = Vec::new();
        for (key, value) in pairs {
            match key.as_str() {
                "q" => params.q = Some(value.clone()),
                "bbox" => params.bbox = Some(value.clone()),
                "lat" => params.lat = Some(parse_number(key, value)?),
                "lng" => params.lng = Some(parse_number(key, value)?),
                "radius_m" => {
                    params.radius_m = Some(parse_number(key, value)?)
                }
                "location_property" => {
                    params.location_property = Some(value.clone())
                }
                "filter" => filters.push(value.clone()),
                "ids" => params.ids = Some(value.clone()),
                "sort" => params.sort = Some(value.clone()),
                "include_unpublished" => {
                    params.include_unpublished = parse_flag(key, value)?
                }
                "include_body" => {
                    params.include_body = parse_flag(key, value)?
                }
                "page" => params.page = Some(parse_number(key, value)?),
                "page_size" => {
                    params.page_size = Some(parse_number(key, value)?)
                }
                // Unknown parameters, such as a client's cache buster,
                // are ignored.
                _ => {}
            }
        }
        params.filter = Some(filters);
        Ok(params)
    }

    fn to_query(&self) -> errors::Result<DataSearchQuery> {
        DataSearchQuery::parse(
            self.q.as_deref(),
            self.bbox.as_deref(),
            self.lat,
            self.lng,
            self.radius_m,
            self.location_property.as_deref(),
            self.filter.as_deref().unwrap_or_default(),
            self.ids.as_deref(),
            self.sort.as_deref(),
        )
    }
}

fn parse_number<T: std::str::FromStr>(
    key: &str,
    value: &str,
) -> errors::Result<T> {
    value.trim().parse().map_err(|_| {
        errors::Error::invalid(format!("{key} must be a number"))
    })
}

fn parse_flag(key: &str, value: &str) -> errors::Result<bool> {
    match value {
        "true" | "1" | "" => Ok(true),
        "false" | "0" => Ok(false),
        _ => Err(errors::Error::invalid(format!(
            "{key} must be true or false"
        ))),
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct DataSearchResponse {
    pub data: Vec<DataSearchHitResponse>,
    pub paginator: OffsetPaginator,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DataSearchHitResponse {
    #[serde(flatten)]
    pub data: DataResponse,
    /// Meters from `lat`/`lng`; absent when the query has no point.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distance_meters: Option<f64>,
}

#[utoipa::path(
    get,
    path = "/v1beta/repos/{org}/{repo}/data-search",
    params(
        ("org" = String, Path, description = "Organization username"),
        ("repo" = String, Path, description = "Repository username"),
        DataSearchParams
    ),
    responses(
        (status = 200, description = "Matching records, one page at a \
time. Public repos answer with `Cache-Control: public` and an `ETag`; send \
it back in `If-None-Match` to get a 304 while nothing has changed", body = DataSearchResponse),
        (status = 304, description = "Unchanged since the ETag in `If-None-Match`"),
        (status = 400, description = "Malformed query, unknown Property, or a Property that cannot be filtered that way"),
        (status = 403, description = "Private repo, or `include_unpublished` without edit permission"),
        (status = 404, description = "Repository not found")
    )
)]
pub async fn search_data_index(
    AxumPath((org, repo)): AxumPath<(String, String)>,
    Query(pairs): Query<Vec<(String, String)>>,
    headers: HeaderMap,
    Extension(library_app): Extension<Arc<LibraryApp>>,
    executor: LibraryExecutor,
    library_org: LibraryOrg,
) -> errors::Result<Response> {
    let params = DataSearchParams::from_pairs(&pairs)?;
    let input = DataSearchInputData {
        executor: &executor,
        multi_tenancy: &library_org,
        org_username: &org,
        repo_username: &repo,
        query: params.to_query()?,
        include_unpublished: params.include_unpublished,
        page: params.page,
        page_size: params.page_size,
    };
    let output = library_app.data_search.execute(&input).await?;

    let etag = etag(&output.fingerprint, &pairs);
    let cache_control = if output.publicly_cacheable && executor.is_none() {
        format!(
            "public, max-age={PUBLIC_MAX_AGE_SECONDS}, \
             stale-while-revalidate={PUBLIC_STALE_WHILE_REVALIDATE_SECONDS}"
        )
    } else {
        "private, no-cache".to_string()
    };
    let cache_headers = [
        (header::ETAG, header_value(&etag)?),
        (header::CACHE_CONTROL, header_value(&cache_control)?),
        (header::VARY, HeaderValue::from_static("Authorization")),
    ];

    if if_none_match(&headers, &etag) {
        return Ok(
            (StatusCode::NOT_MODIFIED, cache_headers).into_response()
        );
    }

    let data = output
        .hits
        .into_iter()
        .map(|hit| DataSearchHitResponse {
            data: data_response(
                &org,
                &repo,
                hit.data,
                &output.properties,
                params.include_body,
            ),
            distance_meters: hit.distance_meters.map(|m| m.round()),
        })
        .collect();
    Ok((
        StatusCode::OK,
        cache_headers,
        Json(DataSearchResponse {
            data,
            paginator: output.paginator,
        }),
    )
        .into_response())
}

fn data_response(
    org: &str,
    repo: &str,
    data: Data,
    properties: &[Property],
    include_body: bool,
) -> DataResponse {
    DataResponse {
        id: data.id().to_string(),
        name: data.name().to_string(),
        record_version: data.record_version().to_string(),
        url: data_url(org, repo, data.id().as_ref()),
        items: data
            .property_data()
            .iter()
            .filter_map(|p| {
                let property = properties
                    .iter()
                    .find(|prop| prop.id() == p.property_id())?;
                Some(PropertyDataResponse {
                    property_id: p.property_id().to_string(),
                    key: property.name().to_string(),
                    value: p.value().clone().map(|v| {
                        if include_body {
                            v.into()
                        } else {
                            PropertyDataValueResponse::for_list(v)
                        }
                    }),
                })
            })
            .collect(),
    }
}

/// The validator for one response. Covers the data and visibility the
/// usecase reported, and every query parameter in a canonical order, so
/// two spellings of the same query share it.
fn etag(fingerprint: &str, pairs: &[(String, String)]) -> String {
    let mut pairs = pairs.to_vec();
    pairs.sort();
    let mut hasher = Sha256::new();
    hasher.update(fingerprint);
    for (key, value) in &pairs {
        hasher.update([0]);
        hasher.update(key);
        hasher.update([0]);
        hasher.update(value);
    }
    format!("\"{}\"", hex::encode(&hasher.finalize()[..16]))
}

fn if_none_match(headers: &HeaderMap, etag: &str) -> bool {
    headers
        .get_all(header::IF_NONE_MATCH)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .map(|tag| tag.trim().trim_start_matches("W/"))
        .any(|tag| tag == etag || tag == "*")
}

fn header_value(value: &str) -> errors::Result<HeaderValue> {
    HeaderValue::from_str(value)
        .map_err(|e| errors::Error::internal_server_error(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pairs(items: &[(&str, &str)]) -> Vec<(String, String)> {
        items
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn repeated_filters_are_all_kept() {
        let params = DataSearchParams::from_pairs(&pairs(&[
            ("filter", "category:home_center"),
            ("filter", "large_dog_allowed:true"),
            ("lat", "43.06"),
            ("lng", "141.35"),
            ("_", "cache-buster"),
        ]))
        .unwrap();
        let query = params.to_query().unwrap();
        assert_eq!(query.filters.len(), 2);
        assert!(query.near.is_some());
    }

    #[test]
    fn non_numeric_coordinates_are_rejected() {
        assert!(DataSearchParams::from_pairs(&pairs(&[("lat", "north")]))
            .is_err());
    }

    #[test]
    fn etag_ignores_parameter_order_but_not_values() {
        let a = etag("fp", &pairs(&[("q", "dcm"), ("page", "1")]));
        let b = etag("fp", &pairs(&[("page", "1"), ("q", "dcm")]));
        let c = etag("fp", &pairs(&[("page", "2"), ("q", "dcm")]));
        let d = etag("fp2", &pairs(&[("page", "1"), ("q", "dcm")]));
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, d);
    }

    #[test]
    fn if_none_match_accepts_weak_and_listed_tags() {
        let tag = "\"abc\"";
        let mut headers = HeaderMap::new();
        headers.insert(
            header::IF_NONE_MATCH,
            HeaderValue::from_static("\"zzz\", W/\"abc\""),
        );
        assert!(if_none_match(&headers, tag));
        assert!(!if_none_match(&HeaderMap::new(), tag));
    }
}
