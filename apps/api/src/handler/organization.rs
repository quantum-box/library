use std::sync::Arc;

use axum::{
    extract::{Extension, Path as AxumPath},
    Json,
};

use crate::handler::library_executor_extractor::LibraryExecutor;
use crate::handler::types::{
    CreateOrganizationRequest, OrganizationResponse,
    UpdateOrganizationRequest,
};
use crate::usecase::{
    CreateOrganizationInputData, LibraryOrg, UpdateOrganizationInputData,
    ViewOrgInputData,
};
use crate::{app::LibraryApp, LIBRARY_TENANT};
use tachyon_sdk::auth::MultiTenancy;

#[utoipa::path(
    get,
    path = "/v1beta/orgs/{org}",
    params(
        ("org" = String, Path, description = "Organization username")
    ),
    responses(
        (status = 200, description = "Organization found", body = OrganizationResponse),
        (status = 404, description = "Organization not found")
    )
)]
#[axum::debug_handler]
pub async fn view_organization(
    AxumPath(org): AxumPath<String>,
    Extension(library_app): Extension<Arc<LibraryApp>>,
    executor: LibraryExecutor,
    library_org: LibraryOrg,
) -> errors::Result<Json<OrganizationResponse>> {
    let input = ViewOrgInputData {
        executor: &executor,
        multi_tenancy: &library_org,
        organization_username: org,
    };

    let output = library_app.view_org.execute(&input).await?;
    let repos = output
        .repos
        .into_iter()
        .map(|repo| crate::handler::types::RepoResponse {
            id: repo.id().to_string(),
            name: repo.name().to_string(),
            username: repo.username().to_string(),
            description: repo.description().as_ref().map(|d| d.to_string()),
            is_public: *repo.is_public(),
            organization_id: repo.organization_id().to_string(),
        })
        .collect();

    let response = OrganizationResponse {
        id: output.organization.id().to_string(),
        name: output.organization.name().to_string(),
        username: output.organization.username().to_string(),
        description: output
            .organization
            .description()
            .as_ref()
            .map(|d| d.to_string()),
        website: output
            .organization
            .website()
            .as_ref()
            .map(|w| w.to_string()),
        repos,
    };
    Ok(Json(response))
}

#[utoipa::path(
    post,
    path = "/v1beta/orgs",
    request_body = CreateOrganizationRequest,
    responses(
        (status = 201, description = "Organization created", body = OrganizationResponse),
        (status = 400, description = "Invalid request"),
        (status = 409, description = "Organization already exists")
    )
)]
#[axum::debug_handler]
pub async fn create_organization(
    executor: LibraryExecutor,
    Extension(library_app): Extension<Arc<LibraryApp>>,
    Json(payload): Json<CreateOrganizationRequest>,
) -> errors::Result<Json<OrganizationResponse>> {
    let mulit_tenncy = MultiTenancy::new_platform(LIBRARY_TENANT.clone());
    let input = CreateOrganizationInputData {
        executor: &executor,
        multi_tenancy: &mulit_tenncy,
        name: payload.name,
        username: payload.username,
        description: payload.description,
        website: payload.website,
    };

    let output = library_app.create_organization.execute(&input).await?;
    let response = OrganizationResponse {
        id: output.id().to_string(),
        name: output.name().to_string(),
        username: output.username().to_string(),
        description: output.description().as_ref().map(|d| d.to_string()),
        website: output.website().as_ref().map(|w| w.to_string()),
        repos: vec![],
    };
    Ok(Json(response))
}

#[utoipa::path(
    put,
    path = "/v1beta/orgs/{org}",
    request_body = UpdateOrganizationRequest,
    params(
        ("org" = String, Path, description = "Organization username")
    ),
    responses(
        (status = 200, description = "Organization updated", body = OrganizationResponse),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Organization not found")
    )
)]
#[axum::debug_handler]
pub async fn update_organization(
    executor: LibraryExecutor,
    library_org: LibraryOrg,
    AxumPath(org): AxumPath<String>,
    Extension(library_app): Extension<Arc<LibraryApp>>,
    Json(payload): Json<UpdateOrganizationRequest>,
) -> errors::Result<Json<OrganizationResponse>> {
    let input = UpdateOrganizationInputData {
        executor: &executor,
        multi_tenancy: &library_org,
        username: org,
        name: payload.name,
        description: payload.description,
        website: payload.website,
    };

    let output = library_app.update_organization.execute(&input).await?;
    let response = OrganizationResponse {
        id: output.organization.id().to_string(),
        name: output.organization.name().to_string(),
        username: output.organization.username().to_string(),
        description: output
            .organization
            .description()
            .as_ref()
            .map(|d| d.to_string()),
        website: output
            .organization
            .website()
            .as_ref()
            .map(|w| w.to_string()),
        repos: vec![],
    };
    Ok(Json(response))
}
