//! Images embedded in record bodies.
//!
//! A rich text editor needs somewhere to put the picture that was just
//! dropped into it. Uploading writes the bytes to object storage under an
//! unguessable name and hands back a URL that is stable enough to live inside
//! a document: reading redirects to a freshly signed URL every time, so the
//! link in the body never expires the way a presigned URL would.
//!
//! In production the object store is Tachyon storage (`/v1/storage/*`).
//! Objects live in each organization's own tenant storage, keyed
//! `{org_tenant}/library/images/{id}`. The service account's credential
//! only verifies in the Library platform tenant, so requests act in that
//! scope and name the organization through the storage API's `tenant_id`
//! delegation field: Tachyon verifies the organization is a descendant of
//! the platform, authorizes the action against
//! `trn:storage:tenant:{org_tenant}`, and bills the platform — the only
//! tenant here with a billing account. That delegation ships from
//! tachyon-apps branch `cfeature/storage-platform-delegation` and must be
//! deployed before this handler; without it Tachyon rejects the key as
//! outside the acting tenant's scope.
//!
//! Reading is deliberately unauthenticated. The browser fetches an `<img>`
//! without the caller's bearer token, so the only credential an image URL can
//! carry is the URL itself — the 128 random bits in its name. Treat an image
//! URL as the capability it is: whoever holds it can read that one image.

use std::sync::Arc;

use axum::{
    extract::{Extension, Path as AxumPath, Query},
    http::{
        header::{CONTENT_TYPE, HOST},
        HeaderMap,
    },
    response::Redirect,
    Json,
};
use bytes::Bytes;
use hex::encode as hex_encode;
use persistence::Storage;
use serde::{Deserialize, Serialize};
use tachyon_sdk::auth::{CheckPolicyInput, MultiTenancyAction};
use url::Url;
use utoipa::{IntoParams, ToSchema};
use value_object::InMemoryFile;

use crate::app::LibraryApp;
use crate::handler::library_executor_extractor::LibraryExecutor;
use crate::usecase::{LibraryOrg, ViewRepoInputData};

/// Pictures pasted into a body are small. A larger upload is a sign the
/// caller meant to attach a file rather than embed one.
pub const MAX_IMAGE_BYTES: usize = 10 * 1024 * 1024;

/// How long a redirect target stays valid. Long enough for the browser to
/// follow it, short enough that a leaked storage URL stops working.
const IMAGE_URL_EXPIRES_SECS: u32 = 900;

/// How long the upload leg's presigned PUT stays valid. The API uploads the
/// bytes itself in the same request, so this only needs to survive one call.
const IMAGE_PUT_EXPIRES_SECS: u32 = 300;

/// Formats every browser renders in an `<img>`, minus SVG: SVG is a document
/// that can carry script, and these bytes are served straight from storage.
const ALLOWED_IMAGE_TYPES: &[(&str, &str)] = &[
    ("image/png", "png"),
    ("image/jpeg", "jpg"),
    ("image/gif", "gif"),
    ("image/webp", "webp"),
    ("image/avif", "avif"),
];

/// Where record body images live. One address — the organization's tenant id
/// plus the image's minted name — resolves the same object on either backing
/// store, so the handlers never know which one is behind them.
#[async_trait::async_trait]
pub trait ImageObjectStore: Send + Sync {
    async fn put(
        &self,
        org_tenant: &str,
        image_id: &str,
        content_type: &str,
        bytes: Bytes,
    ) -> errors::Result<()>;

    async fn presigned_get(
        &self,
        org_tenant: &str,
        image_id: &str,
    ) -> errors::Result<Url>;
}

/// Tachyon storage, spoken as the Library service account.
///
/// The flow per object is presign → PUT → confirm; reads are one `get-url`
/// call. Every request carries `x-operator-id: {platform_tenant}` — the
/// only tenant the credential verifies in — plus a `tenant_id` body field
/// naming the organization, which Tachyon's delegation turns into the
/// tenant whose storage the key must live in (see module docs).
pub struct TachyonImageStore {
    base_url: String,
    auth_token: String,
    /// The Library platform tenant the service account acts as.
    platform_tenant: String,
    http: reqwest::Client,
}

