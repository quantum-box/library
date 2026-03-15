#![feature(error_generic_member_access)]
use std::backtrace::Backtrace;

mod display;

#[cfg(feature = "axum-extension")]
pub mod axum;

#[cfg(feature = "graphql")]
pub mod async_graphql;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error)]
pub enum Error {
    #[error("{message}")]
    NotFound {
        message: String,
        #[backtrace]
        backtrace: Backtrace,
    },
    #[error("{message}")]
    InternalServerError {
        message: String,
        #[backtrace]
        backtrace: Backtrace,
    },
    #[error("{message}")]
    BadRequest {
        message: String,
        #[backtrace]
        backtrace: Backtrace,
    },
    #[error("{message}")]
    Unauthorized {
        message: String,
        #[backtrace]
        backtrace: Backtrace,
    },
    #[error("{message}")]
    Forbidden {
        message: String,
        #[backtrace]
        backtrace: Backtrace,
    },
    #[error("{message}")]
    Conflict {
        message: String,
        #[backtrace]
        backtrace: Backtrace,
    },
    #[error("{message}")]
    PaymentRequired {
        message: String,
        #[backtrace]
        backtrace: Backtrace,
    },
    #[error("{message}")]
    ServiceUnavailable {
        message: String,
        #[backtrace]
        backtrace: Backtrace,
    },
}

impl Error {
    fn capture_backtrace() -> Backtrace {
        Backtrace::capture()
    }

    fn format_message(prefix: &str, message: impl ToString) -> String {
        let detail = message.to_string();
        if detail.is_empty() {
            prefix.to_string()
        } else {
            format!("{prefix}: {detail}")
        }
    }

    fn bad_request_raw(message: String) -> Error {
        Error::BadRequest {
            message,
            backtrace: Self::capture_backtrace(),
        }
    }

    pub fn internal_server_error(message: impl ToString) -> Error {
        Error::InternalServerError {
            message: Self::format_message("InternalServerError", message),
            backtrace: Self::capture_backtrace(),
        }
    }

    pub fn bad_request(message: impl ToString) -> Error {
        Self::bad_request_raw(Self::format_message("BadRequest", message))
    }

    pub fn unauthorized(message: impl ToString) -> Error {
        Error::Unauthorized {
            message: Self::format_message("UnauthorizedError", message),
            backtrace: Self::capture_backtrace(),
        }
    }

    pub fn forbidden(message: impl ToString) -> Error {
        Error::Forbidden {
            message: Self::format_message("Forbidden", message),
            backtrace: Self::capture_backtrace(),
        }
    }

    pub fn not_found(message: impl ToString) -> Error {
        Error::NotFound {
            message: Self::format_message("NotFoundError", message),
            backtrace: Self::capture_backtrace(),
        }
    }

    pub fn conflict(message: impl ToString) -> Error {
        Error::Conflict {
            message: Self::format_message("Conflict", message),
            backtrace: Self::capture_backtrace(),
        }
    }

    pub fn payment_required(message: impl ToString) -> Error {
        Error::PaymentRequired {
            message: Self::format_message("PaymentRequired", message),
            backtrace: Self::capture_backtrace(),
        }
    }

    pub fn service_unavailable(message: impl ToString) -> Error {
        Error::ServiceUnavailable {
            message: Self::format_message("ServiceUnavailable", message),
            backtrace: Self::capture_backtrace(),
        }
    }

    pub fn permission_denied(message: impl ToString) -> Error {
        Error::Forbidden {
            message: Self::format_message("PermissionDenied", message),
            backtrace: Self::capture_backtrace(),
        }
    }

    pub fn is_not_found(&self) -> bool {
        matches!(self, Error::NotFound { .. })
    }

    pub fn is_forbidden(&self) -> bool {
        matches!(self, Error::Forbidden { .. })
    }

    pub fn is_bad_request(&self) -> bool {
        matches!(self, Error::BadRequest { .. })
    }

    pub fn business_logic(message: impl ToString) -> Error {
        Self::bad_request_raw(Self::format_message(
            "BusinessLogicError",
            message,
        ))
    }

    pub fn application_logic_error(message: impl ToString) -> Error {
        Self::bad_request_raw(Self::format_message(
            "ApplicationLogicError",
            message,
        ))
    }

    pub fn http_request_error(message: impl ToString) -> Error {
        Self::bad_request_raw(Self::format_message(
            "HttpResponseError",
            message,
        ))
    }

    pub fn type_error(message: impl ToString) -> Error {
        Self::bad_request_raw(Self::format_message("TypeError", message))
    }

    pub fn parse_from_string(message: impl ToString) -> Error {
        Self::bad_request_raw(Self::format_message(
            "ParseFromString",
            message,
        ))
    }

    pub fn parse_error(message: impl ToString) -> Error {
        Self::bad_request_raw(Self::format_message("ParseError", message))
    }

    pub fn unauthenticated(message: impl ToString) -> Error {
        Error::unauthorized(message)
    }

