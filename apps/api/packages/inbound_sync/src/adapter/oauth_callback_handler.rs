//! OAuth callback handler for integration authentication.
//!
//! This handler receives OAuth callbacks from external providers and exchanges
//! the authorization code for access tokens.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
    routing::get,
    Router,
};
use serde::Deserialize;
use std::sync::Arc;

use inbound_sync_domain::{
    Connection, ConnectionId, ConnectionRepository, ExchangeOAuthCodeInput,
    IntegrationId, IntegrationRepository, OAuthService,
};
use value_object::TenantId;

/// State for OAuth callback handlers.
#[derive(Clone)]
pub struct OAuthCallbackState {
    /// OAuth service for token exchange
    pub oauth_service: Arc<dyn OAuthService>,
    /// Integration repository to look up integrations
    pub integration_repository: Arc<dyn IntegrationRepository>,
    /// Connection repository to save connections
    pub connection_repository: Arc<dyn ConnectionRepository>,
    /// Base URL for redirects (frontend URL)
    pub frontend_base_url: String,
}

/// Query parameters from OAuth callback.
#[derive(Debug, Deserialize)]
pub struct OAuthCallbackQuery {
    /// Authorization code from provider
    pub code: String,
    /// State parameter for CSRF protection
    pub state: Option<String>,
    /// Error from provider (if any)
    pub error: Option<String>,
    /// Error description
    pub error_description: Option<String>,
}

/// Path parameters for OAuth callback.
#[derive(Debug, Deserialize)]
pub struct OAuthCallbackPath {
    /// Tenant ID
    pub tenant_id: String,
    /// Integration ID
    pub integration_id: String,
}

/// Create the OAuth callback router.
///
/// # Routes
///
/// - `GET /v1beta/:tenant_id/integrations/callback` - Handle OAuth callback
/// - `GET /v1beta/:tenant_id/integrations/:integration_id/callback` - Handle OAuth callback for specific integration
///
/// # Example
///
/// ```ignore
/// let state = OAuthCallbackState {
///     oauth_service: Arc::new(oauth_service),
///     integration_repository: Arc::new(integration_repo),
///     connection_repository: Arc::new(connection_repo),
///     frontend_base_url: "https://app.example.com".to_string(),
/// };
/// let router = create_oauth_callback_router(state);
/// ```
pub fn create_oauth_callback_router(state: OAuthCallbackState) -> Router {
    Router::new()
        .route(
            "/v1beta/:tenant_id/integrations/callback",
            get(handle_oauth_callback_generic),
        )
        .route(
            "/v1beta/:tenant_id/integrations/:integration_id/callback",
            get(handle_oauth_callback),
        )
        .with_state(state)
}

/// Handle OAuth callback for a specific integration.
async fn handle_oauth_callback(
    State(state): State<OAuthCallbackState>,
    Path(path): Path<OAuthCallbackPath>,
    Query(query): Query<OAuthCallbackQuery>,
) -> impl IntoResponse {
    // Check for error from provider
    if let Some(error) = query.error {
        let error_desc = query
            .error_description
            .unwrap_or_else(|| "Unknown error".to_string());
        tracing::warn!(
            tenant_id = %path.tenant_id,
            integration_id = %path.integration_id,
            error = %error,
            description = %error_desc,
            "OAuth callback received error from provider"
        );

        let redirect_url = format!(
            "{}/v1beta/{}/integrations?error={}&error_description={}",
            state.frontend_base_url,
            path.tenant_id,
            urlencoding::encode(&error),
            urlencoding::encode(&error_desc)
        );
        return Redirect::temporary(&redirect_url).into_response();
    }

    // Parse tenant ID
    let tenant_id = match path.tenant_id.parse::<TenantId>() {
        Ok(id) => id,
        Err(e) => {
            tracing::error!(error = %e, "Invalid tenant ID in OAuth callback");
            return (StatusCode::BAD_REQUEST, "Invalid tenant ID")
                .into_response();
        }
    };

    let integration_id = IntegrationId::new(&path.integration_id);

    // Look up integration to get provider
    let integration = match state
        .integration_repository
        .find_by_id(&integration_id)
        .await
    {
        Ok(Some(i)) => i,
        Ok(None) => {
            tracing::error!(
                integration_id = %path.integration_id,
                "Integration not found in OAuth callback"
            );
            let redirect_url = format!(
                "{}/v1beta/{}/integrations?error=integration_not_found",
                state.frontend_base_url, path.tenant_id
            );
            return Redirect::temporary(&redirect_url).into_response();
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to look up integration");
            let redirect_url = format!(
                "{}/v1beta/{}/integrations?error=internal_error",
                state.frontend_base_url, path.tenant_id
            );
            return Redirect::temporary(&redirect_url).into_response();
        }
    };

    // Build redirect URI (must match what was used in init)
    let redirect_uri = format!(
        "{}/v1beta/{}/integrations/{}/callback",
        state.frontend_base_url, path.tenant_id, path.integration_id
    );

    // Exchange code for tokens
    let exchange_input = ExchangeOAuthCodeInput {
        tenant_id: tenant_id.clone(),
        provider: integration.provider(),
        code: query.code,
        state: query.state,
        redirect_uri,
    };

    let stored_token = match state
        .oauth_service
        .exchange_code(exchange_input)
        .await
    {
        Ok(token) => token,
        Err(e) => {
            tracing::error!(error = %e, "Failed to exchange OAuth code");
            let redirect_url = format!(
                "{}/v1beta/{}/integrations?error=token_exchange_failed&error_description={}",
                state.frontend_base_url,
                path.tenant_id,
                urlencoding::encode(&e.to_string())
            );
            return Redirect::temporary(&redirect_url).into_response();
        }
    };

    // Check for existing connection
    let existing = state
        .connection_repository
        .find_by_tenant_and_integration(&tenant_id, &integration_id)
        .await
        .ok()
        .flatten();

    // Create or update connection
    let connection = if let Some(mut conn) = existing {
        conn.set_external_account(
            stored_token.external_account_id.clone(),
            stored_token.external_account_name.clone(),
        );
        conn.set_token_expires_at(stored_token.expires_at);
        conn.resume();
        conn
    } else {
        let mut conn = Connection::create(
            ConnectionId::generate(),
            tenant_id.clone(),
            integration_id.clone(),
            integration.provider(),
        );
        conn.set_external_account(
            stored_token.external_account_id.clone(),
            stored_token.external_account_name.clone(),
        );
        conn.set_token_expires_at(stored_token.expires_at);
        conn
    };

    // Save connection
    if let Err(e) = state.connection_repository.save(&connection).await {
        tracing::error!(error = %e, "Failed to save connection");
        let redirect_url = format!(
            "{}/v1beta/{}/integrations?error=save_failed",
            state.frontend_base_url, path.tenant_id
        );
        return Redirect::temporary(&redirect_url).into_response();
    }

    tracing::info!(
        tenant_id = %tenant_id,
        integration_id = %path.integration_id,
        connection_id = %connection.id(),
        "OAuth connection established successfully"
    );

    // Redirect to success page
    let redirect_url = format!(
        "{}/v1beta/{}/integrations?connected={}&integration_id={}",
        state.frontend_base_url,
        path.tenant_id,
        connection.id(),
        path.integration_id
    );

    Redirect::temporary(&redirect_url).into_response()
}

