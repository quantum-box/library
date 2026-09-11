//! Read-only share links for one document in a private repo.
//!
//! Three of these routes manage links and are authenticated like any
//! other repo write. The fourth, `GET /v1beta/share/{token}`, is
//! deliberately unauthenticated: the unguessable token in the URL is the
//! credential, the same arrangement the image route uses, and it answers
//! with exactly one document.

use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::{Extension, Path as AxumPath},
    http::{header::AUTHORIZATION, HeaderMap},
    Json,
};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::app::LibraryApp;
use crate::domain::ShareLink;
use crate::handler::library_executor_extractor::{
    LibraryExecutor, LibraryExecutorKind,
};
use crate::handler::types::{
    property_select_options, DataResponse, PropertyDataResponse,
    PropertyResponse,
};
use crate::ttl_cache::TtlCache;
use crate::usecase::{
    library_client_url::share_url, CreateShareLinkInputData, LibraryOrg,
    ListShareLinksInputData, RevokeShareLinkInputData, ShareLinkRepoTarget,
    SharedData, ViewDataInputData,
};

const SLACK_UNFURL_ISSUER_ENV: &str = "LIBRARY_SLACK_UNFURL_ISSUER";
const SLACK_UNFURL_AUDIENCE_ENV: &str = "LIBRARY_SLACK_UNFURL_AUDIENCE";
const SLACK_UNFURL_AUTH_SECRET_ENV: &str =
    "LIBRARY_SLACK_UNFURL_AUTH_SECRET";
const DEFAULT_SLACK_UNFURL_ISSUER: &str = "tachyon-slack-unfurl";
const DEFAULT_SLACK_UNFURL_AUDIENCE: &str =
    "library-slack-unfurl-projection";
const SLACK_UNFURL_REPLAY_TTL: Duration = Duration::from_secs(300);
const SLACK_UNFURL_REPLAY_CAPACITY: usize = 4096;

static SLACK_UNFURL_JTI_CACHE: Lazy<TtlCache<String, ()>> =
    Lazy::new(|| {
        TtlCache::new(SLACK_UNFURL_REPLAY_TTL, SLACK_UNFURL_REPLAY_CAPACITY)
    });

#[derive(Deserialize, ToSchema)]
pub struct CreateShareLinkRequest {
    /// Label shown beside the link in the owner's list, for telling
    /// several links to the same document apart. Optional.
    pub name: Option<String>,
}

/// One link, as its owner sees it.
///
/// Carries no token: only the SHA-256 is stored, so the secret cannot be
/// shown again after the response that created it. A lost link is
/// replaced by minting another and revoking the old one.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ShareLinkResponse {
    pub id: String,
    pub name: Option<String>,
    pub data_id: String,
    pub created_by: Option<String>,
    pub created_at: String,
    pub revoked_at: Option<String>,
    /// Whether the link still opens the document.
    pub active: bool,
}

impl From<&ShareLink> for ShareLinkResponse {
    fn from(link: &ShareLink) -> Self {
        Self {
            id: link.id().to_string(),
            name: link.name().as_ref().map(|name| name.to_string()),
            data_id: link.data_id().clone(),
            created_by: link.created_by().clone(),
            created_at: link.created_at().to_rfc3339(),
            revoked_at: link
                .revoked_at()
                .map(|revoked_at| revoked_at.to_rfc3339()),
            active: !link.is_revoked(),
        }
    }
}

/// The one response that carries the secret.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateShareLinkResponse {
    #[serde(flatten)]
    pub link: ShareLinkResponse,
    /// The secret itself. Shown once; it is unrecoverable afterwards.
    pub token: String,
    /// The address to hand out, in the Library client.
    pub url: String,
}

#[derive(Serialize, ToSchema)]
pub struct ShareLinkListResponse {
    pub share_links: Vec<ShareLinkResponse>,
}

/// What a share token resolves to: one document, and the property
/// definitions needed to render it.
///
/// It names neither the organization nor the repository. A recipient
/// holding the token can read this JSON, and those usernames are part of
/// what a private repository keeps private -- the document is what was
/// shared, not the collection it came from. Nothing in the client needs
/// them either: image values carry absolute URLs of their own.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SharedDataResponse {
    pub data: DataResponse,
    pub properties: Vec<PropertyResponse>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SlackUnfurlProjectionRequest {
    pub url: String,
    pub slack_team_id: String,
    pub tachyon_tenant_id: String,
    pub request_id: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "snake_case", tag = "result")]
