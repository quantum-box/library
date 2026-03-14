use crate::Error;
use async_graphql::ErrorExtensions;

impl ErrorExtensions for Error {
    fn extend(&self) -> async_graphql::Error {
        // Log at appropriate level based on error type
        log_graphql_error(self);
        async_graphql::Error::new(format!("{self}")).extend_with(|_, e| {
            match self {
                Error::BadRequest { message, .. } => {
                    e.set("code", "BAD_REQUEST");
                    e.set("message", message)
                }
                Error::InternalServerError { message, .. } => {
                    e.set("code", "INTERNAL_SERVER_ERROR");
                    e.set("message", message)
                }
                Error::Unauthorized { message, .. } => {
                    e.set("code", "UNAUTHORIZED");
                    e.set("message", message)
                }
                Error::Forbidden { message, .. } => {
                    e.set("code", "FORBIDDEN");
                    e.set("message", message)
                }
                Error::NotFound { message, .. } => {
                    e.set("code", "NOT_FOUND");
                    e.set("message", message)
                }
                Error::Conflict { message, .. } => {
                    e.set("code", "CONFLICT");
                    e.set("message", message)
                }
                Error::PaymentRequired { message, .. } => {
                    e.set("code", "PAYMENT_REQUIRED");
                    e.set("message", message)
                }
                Error::ServiceUnavailable { message, .. } => {
                    e.set("code", "SERVICE_UNAVAILABLE");
                    e.set("message", message)
                }
            }
        })
    }
}

/// Log GraphQL errors at appropriate level based on error type
/// - 5xx errors (server issues): ERROR level
/// - 4xx errors (client issues): WARN level
fn log_graphql_error(error: &Error) {
    match error {
        // Server errors (5xx) - ERROR level
        Error::InternalServerError { message, .. } => {
            tracing::error!(
                error_code = "INTERNAL_SERVER_ERROR",
                message = %message,
                error_type = "server",
                "graphql server error"
            );
        }
        Error::ServiceUnavailable { message, .. } => {
            tracing::error!(
                error_code = "SERVICE_UNAVAILABLE",
                message = %message,
                error_type = "server",
                "graphql server error"
            );
        }
        // Client errors (4xx) - WARN level
        Error::BadRequest { message, .. } => {
            tracing::warn!(
                error_code = "BAD_REQUEST",
                message = %message,
                error_type = "client",
                "graphql client error"
            );
        }
        Error::Unauthorized { message, .. } => {
            tracing::warn!(
                error_code = "UNAUTHORIZED",
                message = %message,
                error_type = "client",
                "graphql client error"
            );
        }
        Error::Forbidden { message, .. } => {
            tracing::warn!(
                error_code = "FORBIDDEN",
                message = %message,
                error_type = "client",
                "graphql client error"
            );
        }
        Error::NotFound { message, .. } => {
            tracing::warn!(
                error_code = "NOT_FOUND",
                message = %message,
                error_type = "client",
                "graphql client error"
            );
        }
        Error::Conflict { message, .. } => {
            tracing::warn!(
                error_code = "CONFLICT",
                message = %message,
                error_type = "client",
                "graphql client error"
            );
        }
        Error::PaymentRequired { message, .. } => {
            tracing::warn!(
                error_code = "PAYMENT_REQUIRED",
                message = %message,
                error_type = "client",
                "graphql client error"
            );
        }
    }
}