/// Generic OAuth callback path (without integration_id).
#[derive(Debug, Deserialize)]
pub struct GenericOAuthCallbackPath {
    /// Tenant ID
    pub tenant_id: String,
}

/// Generic OAuth callback query with integration info in state.
#[derive(Debug, Deserialize)]
pub struct GenericOAuthCallbackQuery {
    /// Authorization code from provider
    pub code: String,
    /// State parameter containing integration_id and CSRF token
    pub state: Option<String>,
    /// Error from provider (if any)
    pub error: Option<String>,
    /// Error description
    pub error_description: Option<String>,
}

/// Handle OAuth callback without integration ID in path.
///
/// The integration ID should be encoded in the state parameter.
async fn handle_oauth_callback_generic(
    State(state): State<OAuthCallbackState>,
    Path(path): Path<GenericOAuthCallbackPath>,
    Query(query): Query<GenericOAuthCallbackQuery>,
) -> impl IntoResponse {
    // Check for error from provider
    if let Some(error) = query.error {
        let error_desc = query
            .error_description
            .unwrap_or_else(|| "Unknown error".to_string());
        let redirect_url = format!(
            "{}/v1beta/{}/integrations?error={}&error_description={}",
            state.frontend_base_url,
            path.tenant_id,
            urlencoding::encode(&error),
            urlencoding::encode(&error_desc)
        );
        return Redirect::temporary(&redirect_url).into_response();
    }

    // Extract integration ID from state
    // State format: "<integration_id>:<csrf_token>" or just "<csrf_token>"
    let (integration_id, csrf_state) = match &query.state {
        Some(s) if s.contains(':') => {
            let parts: Vec<&str> = s.splitn(2, ':').collect();
            (Some(parts[0].to_string()), Some(parts[1].to_string()))
        }
        other => (None, other.clone()),
    };

    let integration_id = match integration_id {
        Some(id) => id,
        None => {
            tracing::error!("No integration ID in OAuth callback state");
            let redirect_url = format!(
                "{}/v1beta/{}/integrations?error=missing_integration_id",
                state.frontend_base_url, path.tenant_id
            );
            return Redirect::temporary(&redirect_url).into_response();
        }
    };

    // Delegate to the specific handler
    let full_path = OAuthCallbackPath {
        tenant_id: path.tenant_id,
        integration_id,
    };

    let full_query = OAuthCallbackQuery {
        code: query.code,
        state: csrf_state,
        error: None,
        error_description: None,
    };

    handle_oauth_callback(State(state), Path(full_path), Query(full_query))
        .await
        .into_response()
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_state_parsing() {
        // With integration ID
        let state = "int_123:csrf_abc";
        let parts: Vec<&str> = state.splitn(2, ':').collect();
        assert_eq!(parts[0], "int_123");
        assert_eq!(parts[1], "csrf_abc");

        // Without integration ID (just CSRF)
        let state = "csrf_only";
        assert!(!state.contains(':'));
    }
}
