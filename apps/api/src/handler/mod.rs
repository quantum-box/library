pub mod auth;
pub mod data;
pub mod graphql;
pub mod library_executor_extractor;
pub mod library_org_extractor;
pub mod openapi;
pub mod organization;
pub mod property;
pub mod repository;
pub mod source;
pub mod types;

pub use self::openapi::codegen;
pub use self::openapi::create_router;

pub fn extract_org_username(
    parts: &axum::http::request::Parts,
) -> Option<String> {
    let path = parts.uri.path();
    let org_username = if path.contains("/orgs/") {
        // Extract the organization username from the path
        // Format: /v1beta/orgs/{org}
        let segments: Vec<&str> = path.split('/').collect();
        let org_index = segments.iter().position(|&s| s == "orgs");

        if let Some(org_index) = org_index {
            // The organization username should be the segment after "orgs"
            if segments.len() > org_index + 1 {
                Some(segments[org_index + 1].to_string())
            } else {
                None
            }
        } else {
            None
        }
    } else if path.contains("/repos/") {
        // Extract the organization username from the path
        // Format: /v1beta/repos/{org}/{repo}
        let segments: Vec<&str> = path.split('/').collect();
        let repos_index = segments.iter().position(|&s| s == "repos");

        if let Some(repos_index) = repos_index {
            // The organization username should be the segment after "repos"
            if segments.len() > repos_index + 1 {
                Some(segments[repos_index + 1].to_string())
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };
    org_username
}
