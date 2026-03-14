//! HTTP tracing utilities for axum applications.
//!
//! This module provides reusable layers for HTTP request tracing:
//! - Request ID generation and propagation
//! - Structured logging with request context
//!
//! ## Usage
//!
//! ```rust,ignore
//! use telemetry::http::{
//!     create_request_id_layer,
//!     create_trace_layer,
//!     create_propagate_request_id_layer,
//! };
//!
//! let app = Router::new()
//!     .route("/", get(handler))
//!     .layer(create_propagate_request_id_layer())
//!     .layer(create_trace_layer())
//!     .layer(create_request_id_layer());
//! ```

use axum::{extract::MatchedPath, http::Request};
use http::HeaderName;
use tower_http::{
    request_id::{
        MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer,
    },
    trace::TraceLayer,
};

/// The standard header name for request IDs.
pub const REQUEST_ID_HEADER: &str = "x-request-id";

/// Creates a layer that generates a UUID for each request and sets it
/// as the `x-request-id` header.
///
/// This layer should be applied outermost (last in the chain) so that
/// the request ID is available to all subsequent layers.
pub fn create_request_id_layer() -> SetRequestIdLayer<MakeRequestUuid> {
    let header = HeaderName::from_static(REQUEST_ID_HEADER);
    SetRequestIdLayer::new(header, MakeRequestUuid)
}

/// Creates a layer that copies the `x-request-id` header from the request
/// to the response.
///
/// This allows clients to correlate their requests with server logs.
pub fn create_propagate_request_id_layer() -> PropagateRequestIdLayer {
    let header = HeaderName::from_static(REQUEST_ID_HEADER);
    PropagateRequestIdLayer::new(header)
}

/// Creates a TraceLayer that logs HTTP requests with structured fields.
///
/// Each request creates a span with:
/// - `request_id`: The unique request identifier
/// - `method`: HTTP method (GET, POST, etc.)
/// - `path`: The matched route path or URI path
/// - `version`: HTTP version
///
/// ## Example log output
///
/// ```text
/// INFO http_request{request_id=a1b2c3d4-... method=GET path=/v1/health version=HTTP/1.1}
/// ```
pub fn create_trace_layer<B>() -> TraceLayer<
    tower_http::classify::SharedClassifier<
        tower_http::classify::ServerErrorsAsFailures,
    >,
    impl Fn(&Request<B>) -> tracing::Span + Clone,
> {
    TraceLayer::new_for_http().make_span_with(|request: &Request<B>| {
        // Get request_id from x-request-id header (set by SetRequestIdLayer)
        let request_id = request
            .headers()
            .get(REQUEST_ID_HEADER)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown");

        // Get matched path for better observability
        let matched_path = request
            .extensions()
            .get::<MatchedPath>()
            .map(MatchedPath::as_str)
            .unwrap_or(request.uri().path());

        tracing::info_span!(
            "http_request",
            request_id = %request_id,
            method = %request.method(),
            path = %matched_path,
            version = ?request.version(),
        )
    })
}
