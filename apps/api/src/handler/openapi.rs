use std::fs;
use std::path::Path;

use axum::extract::DefaultBodyLimit;
use utoipa::OpenApi;
use utoipa_axum::{
    router::{OpenApiRouter, UtoipaMethodRouterExt},
    routes,
};
use utoipa_rapidoc::RapiDoc;
use utoipa_redoc::{Redoc, Servable};
use utoipa_swagger_ui::SwaggerUi;

use crate::handler::{
    auth::*, data::*, docs::*, global_id_mapping::*, image::*, live::*,
    organization::*, property::*, repository::*, source::*, translation::*,
};

// TODO: add English comment
#[derive(OpenApi)]
#[openapi(
    paths(
        health_check,
        sign_in,
        view_organization,
        create_organization,
        update_organization,
        view_repo,
        create_repo,
        update_repo,
        delete_repo,
        change_repo_username,
        search_repo,
        view_data,
        view_data_markdown,
        view_data_list,
        list_docs,
        view_doc,
        view_doc_markdown,
        list_doc_languages,
        get_published_languages,
        set_published_languages,
        run_translations,
        get_glossary,
        set_glossary,
        view_data_parquet,
        add_data,
        update_data,
        upsert_data,
        delete_data,
        search_data,
        get_properties,
        get_property,
        add_property,
        update_property,
        delete_property,
        get_source,
        find_sources,
        create_source,
        update_source,
        delete_source,
        get_global_id_mapping,
        upload_image,
        view_image,
        authorize_live,
        checkpoint_live,
    ),
    components(schemas(
        crate::handler::auth::SignInRequest,
        crate::handler::auth::SignInResponse,
        crate::handler::auth::UserResponse,
        crate::handler::types::OrganizationResponse,
        crate::handler::types::CreateOrganizationRequest,
        crate::handler::types::UpdateOrganizationRequest,
        crate::handler::types::RepoResponse,
        crate::handler::types::CreateRepoRequest,
        crate::handler::types::UpdateRepoRequest,
        crate::handler::types::ChangeRepoUsernameRequest,
        crate::handler::types::SearchRepoQuery,
        crate::handler::types::DataListResponse,
        crate::handler::types::DataResponse,
        crate::handler::types::ParquetResponse,
        crate::handler::types::AddDataRequest,
        crate::handler::types::UpdateDataRequest,
        crate::handler::types::UpsertDataRequest,
        crate::handler::types::SearchDataQuery,
        crate::handler::types::DataPaginationQuery,
        value_object::OffsetPaginator,
        crate::handler::types::PropertyResponse,
        crate::handler::types::AddPropertyRequest,
        crate::handler::types::UpdatePropertyRequest,
        crate::handler::translation::PublishedLanguagesResponse,
        crate::handler::translation::SetPublishedLanguagesRequest,
        crate::handler::translation::RunTranslationsResponse,
        crate::handler::translation::TranslationOutcomeResponse,
        crate::handler::translation::GlossaryResponse,
        crate::handler::translation::SetGlossaryRequest,
        crate::handler::translation::GlossaryTermPayload,
        crate::handler::types::SourceResponse,
        crate::handler::types::CreateSourceRequest,
        crate::handler::types::UpdateSourceRequest,
        crate::handler::global_id_mapping::GlobalIdMappingResponse,
        crate::handler::image::ImageResponse,
        crate::handler::live::LiveAuthorizeRequest,
        crate::handler::live::LiveAuthorizationResponse,
        crate::handler::live::LiveBodyFormat,
        crate::handler::live::LiveCheckpointRequest,
        crate::handler::live::LiveCheckpointResponse,
    ))
)]
pub struct ApiDoc;

// TODO: add English comment
#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "OK")
    )
)]
pub async fn health_check() -> &'static str {
    "OK"
}