pub enum SlackUnfurlProjectionResponse {
    Unfurl {
        kind: SlackUnfurlProjectionKind,
        title: String,
        summary: String,
        content_kind: String,
        canonical_url: String,
    },
    NoUnfurl {
        reason: SlackUnfurlNoUnfurlReason,
    },
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SlackUnfurlProjectionKind {
    PrivateShare,
    PublicData,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SlackUnfurlNoUnfurlReason {
    UnsupportedOrNotFound,
}

#[derive(Clone, Debug, Deserialize)]
struct SlackUnfurlClaims {
    sub: String,
    exp: i64,
    jti: String,
    tenant: String,
}

fn target<'a>(
    executor: &'a LibraryExecutor,
    library_org: &'a LibraryOrg,
    org: &'a str,
    repo: &'a str,
) -> ShareLinkRepoTarget<'a> {
    ShareLinkRepoTarget {
        executor,
        multi_tenancy: library_org,
        org_username: org,
        repo_username: repo,
    }
}

/// `POST /v1beta/repos/{org}/{repo}/data/{data_id}/share-links`
#[utoipa::path(
    post,
    path = "/v1beta/repos/{org}/{repo}/data/{data_id}/share-links",
    params(
        ("org" = String, Path, description = "Organization username"),
        ("repo" = String, Path, description = "Repository username"),
        ("data_id" = String, Path, description = "Data ID"),
    ),
    request_body = CreateShareLinkRequest,
    responses(
        (status = 200, description = "The link, with its secret shown once", body = CreateShareLinkResponse),
        (status = 403, description = "Caller may not share this repository"),
        (status = 404, description = "Organization, repository or document not found")
    ),
    tag = "share-links"
)]
#[axum::debug_handler]
pub async fn create_share_link(
    AxumPath((org, repo, data_id)): AxumPath<(String, String, String)>,
    Extension(library_app): Extension<Arc<LibraryApp>>,
    executor: LibraryExecutor,
    library_org: LibraryOrg,
    Json(payload): Json<CreateShareLinkRequest>,
) -> errors::Result<Json<CreateShareLinkResponse>> {
    let created = library_app
        .share_links
        .create(&CreateShareLinkInputData {
            target: target(&executor, &library_org, &org, &repo),
            data_id: &data_id,
            name: payload
                .name
                .as_deref()
                .map(str::trim)
                .filter(|name| !name.is_empty()),
        })
        .await?;

    let token = created.token.as_str().to_string();
    Ok(Json(CreateShareLinkResponse {
        link: ShareLinkResponse::from(&created.link),
        url: share_url(&token),
        token,
    }))
}

/// `GET /v1beta/repos/{org}/{repo}/data/{data_id}/share-links`
#[utoipa::path(
    get,
    path = "/v1beta/repos/{org}/{repo}/data/{data_id}/share-links",
    params(
        ("org" = String, Path, description = "Organization username"),
        ("repo" = String, Path, description = "Repository username"),
        ("data_id" = String, Path, description = "Data ID"),
    ),
    responses(
        (status = 200, description = "Links for this document, newest first", body = ShareLinkListResponse),
        (status = 403, description = "Caller may not share this repository"),
        (status = 404, description = "Organization or repository not found")
    ),
    tag = "share-links"
)]
#[axum::debug_handler]
pub async fn list_share_links(
    AxumPath((org, repo, data_id)): AxumPath<(String, String, String)>,
    Extension(library_app): Extension<Arc<LibraryApp>>,
    executor: LibraryExecutor,
    library_org: LibraryOrg,
) -> errors::Result<Json<ShareLinkListResponse>> {
    let links = library_app
        .share_links
        .list(&ListShareLinksInputData {
            target: target(&executor, &library_org, &org, &repo),
            data_id: &data_id,
        })
        .await?;

    Ok(Json(ShareLinkListResponse {
        share_links: links.iter().map(ShareLinkResponse::from).collect(),
    }))
}

