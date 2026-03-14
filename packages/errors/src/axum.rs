use crate::{display::format_backtrace, Error};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use std::backtrace::Backtrace;
use utoipa::ToSchema;

/// Common error response body for REST APIs
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponse {
    /// Error message
    pub message: String,
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        match self {
            // 5xx errors - server side issues (ERROR level)
            Error::InternalServerError { message, backtrace } => {
                log_server_error(&message);
                log_debug_backtrace(&message, &backtrace);
                create_response(StatusCode::INTERNAL_SERVER_ERROR, message)
            }
            Error::ServiceUnavailable { message, backtrace } => {
                log_server_error(&message);
                log_debug_backtrace(&message, &backtrace);
                create_response(StatusCode::SERVICE_UNAVAILABLE, message)
            }
            // 4xx errors - client side issues (WARN level)
            Error::BadRequest { message, backtrace } => {
                log_client_error(&message);
                log_debug_backtrace(&message, &backtrace);
                create_response(StatusCode::BAD_REQUEST, message)
            }
            Error::Unauthorized { message, backtrace } => {
                log_client_error(&message);
                log_debug_backtrace(&message, &backtrace);
                create_response(StatusCode::UNAUTHORIZED, message)
            }
            Error::Forbidden { message, backtrace } => {
                log_client_error(&message);
                log_debug_backtrace(&message, &backtrace);
                create_response(StatusCode::FORBIDDEN, message)
            }
            Error::NotFound { message, backtrace } => {
                log_client_error(&message);
                log_debug_backtrace(&message, &backtrace);
                create_response(StatusCode::NOT_FOUND, message)
            }
            Error::Conflict { message, backtrace } => {
                log_client_error(&message);
                log_debug_backtrace(&message, &backtrace);
                create_response(StatusCode::CONFLICT, message)
            }
            Error::PaymentRequired { message, backtrace } => {
                log_client_error(&message);
                log_debug_backtrace(&message, &backtrace);
                create_response(StatusCode::PAYMENT_REQUIRED, message)
            }
        }
    }
}

fn create_response(status: StatusCode, message: String) -> Response {
    let body = Json(ErrorResponse { message });
    (status, body).into_response()
}

/// Log server-side errors (5xx) at ERROR level
fn log_server_error(message: &str) {
    tracing::error!(message = %message, error_type = "server", "server error response");
}

/// Log client-side errors (4xx) at WARN level
fn log_client_error(message: &str) {
    tracing::warn!(message = %message, error_type = "client", "client error response");
}

fn log_debug_backtrace(message: &str, backtrace: &Backtrace) {
    if tracing::level_enabled!(tracing::Level::DEBUG) {
        let formatted = format_backtrace(backtrace);
        if !formatted.is_empty() {
            tracing::debug!(
                message = %message,
                backtrace = %formatted,
                "error backtrace"
            );
        }
    }
}