#[derive(Debug, Deserialize)]
struct StoragePresignResponse {
    url: String,
}

impl TachyonImageStore {
    pub fn new(
        base_url: String,
        auth_token: String,
        platform_tenant: String,
    ) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            auth_token,
            platform_tenant,
            http: reqwest::Client::new(),
        }
    }

    fn object_key(org_tenant: &str, image_id: &str) -> String {
        format!("{org_tenant}/library/images/{image_id}")
    }

    async fn storage_call<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: serde_json::Value,
    ) -> errors::Result<T> {
        let response = self
            .http
            .post(format!("{}{path}", self.base_url))
            .header("authorization", format!("Bearer {}", self.auth_token))
            .header("x-operator-id", &self.platform_tenant)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                errors::Error::http_request_error(format!(
                    "tachyon storage {path}: {e}"
                ))
            })?;

        let status = response.status();
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(errors::Error::not_found("image"));
        }
        if !status.is_success() {
            // 403 here means the service account is missing a storage
            // grant; 402 means the platform tenant has no billing account.
            // Neither is the caller's fault, so both surface as a plain
            // upstream failure with the status for the log line.
            return Err(errors::Error::http_request_error(format!(
                "tachyon storage {path} returned {status}"
            )));
        }
        response.json::<T>().await.map_err(|e| {
            errors::Error::http_request_error(format!(
                "tachyon storage {path}: {e}"
            ))
        })
    }
}

#[async_trait::async_trait]
impl ImageObjectStore for TachyonImageStore {
    async fn put(
        &self,
        org_tenant: &str,
        image_id: &str,
        content_type: &str,
        bytes: Bytes,
    ) -> errors::Result<()> {
        let key = Self::object_key(org_tenant, image_id);
        let presign: StoragePresignResponse = self
            .storage_call(
                "/v1/storage/presigned-url",
                serde_json::json!({
                    "key": key,
                    "method": "PUT",
                    "content_type": content_type,
                    "expires_in_secs": IMAGE_PUT_EXPIRES_SECS,
                    "tenant_id": org_tenant,
                }),
            )
            .await?;

        let response = self
            .http
            .put(&presign.url)
            .header(reqwest::header::CONTENT_TYPE, content_type)
            .body(bytes)
            .send()
            .await
            .map_err(|e| {
                errors::Error::http_request_error(format!(
                    "image upload PUT: {e}"
                ))
            })?;
        if !response.status().is_success() {
            return Err(errors::Error::http_request_error(format!(
                "image upload PUT returned {}",
                response.status()
            )));
        }

        // Confirm verifies the object landed; without it a failed PUT
        // would still hand the caller a URL that 404s forever.
        let _: serde_json::Value = self
            .storage_call(
                "/v1/storage/confirm",
                serde_json::json!({
                    "storage_key": key,
                    "tenant_id": org_tenant,
                }),
            )
            .await?;
        Ok(())
    }

    async fn presigned_get(
        &self,
        org_tenant: &str,
        image_id: &str,
    ) -> errors::Result<Url> {
        let key = Self::object_key(org_tenant, image_id);
        let presign: StoragePresignResponse = self
            .storage_call(
                "/v1/storage/get-url",
                serde_json::json!({
                    "storage_key": key,
                    "expires_in_secs": IMAGE_URL_EXPIRES_SECS,
                    "tenant_id": org_tenant,
                }),
            )
            .await?;
        Url::parse(&presign.url).map_err(|e| {
            errors::Error::internal_server_error(format!(
                "tachyon storage returned an unparsable URL: {e}"
            ))
        })
    }
}