pub fn create_openapi_router() -> OpenApiRouter<()> {
    OpenApiRouter::new()
        .routes(routes!(health_check))
        .routes(routes!(sign_in))
        .routes(routes!(add_property))
        .routes(routes!(update_property))
        .routes(routes!(view_organization))
        .routes(routes!(create_organization))
        .routes(routes!(update_organization))
        .routes(routes!(view_repo))
        .routes(routes!(create_repo))
        .routes(routes!(update_repo))
        .routes(routes!(delete_repo))
        .routes(routes!(change_repo_username))
        .routes(routes!(search_repo))
        .routes(routes!(view_data))
        .routes(routes!(view_data_markdown))
        .routes(routes!(view_data_list))
        .routes(routes!(view_data_parquet))
        .routes(routes!(add_data))
        .routes(routes!(update_data))
        .routes(routes!(upsert_data))
        .routes(routes!(delete_data))
        .routes(routes!(search_data))
        .routes(routes!(get_properties))
        .routes(routes!(get_property))
        .routes(routes!(delete_property))
        .routes(routes!(get_source))
        .routes(routes!(find_sources))
        .routes(routes!(create_source))
        .routes(routes!(update_source))
        .routes(routes!(delete_source))
        .routes(routes!(get_global_id_mapping))
        .routes(routes!(get_published_languages))
        .routes(routes!(set_published_languages))
        .routes(routes!(run_translations))
        .routes(routes!(get_glossary))
        .routes(routes!(set_glossary))
        .routes(routes!(authorize_live))
        .routes(
            routes!(checkpoint_live).layer(checkpoint_live_body_limit()),
        )
}

fn checkpoint_live_body_limit() -> DefaultBodyLimit {
    DefaultBodyLimit::max(LIVE_CHECKPOINT_REQUEST_MAX_BYTES)
}

pub fn create_router() -> axum::Router {
    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .merge(create_openapi_router())
        .split_for_parts();
    router
        .merge(
            SwaggerUi::new("/v1beta/swagger-ui")
                .url("/v1beta/api-docs/openapi.json", api.clone()),
        )
        .merge(Redoc::with_url("/v1beta/redoc", api.clone()))
        // There is no need to create `RapiDoc::with_openapi` because the OpenApi is served
        // via SwaggerUi instead we only make rapidoc to point to the existing doc.
        .merge(
            RapiDoc::new("/v1beta/api-docs/openapi.json")
                .path("/v1beta/rapidoc"),
        )
}

pub fn codegen() -> Result<(), Box<dyn std::error::Error>> {
    let router = create_openapi_router();
    let api_doc = ApiDoc::openapi();
    let merged = OpenApiRouter::with_openapi(api_doc).merge(router);
    let api = merged.get_openapi();
    let json = api.to_json().unwrap();

    // TODO: add English comment
    let yaml_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("library.openapi.yaml");

    // TODO: add English comment
    let json_value: serde_json::Value = serde_json::from_str(&json)?;

    // TODO: add English comment
    let yaml_content = serde_yaml::to_string(&json_value)?;

    // TODO: add English comment
    fs::write(&yaml_path, yaml_content)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        routing::post,
        Json, Router,
    };
    use tower::util::ServiceExt;

    /// The upsert route has to survive two conversions that silently drop it:
    /// `routes!` must find a `#[utoipa::path]` for it, and utoipa-axum must
    /// rewrite `{data_id}` into axum's own parameter syntax. A path that fails
    /// either becomes a 404 at runtime, which is exactly the failure the
    /// endpoint exists to prevent.
    #[test]
    fn the_data_upsert_route_is_registered() {
        let api = OpenApiRouter::with_openapi(ApiDoc::openapi())
            .merge(create_openapi_router())
            .get_openapi()
            .clone();

        let path = api
            .paths
            .paths
            .get("/v1beta/repos/{org}/{repo}/data/{data_id}/upsert")
            .expect("the upsert path must reach the OpenAPI document");
        assert!(path.put.is_some(), "upsert is a PUT");

        // Building the axum router panics on a conflicting route pattern, so
        // this also proves `/data/:data_id/upsert` and the `/md` sibling can
        // coexist.
        let _ = create_router();
    }

    #[tokio::test]
    async fn checkpoint_body_limit_accepts_worker_payload_and_rejects_larger_request(
    ) {
        async fn probe(
            Json(_payload): Json<serde_json::Value>,
        ) -> StatusCode {
            StatusCode::NO_CONTENT
        }

        let router = Router::new().route(
            "/checkpoint",
            post(probe).layer(checkpoint_live_body_limit()),
        );
        let payload = serde_json::json!({
            "property_id": "prop_01hmp05xtq6fs5mmk8fg125cy7",
            "operation_id": "live-test",
            "expected_record_version": "1",
            "format": "markdown",
            "body": "x".repeat(LIVE_CHECKPOINT_BODY_MAX_BYTES),
        });
        let payload = serde_json::to_vec(&payload).unwrap();
        let response = router
            .clone()
            .oneshot(
                Request::post("/checkpoint")
                    .header("content-type", "application/json")
                    .body(Body::from(payload))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        let oversized = vec![b'0'; LIVE_CHECKPOINT_REQUEST_MAX_BYTES + 1];
        let response = router
            .oneshot(
                Request::post("/checkpoint")
                    .header("content-type", "application/json")
                    .body(Body::from(oversized))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }
}