/// `DELETE /v1beta/repos/{org}/{repo}/share-links/{share_link_id}`
///
/// Revokes rather than deletes, so the owner keeps seeing that the link
/// existed and when it stopped working.
#[utoipa::path(
    delete,
    path = "/v1beta/repos/{org}/{repo}/share-links/{share_link_id}",
    params(
        ("org" = String, Path, description = "Organization username"),
        ("repo" = String, Path, description = "Repository username"),
        ("share_link_id" = String, Path, description = "Share link ID (sl_)"),
    ),
    responses(
        (status = 200, description = "The revoked link", body = ShareLinkResponse),
        (status = 403, description = "Caller may not share this repository"),
        (status = 404, description = "No such link in this repository")
    ),
    tag = "share-links"
)]
#[axum::debug_handler]
pub async fn revoke_share_link(
    AxumPath((org, repo, share_link_id)): AxumPath<(
        String,
        String,
        String,
    )>,
    Extension(library_app): Extension<Arc<LibraryApp>>,
    executor: LibraryExecutor,
    library_org: LibraryOrg,
) -> errors::Result<Json<ShareLinkResponse>> {
    let link = library_app
        .share_links
        .revoke(&RevokeShareLinkInputData {
            target: target(&executor, &library_org, &org, &repo),
            share_link_id: &share_link_id,
        })
        .await?;

    Ok(Json(ShareLinkResponse::from(&link)))
}

/// `GET /v1beta/share/{token}`
///
/// Unauthenticated on purpose; see the module note.
#[utoipa::path(
    get,
    path = "/v1beta/share/{token}",
    params(
        ("token" = String, Path, description = "Share token (shr_)"),
    ),
    responses(
        (status = 200, description = "The shared document", body = SharedDataResponse),
        (status = 404, description = "Unknown or revoked token")
    ),
    tag = "share-links"
)]
#[axum::debug_handler]
pub async fn view_shared_data(
    AxumPath(token): AxumPath<String>,
    Extension(library_app): Extension<Arc<LibraryApp>>,
) -> errors::Result<Json<SharedDataResponse>> {
    let SharedData { data, properties } =
        library_app.view_shared_data.execute(&token).await?;

    let items = data
        .property_data()
        .iter()
        .filter_map(|value| {
            // A value whose property has since been deleted has no key
            // to report it under, and the viewer has no way to render
            // it. Dropping it keeps the page up; the alternative here
            // used to be an `unwrap`.
            let property = properties
                .iter()
                .find(|property| property.id() == value.property_id())?;
            Some(PropertyDataResponse {
                property_id: value.property_id().to_string(),
                key: property.name().to_string(),
                value: value.value().clone().map(|value| value.into()),
            })
        })
        .collect();

    Ok(Json(SharedDataResponse {
        data: DataResponse {
            id: data.id().to_string(),
            name: data.name().to_string(),
            record_version: data.record_version().to_string(),
            // The share URL, not the client's record route: a visitor
            // holding this token cannot open the private document, and
            // handing them a URL that 403s would be worse than useless.
            url: share_url(&token),
            items,
        },
        properties: properties
            .iter()
            .map(|property| PropertyResponse {
                id: property.id().to_string(),
                name: property.name().to_string(),
                property_type: property.property_type().to_string(),
                auto_generate: None,
                database_id: match property.property_type() {
                    database_manager::domain::PropertyType::Relation(
                        relation,
                    ) => Some(relation.database_id.to_string()),
                    _ => None,
                },
                // Without these a Select value renders as the raw
                // `op_...` id, which is not the document the owner
                // shared.
                options: property_select_options(property.property_type()),
            })
            .collect(),
    }))
}

/// `POST /internal/slack/unfurl-projection`
///
/// Private service route for Tachyon's Slack integration. Authentication is a
/// short-lived workload JWT; a share token by itself is never enough to call
/// this endpoint.
#[utoipa::path(
    post,
    path = "/internal/slack/unfurl-projection",
    request_body = SlackUnfurlProjectionRequest,
    responses(
        (status = 200, description = "Projection or explicit no-op", body = SlackUnfurlProjectionResponse),
        (status = 401, description = "Missing or invalid workload token")
    ),
    tag = "internal"
)]
#[axum::debug_handler]
pub async fn slack_unfurl_projection(
    headers: HeaderMap,
    Extension(library_app): Extension<Arc<LibraryApp>>,
    Json(payload): Json<SlackUnfurlProjectionRequest>,
) -> errors::Result<Json<SlackUnfurlProjectionResponse>> {
    verify_slack_unfurl_workload(&headers, &payload.tachyon_tenant_id)?;

    let response = match classify_slack_unfurl_url(&payload.url) {
        SlackUnfurlUrl::PrivateShare { token } => {
            private_share_projection(&library_app, &token, &payload.url)
                .await
        }
        SlackUnfurlUrl::PublicData { org, repo, data_id } => {
            public_data_projection(
                &library_app,
                &org,
                &repo,
                &data_id,
                &payload.url,
            )
            .await
        }
        SlackUnfurlUrl::Unsupported => {
            Ok(SlackUnfurlProjectionResponse::NoUnfurl {
                reason: SlackUnfurlNoUnfurlReason::UnsupportedOrNotFound,
            })
        }
    }?;

    Ok(Json(response))
}