/// A plain bucket (MinIO in development), for environments without a
/// Tachyon account. Keys use the same per-tenant layout as Tachyon
/// storage, with the bucket standing in for the storage service.
pub struct BucketImageStore {
    storage: Arc<dyn Storage>,
    /// Signing goes through the endpoint clients can reach, which in local
    /// development is not the endpoint the API itself talks to.
    presign_storage: Arc<dyn Storage>,
    bucket: String,
}

impl BucketImageStore {
    pub fn new(
        storage: Arc<dyn Storage>,
        presign_storage: Arc<dyn Storage>,
        bucket: String,
    ) -> Self {
        Self {
            storage,
            presign_storage,
            bucket,
        }
    }

    fn object_path(org_tenant: &str, image_id: &str) -> String {
        format!("{org_tenant}/library/images/{image_id}")
    }
}

#[async_trait::async_trait]
impl ImageObjectStore for BucketImageStore {
    async fn put(
        &self,
        org_tenant: &str,
        image_id: &str,
        content_type: &str,
        bytes: Bytes,
    ) -> errors::Result<()> {
        let file = InMemoryFile::new(
            image_id.to_string(),
            Some(content_type.to_string()),
            bytes,
        )?;
        self.storage
            .put_object(
                &self.bucket,
                &Self::object_path(org_tenant, image_id),
                &file,
            )
            .await?;
        Ok(())
    }

    async fn presigned_get(
        &self,
        org_tenant: &str,
        image_id: &str,
    ) -> errors::Result<Url> {
        self.presign_storage
            .presigned_get(
                &self.bucket,
                &Self::object_path(org_tenant, image_id),
                IMAGE_URL_EXPIRES_SECS,
            )
            .await
    }
}

#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct UploadImageQuery {
    /// Original file name, used only to keep a recognisable extension.
    pub filename: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ImageResponse {
    /// Storage name of the image, unique within the repository.
    pub id: String,
    /// Absolute URL to embed in a document.
    pub url: String,
}

#[utoipa::path(
    post,
    path = "/v1beta/repos/{org}/{repo}/images",
    params(
        ("org" = String, Path, description = "Organization username"),
        ("repo" = String, Path, description = "Repository username"),
        UploadImageQuery,
    ),
    request_body(
        content = Vec<u8>,
        description = "Raw image bytes; the Content-Type header names the format",
        content_type = "image/png",
    ),
    responses(
        (status = 200, description = "Image stored", body = ImageResponse),
        (status = 400, description = "Empty body, oversized body, or unsupported image format"),
        (status = 403, description = "Caller may not write to the repository"),
        (status = 404, description = "Repository not found")
    )
)]
// One parameter per extractor is how axum handlers are written; the count
// says nothing about the work this one does.
#[allow(clippy::too_many_arguments)]
pub async fn upload_image(
    AxumPath((org, repo)): AxumPath<(String, String)>,
    Query(query): Query<UploadImageQuery>,
    Extension(library_app): Extension<Arc<LibraryApp>>,
    Extension(image_store): Extension<Arc<dyn ImageObjectStore>>,
    executor: LibraryExecutor,
    library_org: LibraryOrg,
    headers: HeaderMap,
    body: Bytes,
) -> errors::Result<Json<ImageResponse>> {
    if body.is_empty() {
        return Err(errors::Error::bad_request("image body is empty"));
    }
    if body.len() > MAX_IMAGE_BYTES {
        return Err(errors::Error::bad_request(format!(
            "image is larger than {MAX_IMAGE_BYTES} bytes"
        )));
    }

    let declared_type = headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    let (content_type, extension) =
        resolve_image_type(declared_type, query.filename.as_deref())?;

    // Existence and read access first, so a caller who cannot see the
    // repository learns nothing beyond "not found".
    let repo_output = library_app
        .view_repo
        .execute(&ViewRepoInputData {
            executor: &executor,
            multi_tenancy: &library_org,
            organization_username: org.clone(),
            repo_username: repo.clone(),
        })
        .await?;
    library_app
        .auth_app
        .check_policy(&CheckPolicyInput {
            executor: &executor,
            multi_tenancy: &library_org,
            action: "library:UpdateRepo",
        })
        .await?;

    // The organization's tenant id, not its username, addresses the object:
    // usernames can be renamed, tenant ids cannot.
    let org_tenant = repo_output.repo.organization_id().to_string();

    let id =
        format!("{}.{extension}", hex_encode(rand::random::<[u8; 16]>()));
    image_store
        .put(&org_tenant, &id, content_type, body)
        .await?;

    let url = format!(
        "{}/v1beta/repos/{org}/{repo}/images/{id}",
        api_base_url(&headers, std::env::var("LIBRARY_API_BASE_URL").ok())
    );
    Ok(Json(ImageResponse { id, url }))
}

