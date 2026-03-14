use axum::{
    extract::{Extension, FromRequestParts},
    http::request::Parts,
    RequestPartsExt,
};
use std::str::FromStr;
use std::sync::Arc;

use crate::{app::LibraryApp, usecase::LibraryOrg};

use super::extract_org_username;

#[async_trait::async_trait]
impl<S> FromRequestParts<S> for LibraryOrg
where
    S: Send + Sync,
{
    type Rejection = errors::Error;

    #[tracing::instrument(
        name = "library_org_from_request_parts",
        skip(_state)
    )]
    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        // Extract organization username from the path if present
        // This is typically used in routes like /v1beta/orgs/{org} or /v1beta/repos/{org}/{repo}
        let org_username = extract_org_username(parts);

        // Extract the LibraryApp from the request extensions
        // TODO: add English comment
        // TODO: add English comment
        let library_app = parts
            .extract::<Extension<Arc<LibraryApp>>>()
            .await
            .map_err(|e| {
                errors::internal_server_error!(
                    "LibraryApp extension not found. {}",
                    e
                )
            })?;

        if let Some(username) = org_username {
            // Parse the username to OperatorAlias
            let _operator_alias =
                match value_object::Identifier::from_str(&username) {
                    Ok(alias) => alias,
                    Err(_) => {
                        // If the username is not a valid OperatorAlias, return a default LibraryOrg with the username
                        return Ok(LibraryOrg::with_org(username));
                    }
                };

            // Get the organization from the repository
            let org_query = &library_app.view_org;

            // Create a temporary LibraryOrg with the username
            let temp_library_org = LibraryOrg::with_org(username.clone());

            let org = org_query
                .execute(&crate::usecase::ViewOrgInputData {
                    executor: &tachyon_sdk::auth::Executor::SystemUser,
                    multi_tenancy: &temp_library_org,
                    organization_username: username.clone(),
                })
                .await?;

            Ok(LibraryOrg::with_org_and_operator(
                username,
                org.organization.id().clone(),
            ))
        } else {
            // If no organization username is found in the path, return a default LibraryOrg
            Ok(LibraryOrg::default())
        }
    }
}