enum SlackUnfurlUrl {
    PrivateShare {
        token: String,
    },
    PublicData {
        org: String,
        repo: String,
        data_id: String,
    },
    Unsupported,
}

fn verify_slack_unfurl_workload(
    headers: &HeaderMap,
    expected_tenant: &str,
) -> errors::Result<()> {
    let token = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .filter(|token| !token.trim().is_empty())
        .ok_or_else(slack_unfurl_unauthorized)?;
    let secret = std::env::var(SLACK_UNFURL_AUTH_SECRET_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(slack_unfurl_unauthorized)?;
    let issuer = std::env::var(SLACK_UNFURL_ISSUER_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_SLACK_UNFURL_ISSUER.to_string());
    let audience = std::env::var(SLACK_UNFURL_AUDIENCE_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_SLACK_UNFURL_AUDIENCE.to_string());

    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_issuer(&[&issuer]);
    validation.set_audience(&[&audience]);
    validation.set_required_spec_claims(&["exp", "iss", "aud", "sub"]);
    validation.leeway = 0;
    let claims = decode::<SlackUnfurlClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map_err(|_| slack_unfurl_unauthorized())?
    .claims;

    if claims.sub.trim().is_empty()
        || claims.jti.trim().is_empty()
        || claims.tenant != expected_tenant
    {
        return Err(slack_unfurl_unauthorized());
    }

    let now = chrono::Utc::now().timestamp();
    if claims.exp <= now || claims.exp - now > 300 {
        return Err(slack_unfurl_unauthorized());
    }
    if !SLACK_UNFURL_JTI_CACHE.insert_if_absent(
        claims.jti,
        (),
        Duration::from_secs((claims.exp - now) as u64),
    ) {
        return Err(slack_unfurl_unauthorized());
    }

    claims
        .tenant
        .parse::<value_object::TenantId>()
        .map_err(|_| slack_unfurl_unauthorized())?;
    Ok(())
}

fn slack_unfurl_unauthorized() -> errors::Error {
    errors::Error::unauthorized("invalid Slack unfurl workload token")
}

fn classify_slack_unfurl_url(value: &str) -> SlackUnfurlUrl {
    let configured =
        crate::usecase::library_client_url::library_client_base_url();
    classify_slack_unfurl_url_with_base(value, &configured)
}

fn classify_slack_unfurl_url_with_base(
    value: &str,
    configured_base: &str,
) -> SlackUnfurlUrl {
    let Ok(url) = url::Url::parse(value) else {
        return SlackUnfurlUrl::Unsupported;
    };
    if url.scheme() != "https"
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return SlackUnfurlUrl::Unsupported;
    }
    let Ok(base) = url::Url::parse(configured_base) else {
        return SlackUnfurlUrl::Unsupported;
    };
    if url.host_str() != base.host_str()
        || url.port_or_known_default() != base.port_or_known_default()
    {
        return SlackUnfurlUrl::Unsupported;
    }

    let segments = url
        .path_segments()
        .map(|segments| segments.collect::<Vec<_>>())
        .unwrap_or_default();
    match segments.as_slice() {
        ["s", token] if !token.is_empty() => SlackUnfurlUrl::PrivateShare {
            token: (*token).to_string(),
        },
        ["public", org, repo, data_id]
            if !org.is_empty()
                && !repo.is_empty()
                && !data_id.is_empty() =>
        {
            SlackUnfurlUrl::PublicData {
                org: (*org).to_string(),
                repo: (*repo).to_string(),
                data_id: (*data_id).to_string(),
            }
        }
        _ => SlackUnfurlUrl::Unsupported,
    }
}

async fn private_share_projection(
    library_app: &LibraryApp,
    token: &str,
    canonical_url: &str,
) -> errors::Result<SlackUnfurlProjectionResponse> {
    let shared = match library_app.view_shared_data.execute(token).await {
        Ok(shared) => shared,
        Err(error) if error.is_not_found() => {
            return Ok(SlackUnfurlProjectionResponse::NoUnfurl {
                reason: SlackUnfurlNoUnfurlReason::UnsupportedOrNotFound,
            });
        }
        Err(error) => return Err(error),
    };

    Ok(SlackUnfurlProjectionResponse::Unfurl {
        kind: SlackUnfurlProjectionKind::PrivateShare,
        title: projection_title(&shared.data),
        summary: projection_summary(&shared.data),
        content_kind: "data".to_string(),
        canonical_url: canonical_url.to_string(),
    })
}