#[utoipa::path(
    get,
    path = "/v1beta/repos/{org}/{repo}/images/{image_id}",
    params(
        ("org" = String, Path, description = "Organization username"),
        ("repo" = String, Path, description = "Repository username"),
        ("image_id" = String, Path, description = "Image id returned by the upload")
    ),
    responses(
        (status = 307, description = "Redirect to the stored image"),
        (status = 400, description = "Malformed image id"),
        (status = 404, description = "Organization or image not found")
    )
)]
pub async fn view_image(
    AxumPath((_org, _repo, image_id)): AxumPath<(String, String, String)>,
    Extension(image_store): Extension<Arc<dyn ImageObjectStore>>,
    library_org: LibraryOrg,
) -> errors::Result<Redirect> {
    if !is_image_id(&image_id) {
        return Err(errors::Error::bad_request("malformed image id"));
    }

    // The extractor resolved the organization username in the path to its
    // tenant id; a URL naming an organization that does not exist stops
    // here rather than probing storage.
    let org_tenant = library_org
        .operator_id()
        .ok_or_else(|| errors::Error::not_found("organization"))?
        .to_string();

    let url = image_store.presigned_get(&org_tenant, &image_id).await?;
    Ok(Redirect::temporary(url.as_str()))
}

/// The origin this API is reached at, so the URL stored in a document works
/// from every client. The proxy headers are what a load balancer rewrites;
/// the environment variable wins because only it knows the public name when
/// the API sits behind something that does not forward them.
fn api_base_url(headers: &HeaderMap, configured: Option<String>) -> String {
    if let Some(configured) = configured {
        let trimmed = configured.trim_end_matches('/');
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    let host = headers
        .get("x-forwarded-host")
        .or_else(|| headers.get(HOST))
        .and_then(|value| value.to_str().ok())
        .unwrap_or("localhost:50055");
    let scheme = headers
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_else(|| {
            if host.starts_with("localhost")
                || host.starts_with("127.0.0.1")
            {
                "http"
            } else {
                "https"
            }
        });
    format!("{scheme}://{host}")
}

/// Picks the stored content type and file extension, preferring what the
/// caller declared and falling back to the file name when a client sends
/// `application/octet-stream` for a picture it clearly named `.png`.
fn resolve_image_type(
    declared_type: &str,
    filename: Option<&str>,
) -> errors::Result<(&'static str, &'static str)> {
    let declared = declared_type
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    if let Some(found) = ALLOWED_IMAGE_TYPES
        .iter()
        .find(|(content_type, _)| *content_type == declared)
    {
        return Ok(*found);
    }

    let extension = filename
        .and_then(|name| name.rsplit_once('.'))
        .map(|(_, extension)| extension.to_ascii_lowercase())
        .unwrap_or_default();
    let extension = if extension == "jpeg" {
        "jpg".to_string()
    } else {
        extension
    };
    if let Some(found) = ALLOWED_IMAGE_TYPES
        .iter()
        .find(|(_, known)| *known == extension)
    {
        return Ok(*found);
    }

    Err(errors::Error::bad_request(format!(
        "unsupported image type: {}",
        if declared.is_empty() {
            "unknown"
        } else {
            &declared
        }
    )))
}

/// Whether an id is one this handler could have minted. Guards the object
/// path against `..` and anything else a caller might try to address.
fn is_image_id(image_id: &str) -> bool {
    let Some((name, extension)) = image_id.rsplit_once('.') else {
        return false;
    };
    name.len() == 32
        && name.chars().all(|c| c.is_ascii_hexdigit())
        && ALLOWED_IMAGE_TYPES
            .iter()
            .any(|(_, known)| *known == extension)
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    #[test]
    fn declared_content_type_decides_the_extension() {
        assert_eq!(
            resolve_image_type("image/png", None).unwrap(),
            ("image/png", "png")
        );
        assert_eq!(
            resolve_image_type("image/jpeg; charset=binary", None).unwrap(),
            ("image/jpeg", "jpg")
        );
    }

    #[test]
    fn file_name_stands_in_for_an_unhelpful_content_type() {
        assert_eq!(
            resolve_image_type(
                "application/octet-stream",
                Some("holiday.JPEG")
            )
            .unwrap(),
            ("image/jpeg", "jpg")
        );
    }

    #[test]
    fn formats_that_are_not_pictures_are_refused() {
        assert!(
            resolve_image_type("image/svg+xml", Some("logo.svg")).is_err()
        );
        assert!(resolve_image_type("text/html", Some("page.html")).is_err());
        assert!(resolve_image_type("", None).is_err());
    }

    #[test]
    fn only_minted_ids_address_an_object() {
        let id = format!("{}.png", hex_encode(rand::random::<[u8; 16]>()));
        assert!(is_image_id(&id));

        assert!(!is_image_id("../../secret.png"));
        assert!(!is_image_id("short.png"));
        assert!(!is_image_id(&format!(
            "{}.svg",
            hex_encode(rand::random::<[u8; 16]>())
        )));
        assert!(!is_image_id("0123456789abcdef0123456789abcdef"));
    }

    #[test]
    fn base_url_follows_the_proxy_headers() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-proto", "https".parse().unwrap());
        headers
            .insert("x-forwarded-host", "api.example.com".parse().unwrap());
        assert_eq!(api_base_url(&headers, None), "https://api.example.com");
        assert_eq!(
            api_base_url(&headers, Some("https://library.example/".into())),
            "https://library.example"
        );

        let mut local = HeaderMap::new();
        local.insert(HOST, "localhost:50055".parse().unwrap());
        assert_eq!(api_base_url(&local, None), "http://localhost:50055");
    }

    /// What one storage-facing request looked like, captured by the stub.
    #[derive(Debug, Default)]
    struct SeenStorageCalls {
        presign: Option<(String, String, serde_json::Value)>,
        put: Option<(String, Vec<u8>)>,
        confirm: Option<serde_json::Value>,
        get_url: Option<(String, String, serde_json::Value)>,
    }

    fn auth_headers(headers: &HeaderMap) -> (String, String) {
        let header = |name: &str| {
            headers
                .get(name)
                .and_then(|value| value.to_str().ok())
                .unwrap_or_default()
                .to_string()
        };
        (header("authorization"), header("x-operator-id"))
    }

    /// A stub tachyon-api serving the three storage calls the store makes,
    /// recording what reached it.
    async fn stub_tachyon(
        seen: Arc<Mutex<SeenStorageCalls>>,
    ) -> std::net::SocketAddr {
        let listener =
            tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let presign_seen = seen.clone();
        let put_seen = seen.clone();
        let confirm_seen = seen.clone();
        let get_seen = seen;
        let app = axum::Router::new()
            .route(
                "/v1/storage/presigned-url",
                axum::routing::post(
                    move |headers: HeaderMap,
                          Json(body): Json<serde_json::Value>| {
                        let (bearer, operator) = auth_headers(&headers);
                        presign_seen.lock().unwrap().presign =
                            Some((bearer, operator, body));
                        async move {
                            Json(serde_json::json!({
                                "url": format!("http://{addr}/r2/put-target"),
                                "expires_in_secs": 300,
                            }))
                        }
                    },
                ),
            )
            .route(
                "/r2/put-target",
                axum::routing::put(
                    move |headers: HeaderMap, body: Bytes| {
                        let content_type = headers
                            .get(CONTENT_TYPE)
                            .and_then(|value| value.to_str().ok())
                            .unwrap_or_default()
                            .to_string();
                        put_seen.lock().unwrap().put =
                            Some((content_type, body.to_vec()));
                        async move { "ok" }
                    },
                ),
            )
            .route(
                "/v1/storage/confirm",
                axum::routing::post(
                    move |Json(body): Json<serde_json::Value>| {
                        confirm_seen.lock().unwrap().confirm =
                            Some(body.clone());
                        async move {
                            Json(serde_json::json!({
                                "storage_key": body["storage_key"],
                                "url": "http://unused.example/",
                                "content_type": "image/png",
                                "content_length": 9,
                            }))
                        }
                    },
                ),
            )
            .route(
                "/v1/storage/get-url",
                axum::routing::post(
                    move |headers: HeaderMap,
                          Json(body): Json<serde_json::Value>| {
                        let (bearer, operator) = auth_headers(&headers);
                        get_seen.lock().unwrap().get_url =
                            Some((bearer, operator, body));
                        async move {
                            Json(serde_json::json!({
                                "url": "http://signed.example/obj?sig=abc",
                                "expires_in_secs": 900,
                            }))
                        }
                    },
                ),
            );

        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        addr
    }

    #[tokio::test]
    async fn tachyon_store_delegates_org_keys_under_platform_credentials() {
        let seen = Arc::new(Mutex::new(SeenStorageCalls::default()));
        let addr = stub_tachyon(seen.clone()).await;

        let store = TachyonImageStore::new(
            format!("http://{addr}"),
            "svc-token".to_string(),
            "tn_01platform".to_string(),
        );

        store
            .put(
                "tn_01orgtenant",
                "0123456789abcdef0123456789abcdef.png",
                "image/png",
                Bytes::from_static(b"png-bytes"),
            )
            .await
            .unwrap();
        let url = store
            .presigned_get(
                "tn_01orgtenant",
                "0123456789abcdef0123456789abcdef.png",
            )
            .await
            .unwrap();

        let expected_key = "tn_01orgtenant/library/images/0123456789abcdef0123456789abcdef.png";
        let seen = seen.lock().unwrap();

        let (bearer, operator, presign_body) =
            seen.presign.as_ref().unwrap();
        assert_eq!(bearer, "Bearer svc-token");
        assert_eq!(operator, "tn_01platform");
        assert_eq!(presign_body["key"], expected_key);
        assert_eq!(presign_body["method"], "PUT");
        assert_eq!(presign_body["content_type"], "image/png");
        assert_eq!(presign_body["tenant_id"], "tn_01orgtenant");

        let (put_content_type, put_body) = seen.put.as_ref().unwrap();
        assert_eq!(put_content_type, "image/png");
        assert_eq!(put_body, b"png-bytes");

        let confirm_body = seen.confirm.as_ref().unwrap();
        assert_eq!(confirm_body["storage_key"], expected_key);
        assert_eq!(confirm_body["tenant_id"], "tn_01orgtenant");

        let (get_bearer, get_operator, get_body) =
            seen.get_url.as_ref().unwrap();
        assert_eq!(get_bearer, "Bearer svc-token");
        assert_eq!(get_operator, "tn_01platform");
        assert_eq!(get_body["storage_key"], expected_key);
        assert_eq!(get_body["tenant_id"], "tn_01orgtenant");

        assert_eq!(url.as_str(), "http://signed.example/obj?sig=abc");
    }
}
