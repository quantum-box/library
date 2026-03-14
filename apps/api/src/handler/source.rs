use std::sync::Arc;

use axum::{
    extract::{Extension, Path as AxumPath},
    http::StatusCode,
    Json,
};

use crate::app::LibraryApp;
use crate::domain::SourceId;
use crate::handler::library_executor_extractor::LibraryExecutor;
use crate::handler::types::{
    CreateSourceRequest, SourceResponse, UpdateSourceRequest,
};
use crate::usecase::{
    CreateSourceInputData, DeleteSourceInputData, FindSourcesInputData,
    GetSourceInputData, UpdateSourceInputData,
};
use tachyon_sdk::auth::ExecutorAction;
use value_object::{Text, Url};

#[utoipa::path(
    get,
    path = "/v1beta/repos/{org}/{repo}/sources/{source_id}",
    params(
        ("org" = String, Path, description = "Organization username"),
        ("repo" = String, Path, description = "Repository username"),
        ("source_id" = String, Path, description = "Source ID")
    ),
    responses(
        (status = 200, description = "Source found", body = SourceResponse),
        (status = 404, description = "Source not found")
    )
)]
#[axum::debug_handler]
pub async fn get_source(
    AxumPath((org, repo, source_id)): AxumPath<(String, String, String)>,
    Extension(library_app): Extension<Arc<LibraryApp>>,
    executor: LibraryExecutor,
    library_org: crate::usecase::LibraryOrg,
) -> errors::Result<Json<SourceResponse>> {
    // TODO: add English comment
    let source_id = source_id.parse::<SourceId>()?;

    // TODO: add English comment
    let source_opt = library_app
        .get_source
        .execute(GetSourceInputData {
            executor: &executor,
            multi_tenancy: &library_org,
            source_id: &source_id,
            org_username: org.clone(),
            repo_username: repo.clone(),
        })
        .await?;

    let source = match source_opt {
        Some(s) => s,
        None => return Err(errors::Error::not_found("source")),
    };

    // TODO: add English comment
    let response = SourceResponse {
        id: source.id().to_string(),
        repo_id: source.repo_id().to_string(),
        name: source.name().to_string(),
        url: source.url().as_ref().map(|u| u.to_string()),
    };

    Ok(Json(response))
}

#[utoipa::path(
    get,
    path = "/v1beta/repos/{org}/{repo}/sources",
    params(
        ("org" = String, Path, description = "Organization username"),
        ("repo" = String, Path, description = "Repository username")
    ),
    responses(
        (status = 200, description = "Sources found", body = Vec<SourceResponse>)
    )
)]
#[axum::debug_handler]
pub async fn find_sources(
    AxumPath((org, repo)): AxumPath<(String, String)>,
    Extension(library_app): Extension<Arc<LibraryApp>>,
    executor: LibraryExecutor,
    library_org: crate::usecase::LibraryOrg,
) -> errors::Result<Json<Vec<SourceResponse>>> {
    // TODO: add English comment
    let _repo_output = library_app
        .view_repo
        .execute(&crate::usecase::ViewRepoInputData {
            executor: &executor,
            multi_tenancy: &library_org,
            organization_username: org.clone(),
            repo_username: repo.clone(),
        })
        .await?;

    // TODO: add English comment
    let sources = library_app
        .find_sources
        .execute(FindSourcesInputData {
            executor: &executor,
            multi_tenancy: &library_org,
            repo_id: _repo_output.repo.id(),
            org_username: org.clone(),
            repo_username: repo.clone(),
        })
        .await?;

    // TODO: add English comment
    let response = sources
        .into_iter()
        .map(|source| SourceResponse {
            id: source.id().to_string(),
            repo_id: source.repo_id().to_string(),
            name: source.name().to_string(),
            url: source.url().as_ref().map(|u| u.to_string()),
        })
        .collect();

    Ok(Json(response))
}

