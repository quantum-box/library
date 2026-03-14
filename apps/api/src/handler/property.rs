use std::sync::Arc;

use axum::{
    extract::{Extension, Path as AxumPath},
    Json,
};

use crate::app::LibraryApp;
use crate::handler::library_executor_extractor::LibraryExecutor;
use crate::handler::types::{
    AddPropertyRequest, PropertyResponse, UpdatePropertyRequest,
};
use crate::usecase::LibraryOrg;
use crate::usecase::{
    AddPropertyInputData, DeletePropertyInputData, GetPropertiesInputData,
    GetPropertyInputData,
};
use database_manager::domain::{Property as DomainProperty, PropertyType};

#[utoipa::path(
    get,
    path = "/v1beta/repos/{org}/{repo}/properties",
    params(
        ("org" = String, Path, description = "Organization username"),
        ("repo" = String, Path, description = "Repository username")
    ),
    responses(
        (status = 200, description = "Properties found", body = Vec<PropertyResponse>),
        (status = 404, description = "Repository not found")
    )
)]
#[axum::debug_handler]
pub async fn get_properties(
    AxumPath((org, repo)): AxumPath<(String, String)>,
    Extension(library_app): Extension<Arc<LibraryApp>>,
    executor: LibraryExecutor,
    library_org: LibraryOrg,
) -> errors::Result<Json<Vec<PropertyResponse>>> {
    let input = GetPropertiesInputData {
        executor: &executor,
        multi_tenancy: &library_org,
        org_username: org,
        repo_username: repo,
    };

    let properties = library_app.get_properties.execute(input).await?;
    let response: Vec<PropertyResponse> = properties
        .into_iter()
        .map(|property| to_property_response(&property))
        .collect();
    Ok(Json(response))
}

#[utoipa::path(
    post,
    path = "/v1beta/repos/{org}/{repo}/properties",
    request_body = AddPropertyRequest,
    params(
        ("org" = String, Path, description = "Organization username"),
        ("repo" = String, Path, description = "Repository username")
    ),
    responses(
        (status = 201, description = "Property created", body = PropertyResponse),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Repository not found")
    )
)]
#[axum::debug_handler]
pub async fn add_property(
    executor: LibraryExecutor,
    library_org: LibraryOrg,
    AxumPath((org, repo)): AxumPath<(String, String)>,
    Extension(library_app): Extension<Arc<LibraryApp>>,
    Json(payload): Json<AddPropertyRequest>,
) -> errors::Result<Json<PropertyResponse>> {
    let property_type = match payload.property_type.as_str() {
        "string" => PropertyType::String,
        "integer" => PropertyType::Integer,
        "html" => {
            tracing::warn!("{}", HTML_DEPRECATION_MESSAGE);
            PropertyType::Html
        }
        "markdown" => PropertyType::Markdown,
        "relation" => PropertyType::Relation(Default::default()),
        "select" => PropertyType::Select(Default::default()),
        "multi_select" => PropertyType::MultiSelect(Default::default()),
        "location" => PropertyType::Location(Default::default()),
        "image" => PropertyType::Image,
        _ => {
            return Err(errors::Error::invalid("Invalid property type"));
        }
    };

    let input = AddPropertyInputData {
        executor: &executor,
        multi_tenancy: &library_org,
        org_username: org,
        repo_username: repo,
        property_name: payload.name,
        property_type,
    };

    let property = library_app.add_property.execute(input).await?;
    let response = to_property_response(&property);
    Ok(Json(response))
}

