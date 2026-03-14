//! Auth error utilities.
//!
//! The SDK auth module uses `errors::Result<T>` for trait
//! method return types to maintain compatibility with the
//! rest of the codebase. This module provides helpers for
//! converting HTTP/SDK errors into `errors::Error`.

/// Convert a reqwest error to `errors::Error`.
pub fn from_http(e: reqwest::Error) -> errors::Error {
    errors::Error::internal_server_error(e)
}

/// Convert an SDK API error response to `errors::Error`
/// based on the HTTP status code.
pub fn from_status(
    status: reqwest::StatusCode,
    message: impl ToString,
) -> errors::Error {
    let msg = message.to_string();
    match status.as_u16() {
        401 => errors::Error::unauthorized(msg),
        403 => errors::Error::forbidden(msg),
        404 => errors::Error::not_found(msg),
        409 => errors::Error::conflict(msg),
        _ => errors::Error::internal_server_error(msg),
    }
}
