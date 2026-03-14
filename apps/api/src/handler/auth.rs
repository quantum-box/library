use std::sync::Arc;

use axum::{Extension, Json};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::app::LibraryApp;

/// Request body for sign-in endpoint
#[derive(Debug, Deserialize, ToSchema)]
pub struct SignInRequest {
    /// Platform ID (tenant ID)
    pub platform_id: String,
    /// Access token from authentication provider (e.g., Cognito)
    pub access_token: String,
    /// Whether to allow sign up if user doesn't exist.
    /// If not provided, defaults to None (sign-in only, no auto sign-up).
    #[serde(default)]
    pub allow_sign_up: Option<bool>,
}

/// Response for sign-in endpoint
#[derive(Debug, Serialize, ToSchema)]
pub struct SignInResponse {
    pub user: UserResponse,
}

/// User information in sign-in response
#[derive(Debug, Serialize, ToSchema)]
pub struct UserResponse {
    pub id: String,
    pub username: String,
    pub email: Option<String>,
    pub name: Option<String>,
    pub role: String,
    pub tenants: Vec<String>,
}

impl From<tachyon_sdk::auth::User> for UserResponse {
    fn from(user: tachyon_sdk::auth::User) -> Self {
        Self {
            id: user.id().to_string(),
            username: user.username().to_string(),
            email: user.email().as_ref().map(|e| e.to_string()),
            name: user.name().as_ref().map(|n| n.to_string()),
            role: user.role().to_string(),
            tenants: user.tenants().iter().map(|t| t.to_string()).collect(),
        }
    }
}

/// Sign in with platform access token.
///
/// This endpoint authenticates a user using an access token from the
/// authentication provider (e.g., Cognito) and returns user information.
#[utoipa::path(
    post,
    path = "/auth/v1beta/sign-in",
    request_body = SignInRequest,
    responses(
        (status = 200, description = "User signed in successfully", body = SignInResponse),
        (status = 400, description = "Bad request - invalid input"),
        (status = 401, description = "Unauthorized - invalid token"),
        (status = 404, description = "User not found and sign up not allowed"),
        (status = 500, description = "Internal server error")
    ),
    tag = "auth"
)]
#[axum::debug_handler]
pub async fn sign_in(
    Extension(library_app): Extension<Arc<LibraryApp>>,
    Json(request): Json<SignInRequest>,
) -> errors::Result<Json<SignInResponse>> {
    let platform_id = request.platform_id.parse()?;

    let user = library_app
        .sign_in
        .execute(platform_id, request.access_token, request.allow_sign_up)
        .await?;

    Ok(Json(SignInResponse { user: user.into() }))
}
