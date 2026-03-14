use std::sync::Arc;

use axum::{
    extract::{Extension, Path as AxumPath, Query},
    http::StatusCode,
    Json,
};

use crate::app::LibraryApp;
use crate::handler::library_executor_extractor::LibraryExecutor;
use crate::handler::types::{
    ChangeRepoUsernameRequest, CreateRepoRequest, RepoResponse,
    SearchRepoQuery, UpdateRepoRequest,
};
use crate::usecase::LibraryOrg;
use crate::usecase::{
    CreateRepoInputData, DeleteRepoInputData, SearchRepoInputData,
    UpdateRepoInputData, ViewRepoInputData,
};
use tachyon_sdk::auth::ExecutorAction;
use value_object::{LongText, Text};

#[utoipa::path(
    get,
    path = "/v1beta/repos/{org}/{repo}",
    params(
        ("org" = String, Path, description = "Organization username"),
        ("repo" = String, Path, description = "Repository username")
    ),
    responses(
        (status = 200, description = "Repository found", body = RepoResponse),
        (status = 404, description = "Repository not found")
    )
)]
#[axum::debug_handler]
pub async fn view_repo(
    AxumPath((org, repo)): AxumPath<(String, String)>,
    Extension(library_app): Extension<Arc<LibraryApp>>,
    executor: LibraryExecutor,
    library_org: LibraryOrg,
) -> errors::Result<Json<RepoResponse>> {
    let input = ViewRepoInputData {
        executor: &executor,
        multi_tenancy: &library_org,
        organization_username: org,
        repo_username: repo,
    };

    let output = library_app.view_repo.execute(&input).await?;
    let response = RepoResponse {
        id: output.repo.id().to_string(),
        name: output.repo.name().to_string(),
        username: output.repo.username().to_string(),
        description: output
            .repo
            .description()
            .as_ref()
            .map(|d| d.to_string()),
        is_public: *output.repo.is_public(),
        organization_id: output.repo.organization_id().to_string(),
    };
    Ok(Json(response))
}

#[utoipa::path(
    post,
    path = "/v1beta/repos/{org}",
    request_body = CreateRepoRequest,
    params(
        ("org" = String, Path, description = "Organization username"),
    ),
    responses(
        (status = 201, description = "Repository created", body = RepoResponse),
        (status = 400, description = "Invalid request"),
        (status = 409, description = "Repository already exists")
    )
)]
#[axum::debug_handler]
pub async fn create_repo(
    executor: LibraryExecutor,
    library_org: LibraryOrg,
    AxumPath(org): AxumPath<String>,
    Extension(library_app): Extension<Arc<LibraryApp>>,
    Json(payload): Json<CreateRepoRequest>,
) -> errors::Result<Json<RepoResponse>> {
    let user_id = executor.get_id().to_string();
    let input = CreateRepoInputData {
        executor: &executor,
        multi_tenancy: &library_org,
        org_username: org,
        repo_name: payload.name,
        repo_username: payload.username,
        user_id,
        is_public: payload.is_public,
        description: payload.description,
        database_id: payload.database_id,
        skip_sample_data: false,
    };

    let repo = library_app.create_repo.execute(input).await?;
    let response = RepoResponse {
        id: repo.id().to_string(),
        name: repo.name().to_string(),
        username: repo.username().to_string(),
        description: repo.description().as_ref().map(|d| d.to_string()),
        is_public: *repo.is_public(),
        organization_id: repo.organization_id().to_string(),
    };
    Ok(Json(response))
}

