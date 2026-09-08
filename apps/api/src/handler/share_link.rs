//! Read-only share links for one document in a private repo.
//!
//! Three of these routes manage links and are authenticated like any
//! other repo write. The fourth, `GET /v1beta/share/{token}`, is
//! deliberately unauthenticated: the unguessable token in the URL is the
//! credential, the same arrangement the image route uses, and it answers
//! with exactly one document.

use std::sync::Arc;

use axum::{
    extract::{Extension, Path as AxumPath},
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::app::LibraryApp;
use crate::domain::ShareLink;
use crate::handler::library_executor_extractor::LibraryExecutor;
use crate::handler::types::{
    property_select_options, DataResponse, PropertyDataResponse,
    PropertyResponse,
};
use crate::usecase::{
    library_client_url::share_url, CreateShareLinkInputData, LibraryOrg,
    ListShareLinksInputData, RevokeShareLinkInputData, ShareLinkRepoTarget,
    SharedData,
};

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
                // Without these a Select value renders as the raw
                // `op_...` id, which is not the document the owner
                // shared.
                options: property_select_options(property.property_type()),
            })
            .collect(),
    }))
}
