use std::sync::Arc;

use axum::{
    extract::State,
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    routing::post,
    Json, Router,
};
use chrono::{DateTime, Utc};

use crate::usecase::{ExternalSyncOutboxDispatch, ExternalSyncScanSummary};

const ENGINE_ENABLED_ENV: &str = "LIBRARY_EXTERNAL_SYNC_ENGINE_ENABLED";
const SCANNER_TOKEN_ENV: &str = "EXTERNAL_SYNC_SCANNER_TOKEN";
const SCAN_AFTER_ENV: &str = "LIBRARY_EXTERNAL_SYNC_OUTBOX_SCAN_AFTER";

#[derive(Clone)]
struct ScannerState {
    scanner: Arc<ExternalSyncOutboxDispatch>,
    token: Arc<str>,
    scan_after: DateTime<Utc>,
}

pub fn create_router(
    scanner: Arc<ExternalSyncOutboxDispatch>,
) -> errors::Result<Router> {
    let enabled = std::env::var(ENGINE_ENABLED_ENV)
        .is_ok_and(|value| value.trim().eq_ignore_ascii_case("true"));
    if !enabled {
        return Ok(Router::new());
    }
    let token = std::env::var(SCANNER_TOKEN_ENV).map_err(|_| {
        errors::Error::service_unavailable(format!(
            "{SCANNER_TOKEN_ENV} is required when {ENGINE_ENABLED_ENV}=true"
        ))
    })?;
    if token.len() < 32 {
        return Err(errors::Error::service_unavailable(format!(
            "{SCANNER_TOKEN_ENV} must contain at least 32 bytes"
        )));
    }
    let scan_after = std::env::var(SCAN_AFTER_ENV)
        .map_err(|_| {
            errors::Error::service_unavailable(format!(
                "{SCAN_AFTER_ENV} is required when {ENGINE_ENABLED_ENV}=true"
            ))
        })?
        .parse::<DateTime<Utc>>()
        .map_err(|error| {
            errors::Error::service_unavailable(format!(
                "{SCAN_AFTER_ENV} must be RFC3339: {error}"
            ))
        })?;
    Ok(Router::new()
        .route("/internal/external-sync/outbound-scan", post(scan_outbound))
        .with_state(ScannerState {
            scanner,
            token: token.into(),
            scan_after,
        }))
}

async fn scan_outbound(
    State(state): State<ScannerState>,
    headers: HeaderMap,
) -> Result<Json<ExternalSyncScanSummary>, StatusCode> {
    let supplied = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .unwrap_or_default();
    if !constant_time_eq(state.token.as_bytes(), supplied.as_bytes()) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    state
        .scanner
        .scan_due(state.scan_after, 25, 3)
        .await
        .map(Json)
        .map_err(|error| {
            tracing::error!(%error, "external sync outbox scan failed");
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

fn constant_time_eq(expected: &[u8], supplied: &[u8]) -> bool {
    let mut difference = expected.len() ^ supplied.len();
    for index in 0..expected.len().max(supplied.len()) {
        difference |= usize::from(
            expected.get(index).copied().unwrap_or_default()
                ^ supplied.get(index).copied().unwrap_or_default(),
        );
    }
    difference == 0
}

#[cfg(test)]
mod tests {
    use super::constant_time_eq;

    #[test]
    fn scanner_token_comparison_requires_exact_bytes() {
        assert!(constant_time_eq(b"scanner-secret", b"scanner-secret"));
        assert!(!constant_time_eq(b"scanner-secret", b"scanner-secreu"));
        assert!(!constant_time_eq(
            b"scanner-secret",
            b"scanner-secret-long"
        ));
        assert!(!constant_time_eq(b"scanner-secret", b""));
    }
}
