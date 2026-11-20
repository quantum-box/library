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
            // The handler acts on the organization its own path extractor
            // decodes. A segment that is not a plain organization username
            // (a percent-encoded one, say) would be authorized as no
            // organization at all, so it is refused rather than let through.
            let operator_alias = parse_org_segment(&username)?;

            // Only the organization's id is needed here. `view_org` would
            // also load every repository the organization owns, and this
            // runs on every org-scoped request.
            let org = library_app
                .get_organization_by_username
                .execute(&operator_alias)
                .await?
                .ok_or(errors::not_found!("organization is not found"))?;

            Ok(library_app.org_scopes.scope_for(&org).await)
        } else {
            // If no organization username is found in the path, return a default LibraryOrg
            Ok(LibraryOrg::default())
        }
    }
}

/// Parse the raw organization segment of a request path.
fn parse_org_segment(
    segment: &str,
) -> errors::Result<value_object::Identifier> {
    if segment.contains('%') {
        return Err(errors::Error::bad_request(
            "The organization in the path must not be percent-encoded",
        ));
    }
    value_object::Identifier::from_str(segment).map_err(|_| {
        errors::Error::bad_request(
            "The organization in the path is invalid",
        )
    })
}

#[cfg(test)]
mod tests {
    use super::parse_org_segment;

    #[test]
    fn a_plain_organization_username_parses() {
        assert_eq!(
            parse_org_segment("victim-org").unwrap().to_string(),
            "victim-org"
        );
    }

    #[test]
    fn a_percent_encoded_organization_is_refused() {
        let err = parse_org_segment("victim%2Dorg").unwrap_err();
        assert!(matches!(err, errors::Error::BadRequest { .. }), "{err:?}");
    }

    #[test]
    fn an_invalid_organization_is_refused() {
        let err = parse_org_segment("not an org!").unwrap_err();
        assert!(matches!(err, errors::Error::BadRequest { .. }), "{err:?}");
    }
}