#[utoipa::path(
    post,
    path = "/v1beta/repos/{org}/{repo}/sources",
    request_body = CreateSourceRequest,
    params(
        ("org" = String, Path, description = "Organization username"),
        ("repo" = String, Path, description = "Repository username")
    ),
    responses(
        (status = 201, description = "Source created", body = SourceResponse),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Repository not found")
    )
)]
#[axum::debug_handler]
pub async fn create_source(
    AxumPath((org, repo)): AxumPath<(String, String)>,
    Extension(library_app): Extension<Arc<LibraryApp>>,
    executor: LibraryExecutor,
    library_org: crate::usecase::LibraryOrg,
    Json(payload): Json<CreateSourceRequest>,
) -> errors::Result<(StatusCode, Json<SourceResponse>)> {
    // TODO: add English comment
    let name = payload.name.parse::<Text>()?;
    let url = match payload.url {
        Some(url_str) => Some(url_str.parse::<Url>()?),
        None => None,
    };

    // TODO: add English comment
    let user_id_str = executor.get_id();
    if user_id_str.is_empty() {
        return Err(errors::Error::permission_denied("User ID not found"));
    }

    // TODO: add English comment
    let source = library_app
        .create_source
        .execute(CreateSourceInputData {
            executor: &executor,
            multi_tenancy: &library_org,
            org_username: org.clone(),
            repo_username: repo.clone(),
            name: &name,
            url,
        })
        .await?;

    // TODO: add English comment
    let response = SourceResponse {
        id: source.id().to_string(),
        repo_id: source.repo_id().to_string(),
        name: source.name().to_string(),
        url: source.url().as_ref().map(|u| u.to_string()),
    };

    Ok((StatusCode::CREATED, Json(response)))
}

#[utoipa::path(
    put,
    path = "/v1beta/repos/{org}/{repo}/sources/{source_id}",
    request_body = UpdateSourceRequest,
    params(
        ("org" = String, Path, description = "Organization username"),
        ("repo" = String, Path, description = "Repository username"),
        ("source_id" = String, Path, description = "Source ID")
    ),
    responses(
        (status = 200, description = "Source updated", body = SourceResponse),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Source not found")
    )
)]
#[axum::debug_handler]
pub async fn update_source(
    AxumPath((org, repo, source_id)): AxumPath<(String, String, String)>,
    Extension(library_app): Extension<Arc<LibraryApp>>,
    executor: LibraryExecutor,
    library_org: crate::usecase::LibraryOrg,
    Json(payload): Json<UpdateSourceRequest>,
) -> errors::Result<Json<SourceResponse>> {
    // TODO: add English comment
    let _repo_output = library_app
        .view_repo
        .execute(&crate::usecase::ViewRepoInputData {
            executor: &executor,
            multi_tenancy: &library_org,
            organization_username: org.clone(),
            repo_username: repo.clone(),
        })
        .await?;

    // TODO: add English comment
    let source_id = source_id.parse::<SourceId>()?;

    // TODO: add English comment
    let name = payload
        .name
        .map(|name_str| name_str.parse::<Text>())
        .transpose()?;

    let url = match payload.url {
        Some(url_opt) => Some(match url_opt {
            Some(url_str) => Some(url_str.parse::<Url>()?),
            None => None,
        }),
        None => None,
    };

    // TODO: add English comment
    let source = library_app
        .update_source
        .execute(UpdateSourceInputData {
            executor: &executor,
            multi_tenancy: &library_org,
            source_id: &source_id,
            org_username: org.clone(),
            repo_username: repo.clone(),
            name,
            url,
        })
        .await?;

    // TODO: add English comment
    let response = SourceResponse {
        id: source.id().to_string(),
        repo_id: source.repo_id().to_string(),
        name: source.name().to_string(),
        url: source.url().as_ref().map(|u| u.to_string()),
    };

    Ok(Json(response))
}

#[utoipa::path(
    delete,
    path = "/v1beta/repos/{org}/{repo}/sources/{source_id}",
    params(
        ("org" = String, Path, description = "Organization username"),
        ("repo" = String, Path, description = "Repository username"),
        ("source_id" = String, Path, description = "Source ID")
    ),
    responses(
        (status = 204, description = "Source deleted"),
        (status = 404, description = "Source not found")
    )
)]
#[axum::debug_handler]
pub async fn delete_source(
    AxumPath((org, repo, source_id)): AxumPath<(String, String, String)>,
    Extension(library_app): Extension<Arc<LibraryApp>>,
    executor: LibraryExecutor,
    library_org: crate::usecase::LibraryOrg,
) -> errors::Result<StatusCode> {
    // TODO: add English comment
    let _repo_output = library_app
        .view_repo
        .execute(&crate::usecase::ViewRepoInputData {
            executor: &executor,
            multi_tenancy: &library_org,
            organization_username: org.clone(),
            repo_username: repo.clone(),
        })
        .await?;

    // TODO: add English comment
    let source_id = source_id.parse::<SourceId>()?;

    // TODO: add English comment
    library_app
        .delete_source
        .execute(DeleteSourceInputData {
            executor: &executor,
            multi_tenancy: &library_org,
            source_id: &source_id,
            org_username: org.clone(),
            repo_username: repo.clone(),
        })
        .await?;

    Ok(StatusCode::NO_CONTENT)
}
