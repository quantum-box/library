use std::fs;
use std::path::Path;

use utoipa::OpenApi;
use utoipa_axum::{router::OpenApiRouter, routes};
use utoipa_rapidoc::RapiDoc;
use utoipa_redoc::{Redoc, Servable};
use utoipa_swagger_ui::SwaggerUi;

use crate::handler::{
    auth::*, data::*, organization::*, property::*, repository::*,
    source::*,
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
        view_data_parquet,
        add_data,
        update_data,
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
        crate::handler::types::DataResponse,
        crate::handler::types::ParquetResponse,
        crate::handler::types::AddDataRequest,
        crate::handler::types::UpdateDataRequest,
        crate::handler::types::SearchDataQuery,
        crate::handler::types::PropertyResponse,
        crate::handler::types::AddPropertyRequest,
        crate::handler::types::UpdatePropertyRequest,
        crate::handler::types::SourceResponse,
        crate::handler::types::CreateSourceRequest,
        crate::handler::types::UpdateSourceRequest
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