async fn public_data_projection(
    library_app: &LibraryApp,
    org: &str,
    repo: &str,
    data_id: &str,
    canonical_url: &str,
) -> errors::Result<SlackUnfurlProjectionResponse> {
    let executor = LibraryExecutor {
        inner: LibraryExecutorKind::None,
        original_token: None,
    };
    let library_org = LibraryOrg::with_org(org.to_string());
    let (data, _properties) = match library_app
        .view_data
        .execute(&ViewDataInputData {
            executor: &executor,
            multi_tenancy: &library_org,
            org_username: org.to_string(),
            repo_username: repo.to_string(),
            data_id: data_id.to_string(),
        })
        .await
    {
        Ok(output) => output,
        Err(error) if error.is_not_found() || error.is_forbidden() => {
            return Ok(SlackUnfurlProjectionResponse::NoUnfurl {
                reason: SlackUnfurlNoUnfurlReason::UnsupportedOrNotFound,
            });
        }
        Err(error) => return Err(error),
    };

    Ok(SlackUnfurlProjectionResponse::Unfurl {
        kind: SlackUnfurlProjectionKind::PublicData,
        title: projection_title(&data),
        summary: projection_summary(&data),
        content_kind: "data".to_string(),
        canonical_url: canonical_url.to_string(),
    })
}

fn projection_title(data: &database_manager::domain::Data) -> String {
    let title = data.name().to_string();
    if title.trim().is_empty() {
        "Untitled".to_string()
    } else {
        title
    }
}

fn projection_summary(data: &database_manager::domain::Data) -> String {
    let text = data
        .property_data()
        .iter()
        .filter_map(|value| value.value().as_ref())
        .map(|value| value.string_value())
        .map(|value| sanitize_projection_text(&value))
        .find(|value| !value.is_empty())
        .unwrap_or_default();
    if text.chars().count() <= 160 {
        return text;
    }
    let mut summary = text.chars().take(159).collect::<String>();
    summary.push('…');
    summary
}

fn sanitize_projection_text(value: &str) -> String {
    value
        .replace(|ch: char| ch.is_control(), " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE_URL: &str = "https://library.example";

    #[test]
    fn slack_unfurl_classifies_private_share_urls() {
        match classify_slack_unfurl_url_with_base(
            "https://library.example/s/shr_deadbeef",
            BASE_URL,
        ) {
            SlackUnfurlUrl::PrivateShare { token } => {
                assert_eq!(token, "shr_deadbeef");
            }
            _ => panic!("expected a private share URL"),
        }
    }

    #[test]
    fn slack_unfurl_classifies_public_data_urls() {
        match classify_slack_unfurl_url_with_base(
            "https://library.example/public/quantumbox/artifacts/data_123",
            BASE_URL,
        ) {
            SlackUnfurlUrl::PublicData { org, repo, data_id } => {
                assert_eq!(org, "quantumbox");
                assert_eq!(repo, "artifacts");
                assert_eq!(data_id, "data_123");
            }
            _ => panic!("expected a public data URL"),
        }
    }

    #[test]
    fn slack_unfurl_rejects_query_fragments_and_foreign_hosts() {
        assert!(matches!(
            classify_slack_unfurl_url_with_base(
                "https://library.example/s/shr_deadbeef?x=1",
                BASE_URL,
            ),
            SlackUnfurlUrl::Unsupported
        ));
        assert!(matches!(
            classify_slack_unfurl_url_with_base(
                "https://evil.example/s/shr_deadbeef",
                BASE_URL,
            ),
            SlackUnfurlUrl::Unsupported
        ));
        assert!(matches!(
            classify_slack_unfurl_url_with_base(
                "http://library.example/s/shr_deadbeef",
                BASE_URL,
            ),
            SlackUnfurlUrl::Unsupported
        ));
    }

    #[test]
    fn sanitize_projection_text_strips_controls() {
        let raw = format!("hello\n\tworld {}", "あ".repeat(200));
        let sanitized = sanitize_projection_text(&raw);

        assert!(sanitized.starts_with("hello world"));
        assert!(!sanitized.contains('\n'));
        assert!(!sanitized.contains('\t'));
    }
}