#[utoipa::path(
    put,
    path = "/v1beta/repos/{org}/{repo}",
    request_body = UpdateRepoRequest,
    params(
        ("org" = String, Path, description = "Organization username"),
        ("repo" = String, Path, description = "Repository username")
    ),
    responses(
        (status = 200, description = "Repository updated", body = RepoResponse),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Repository not found")
    )
)]
#[axum::debug_handler]
pub async fn update_repo(
    executor: LibraryExecutor,
    library_org: LibraryOrg,
    AxumPath((org, repo)): AxumPath<(String, String)>,
    Extension(library_app): Extension<Arc<LibraryApp>>,
    Json(payload): Json<UpdateRepoRequest>,
) -> errors::Result<Json<RepoResponse>> {
    let name = payload
        .name
        .map(|value| {
            value
                .parse::<Text>()
                .map_err(|err| errors::Error::invalid(err.to_string()))
        })
        .transpose()?;
    let description = payload
        .description
        .map(|value| {
            value
                .parse::<LongText>()
                .map_err(|err| errors::Error::invalid(err.to_string()))
        })
        .transpose()?;
    let tags = payload
        .tags
        .map(|values| {
            values
                .into_iter()
                .map(|value| {
                    value.parse::<Text>().map_err(|err| {
                        errors::Error::invalid(err.to_string())
                    })
                })
                .collect::<errors::Result<Vec<Text>>>()
        })
        .transpose()?;

    let input = UpdateRepoInputData {
        executor: &executor,
        multi_tenancy: &library_org,
        org_username: org,
        repo_username: repo,
        name,
        description,
        is_public: payload.is_public,
        tags,
    };

    let repo = library_app.update_repo.execute(input).await?;
    let response = RepoResponse {
        id: repo.id().to_string(),
        name: repo.name().to_string(),
        username: repo.username().to_string(),
        description: repo.description().as_ref().map(|d| d.to_string()),
        is_public: *repo.is_public(),
        organization_id: repo.organization_id().to_string(),
    };
    Ok(Json(response))
}

#[utoipa::path(
    delete,
    path = "/v1beta/repos/{org}/{repo}",
    params(
        ("org" = String, Path, description = "Organization username"),
        ("repo" = String, Path, description = "Repository username")
    ),
    responses(
        (status = 204, description = "Repository deleted"),
        (status = 404, description = "Repository not found")
    )
)]
#[axum::debug_handler]
pub async fn delete_repo(
    AxumPath((org, repo)): AxumPath<(String, String)>,
    Extension(library_app): Extension<Arc<LibraryApp>>,
    executor: LibraryExecutor,
    library_org: LibraryOrg,
) -> errors::Result<StatusCode> {
    let input = DeleteRepoInputData {
        executor: &executor,
        multi_tenancy: &library_org,
        org_username: org,
        repo_username: repo,
    };

    library_app.delete_repo.execute(input).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    put,
    path = "/v1beta/repos/{org}/{repo}/change-username",
    request_body = ChangeRepoUsernameRequest,
    params(
        ("org" = String, Path, description = "Organization username"),
        ("repo" = String, Path, description = "Current repository username")
    ),
    responses(
        (status = 200, description = "Repository username changed", body = RepoResponse),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Repository not found"),
        (status = 409, description = "Repository with new username already exists")
    )
)]
#[axum::debug_handler]
pub async fn change_repo_username(
    AxumPath((org, repo)): AxumPath<(String, String)>,
    Extension(library_app): Extension<Arc<LibraryApp>>,
    Json(payload): Json<ChangeRepoUsernameRequest>,
) -> errors::Result<Json<RepoResponse>> {
    let input = crate::usecase::ChangeRepoUsernameInput {
        org_username: org,
        old_repo_username: repo,
        new_repo_username: payload.new_username,
    };

    let updated_repo =
        library_app.change_repo_username.execute(input).await?;
    let response = RepoResponse {
        id: updated_repo.id().to_string(),
        name: updated_repo.name().to_string(),
        username: updated_repo.username().to_string(),
        description: updated_repo
            .description()
            .as_ref()
            .map(|d| d.to_string()),
        is_public: *updated_repo.is_public(),
        organization_id: updated_repo.organization_id().to_string(),
    };
    Ok(Json(response))
}

#[utoipa::path(
    get,
    path = "/v1beta/repos",
    params(
        ("name" = Option<String>, Query, description = "Repository name to search for"),
        ("limit" = Option<i64>, Query, description = "Maximum number of results to return")
    ),
    responses(
        (status = 200, description = "Repositories found", body = Vec<RepoResponse>)
    )
)]
#[axum::debug_handler]
pub async fn search_repo(
    Query(query): Query<SearchRepoQuery>,
    Extension(library_app): Extension<Arc<LibraryApp>>,
) -> errors::Result<Json<Vec<RepoResponse>>> {
    let input = SearchRepoInputData {
        org_username: None,
        name: query.name,
        limit: query.limit,
    };

    let repos = library_app.search_repo.execute(&input).await?;
    let response: Vec<RepoResponse> = repos
        .into_iter()
        .map(|repo| RepoResponse {
            id: repo.id().to_string(),
            name: repo.name().to_string(),
            username: repo.username().to_string(),
            description: repo.description().as_ref().map(|d| d.to_string()),
            is_public: *repo.is_public(),
            organization_id: repo.organization_id().to_string(),
        })
        .collect();
    Ok(Json(response))
}