    pub fn other_error(message: impl ToString) -> Error {
        Error::InternalServerError {
            message: Self::format_message("OtherError", message),
            backtrace: Self::capture_backtrace(),
        }
    }

    pub fn provider_error(
        provider_name: impl ToString,
        message: impl ToString,
    ) -> Error {
        let provider_name = provider_name.to_string();
        let message = message.to_string();
        Error::ServiceUnavailable {
            message: Self::format_message(
                "ProviderError",
                format!("{provider_name}: {message}"),
            ),
            backtrace: Self::capture_backtrace(),
        }
    }

    pub fn invalid(message: impl ToString) -> Error {
        Self::bad_request_raw(Self::format_message("Invalid", message))
    }

    pub fn not_supported(message: impl ToString) -> Error {
        Self::bad_request_raw(Self::format_message("NotSupported", message))
    }
}

#[cfg(feature = "sqlx")]
impl From<sqlx::Error> for Error {
    fn from(error: sqlx::Error) -> Self {
        Error::internal_server_error(error)
    }
}

impl From<anyhow::Error> for Error {
    fn from(error: anyhow::Error) -> Self {
        Error::internal_server_error(error)
    }
}

impl From<std::num::ParseIntError> for Error {
    fn from(error: std::num::ParseIntError) -> Self {
        Error::type_error(error)
    }
}

impl From<strum::ParseError> for Error {
    fn from(error: strum::ParseError) -> Self {
        Error::bad_request_raw(Error::format_message(
            "EnumParseError",
            error,
        ))
    }
}

impl From<ParseIdError> for Error {
    fn from(error: ParseIdError) -> Self {
        Error::parse_error(error)
    }
}

impl From<chrono::ParseError> for Error {
    fn from(error: chrono::ParseError) -> Self {
        Error::parse_error(error)
    }
}

impl From<email_address::Error> for Error {
    fn from(error: email_address::Error) -> Self {
        Error::parse_error(error)
    }
}

impl From<url::ParseError> for Error {
    fn from(error: url::ParseError) -> Self {
        Error::parse_error(error)
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Error::parse_error(error)
    }
}

#[derive(Clone, Debug)]
pub struct ParseIdError {
    pub typename: &'static str,
    pub expected: &'static str,
}

impl std::fmt::Display for ParseIdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid `{}`, expected {}", self.typename, self.expected)
    }
}

impl std::error::Error for ParseIdError {
    fn description(&self) -> &str {
        "error parsing an id"
    }
}

pub fn type_error(s: impl ToString) -> Error {
    Error::type_error(s)
}

pub fn parse_from_string(s: impl ToString) -> Error {
    Error::parse_from_string(s)
}

pub fn parse_error(s: impl ToString) -> Error {
    Error::parse_error(s)
}

pub fn internal_server_error(s: impl ToString) -> Error {
    Error::internal_server_error(s)
}

pub fn unauthenticated(s: impl ToString) -> Error {
    Error::unauthenticated(s)
}

pub fn http_request_error(s: impl ToString) -> Error {
    Error::http_request_error(s)
}

pub fn other_error(s: impl ToString) -> Error {
    Error::other_error(s)
}

pub fn provider_error(
    provider_name: impl ToString,
    message: impl ToString,
) -> Error {
    Error::provider_error(provider_name, message)
}

pub fn not_supported(s: impl ToString) -> Error {
    Error::not_supported(s)
}

pub fn payment_required(s: impl ToString) -> Error {
    Error::payment_required(s)
}

#[macro_export]
macro_rules! business_logic {
    ($($arg:tt)*) => {
        $crate::Error::business_logic(format!($($arg)*))
    };
}

#[macro_export]
macro_rules! parse_error {
    ($($arg:tt)*) => {
        $crate::Error::parse_error(format!($($arg)*))
    };
}

#[macro_export]
macro_rules! unauthenticated {
    ($($arg:tt)*) => {
        $crate::Error::unauthenticated(format!($($arg)*))
    };
}

#[macro_export]
macro_rules! not_found {
    ($($arg:tt)*) => {
        $crate::Error::not_found(format!($($arg)*))
    };
}

#[macro_export]
macro_rules! application_logic_error {
    ($($arg:tt)*) => {
        $crate::Error::application_logic_error(format!($($arg)*))
    };
}

#[macro_export]
macro_rules! invalid {
    ($($arg:tt)*) => {
        $crate::Error::invalid(format!($($arg)*))
    };
}

#[macro_export]
macro_rules! permission_denied {
    ($($arg:tt)*) => {
        $crate::Error::permission_denied(format!($($arg)*))
    };
}

#[macro_export]
macro_rules! internal_server_error {
    ($($arg:tt)*) => {
        $crate::Error::internal_server_error(format!($($arg)*))
    };
}

#[macro_export]
macro_rules! bail {
    ($($arg:tt)*) => {
        return Err($crate::Error::business_logic(format!($($arg)*)))
    };
}
