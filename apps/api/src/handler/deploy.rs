use axum::{
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    Json,
};
use serde::Serialize;

#[derive(Serialize)]
pub struct DeployMigrationResponse {
    status: &'static str,
}

pub async fn run_migrations(
    headers: HeaderMap,
) -> Result<Json<DeployMigrationResponse>, StatusCode> {
    let expected_token = std::env::var("LIBRARY_DEPLOY_HOOK_TOKEN")
        .map_err(|_| {
            tracing::error!("deploy migration rejected because hook auth is unavailable");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    if !bearer_token_matches(&headers, &expected_token) {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let database_url = crate::migrations::resolve_prod_database_url()
        .map_err(|_| {
            tracing::error!(
                "deploy migration could not resolve the production database"
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    crate::migrations::run_library_migrations(&database_url)
        .await
        .map_err(|_| {
            tracing::error!("deploy migration failed");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tracing::info!("deploy migration completed successfully");
    Ok(Json(DeployMigrationResponse { status: "ok" }))
}

fn bearer_token_matches(headers: &HeaderMap, expected: &str) -> bool {
    let Some(provided) = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
    else {
        return false;
    };
    constant_time_compare(provided.as_bytes(), expected.as_bytes())
}

fn constant_time_compare(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }

    left.iter()
        .zip(right)
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == 0
}

#[cfg(test)]
mod tests {
    use super::bearer_token_matches;
    use axum::http::{header::AUTHORIZATION, HeaderMap, HeaderValue};

    #[test]
    fn accepts_matching_bearer_token() {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_static("Bearer deploy-secret"),
        );

        assert!(bearer_token_matches(&headers, "deploy-secret"));
    }

    #[test]
    fn rejects_missing_or_different_bearer_token() {
        assert!(!bearer_token_matches(&HeaderMap::new(), "deploy-secret"));

        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_static("Bearer another-secret"),
        );
        assert!(!bearer_token_matches(&headers, "deploy-secret"));
    }
}