#[utoipa::path(
    get,
    path = "/v1beta/repos/{org}/{repo}/properties/{property_id}",
    params(
        ("org" = String, Path, description = "Organization username"),
        ("repo" = String, Path, description = "Repository username"),
        ("property_id" = String, Path, description = "Property ID")
    ),
    responses(
        (status = 200, description = "Property found", body = PropertyResponse),
        (status = 404, description = "Property not found")
    )
)]
#[axum::debug_handler]
pub async fn get_property(
    AxumPath((org, repo, property_id)): AxumPath<(String, String, String)>,
    Extension(library_app): Extension<Arc<LibraryApp>>,
    executor: LibraryExecutor,
    library_org: LibraryOrg,
) -> errors::Result<Json<PropertyResponse>> {
    let input = GetPropertyInputData {
        executor: &executor,
        multi_tenancy: &library_org,
        org_username: org,
        repo_username: repo,
        property_id: property_id.clone(),
    };

    let properties = library_app
        .get_properties
        .execute(GetPropertiesInputData {
            executor: &executor,
            multi_tenancy: &library_org,
            org_username: input.org_username,
            repo_username: input.repo_username,
        })
        .await?;

    let property = properties
        .into_iter()
        .find(|p| *p.id() == input.property_id)
        .ok_or_else(|| errors::Error::not_found("Property not found"))?;

    let response = to_property_response(&property);
    Ok(Json(response))
}

#[utoipa::path(
    delete,
    path = "/v1beta/repos/{org}/{repo}/properties/{property_id}",
    params(
        ("org" = String, Path, description = "Organization username"),
        ("repo" = String, Path, description = "Repository username"),
        ("property_id" = String, Path, description = "Property ID")
    ),
    responses(
        (status = 204, description = "Property deleted"),
        (status = 404, description = "Property not found")
    )
)]
pub async fn delete_property(
    AxumPath((org, repo, property_id)): AxumPath<(String, String, String)>,
    Extension(library_app): Extension<Arc<LibraryApp>>,
    executor: LibraryExecutor,
    library_org: LibraryOrg,
) -> errors::Result<()> {
    let input = DeletePropertyInputData {
        executor: &executor,
        multi_tenancy: &library_org,
        org_username: org,
        repo_username: repo,
        property_id,
    };

    library_app.delete_property.execute(input).await?;
    Ok(())
}

#[utoipa::path(
    put,
    path = "/v1beta/repos/{org}/{repo}/properties/{property_id}",
    request_body = UpdatePropertyRequest,
    params(
        ("org" = String, Path, description = "Organization username"),
        ("repo" = String, Path, description = "Repository username"),
        ("property_id" = String, Path, description = "Property ID")
    ),
    responses(
        (status = 200, description = "Property updated", body = PropertyResponse),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Property not found")
    )
)]
#[axum::debug_handler]
pub async fn update_property(
    executor: LibraryExecutor,
    library_org: LibraryOrg,
    AxumPath((org, repo, property_id)): AxumPath<(String, String, String)>,
    Extension(library_app): Extension<Arc<LibraryApp>>,
    Json(payload): Json<UpdatePropertyRequest>,
) -> errors::Result<Json<PropertyResponse>> {
    let input = crate::usecase::UpdatePropertyInputData {
        executor: &executor,
        multi_tenancy: &library_org,
        org_username: org,
        repo_username: repo,
        property_id,
        property_name: Some(payload.name),
        property_type: None,
        meta_json: None,
    };

    let property = library_app.update_property.execute(input).await?;
    let response = to_property_response(&property);
    Ok(Json(response))
}
const HTML_DEPRECATION_MESSAGE: &str =
    "HTML property type is deprecated. Please migrate to MARKDOWN.";

fn property_deprecation(property_type: &str) -> Option<String> {
    if property_type.eq_ignore_ascii_case("html") {
        Some(HTML_DEPRECATION_MESSAGE.to_string())
    } else {
        None
    }
}

fn to_property_response(property: &DomainProperty) -> PropertyResponse {
    let property_type = property.property_type().to_string();
    PropertyResponse {
        id: property.id().to_string(),
        name: property.name().to_string(),
        property_type: property_type.clone(),
        deprecation: property_deprecation(&property_type),
    }
}
