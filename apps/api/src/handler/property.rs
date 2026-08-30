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
use database_manager::domain::{
    Property as DomainProperty, PropertyType, TypeId,
};

fn property_type_from_request(
    payload: &AddPropertyRequest,
) -> errors::Result<PropertyType> {
    if payload.property_type != "id" && payload.auto_generate.is_some() {
        return Err(errors::Error::invalid(
            "auto_generate is only valid for an Id property",
        ));
    }

    match payload.property_type.as_str() {
        "string" => Ok(PropertyType::String),
        "integer" => Ok(PropertyType::Integer),
        "html" => {
            tracing::warn!("{}", HTML_DEPRECATION_MESSAGE);
            Ok(PropertyType::Html)
        }
        "markdown" => Ok(PropertyType::Markdown),
        "relation" => Ok(PropertyType::Relation(Default::default())),
        "select" => Ok(PropertyType::Select(Default::default())),
        "multi_select" => Ok(PropertyType::MultiSelect(Default::default())),
        "id" => Ok(PropertyType::Id(TypeId::new(
            payload.auto_generate.ok_or_else(|| {
                errors::Error::invalid(
                    "auto_generate is required for an Id property",
                )
            })?,
        ))),
        "location" => Ok(PropertyType::Location(Default::default())),
        "date" => Ok(PropertyType::Date),
        "image" => Ok(PropertyType::Image),
        "rich_text" => Ok(PropertyType::RichText),
        _ => Err(errors::Error::invalid("Invalid property type")),
    }
}

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
    let property_type = property_type_from_request(&payload)?;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rest_id_property_preserves_auto_generate() {
        let property_type =
            property_type_from_request(&AddPropertyRequest {
                name: "id".to_string(),
                property_type: "id".to_string(),
                auto_generate: Some(true),
            })
            .expect("Id must be supported by the REST adapter");

        assert!(matches!(
            property_type,
            PropertyType::Id(TypeId {
                auto_generate: true
            })
        ));
    }

    #[test]
    fn rest_id_property_requires_auto_generate() {
        let error = property_type_from_request(&AddPropertyRequest {
            name: "id".to_string(),
            property_type: "id".to_string(),
            auto_generate: None,
        })
        .expect_err("Id metadata must not silently default");

        assert!(error.is_bad_request());
        assert!(error.to_string().contains("auto_generate is required"));
    }

    /// `date` is a first-class property type — the MCP adapter accepts it
    /// and repositories store it — but the REST adapter had no arm for
    /// it, so `POST /properties` answered "Invalid property type".
    #[test]
    fn rest_date_property_is_accepted() {
        let property_type =
            property_type_from_request(&AddPropertyRequest {
                name: "published".to_string(),
                property_type: "date".to_string(),
                auto_generate: None,
            })
            .expect("Date must be supported by the REST adapter");

        assert!(matches!(property_type, PropertyType::Date));
    }

    #[test]
    fn rest_non_id_property_rejects_auto_generate() {
        let error = property_type_from_request(&AddPropertyRequest {
            name: "title".to_string(),
            property_type: "string".to_string(),
            auto_generate: Some(true),
        })
        .expect_err("Id metadata must not leak into other property types");

        assert!(error.is_bad_request());
        assert!(error.to_string().contains("only valid for an Id"));
    }
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
        auto_generate: match property.property_type() {
            PropertyType::Id(type_id) => Some(type_id.auto_generate),
            _ => None,
        },
        deprecation: property_deprecation(&property_type),
    }
}
