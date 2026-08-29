//! UI URLs for Library documents.
//!
//! Public API responses carry the address a document has in the Library
//! client (`apps/client`) so integrators do not have to reassemble it
//! from path fragments. The origin differs per deployment, so it is read
//! from `LIBRARY_CLIENT_BASE_URL` instead of being compiled in.
//!
//! The client is where the product is heading, so these URLs point there
//! and never at the older web app (`apps/web`, the `/v1beta/...` routes)
//! -- integrators who follow what the API hands them are carried off v1
//! without a second migration.

/// Local Library client dev server (`apps/client`). Every deployed
/// environment sets `LIBRARY_CLIENT_BASE_URL` explicitly; see
/// `tachyon.yaml`.
const DEFAULT_LIBRARY_CLIENT_BASE_URL: &str = "http://localhost:5173";

/// Origin of the Library client UI, without a trailing slash.
pub fn library_client_base_url() -> String {
    let configured = std::env::var("LIBRARY_CLIENT_BASE_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_LIBRARY_CLIENT_BASE_URL.to_string());

    configured.trim().trim_end_matches('/').to_string()
}

/// URL that opens a single document in the Library client.
///
/// Mirrors the client's `/$organization/$repository/data/$recordId`
/// route.
pub fn data_url(org: &str, repo: &str, data_id: &str) -> String {
    format!("{}/{org}/{repo}/data/{data_id}", library_client_base_url())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn with_base_url(value: Option<&str>, test: impl FnOnce()) {
        let _guard = ENV_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("env lock poisoned");
        match value {
            Some(value) => {
                std::env::set_var("LIBRARY_CLIENT_BASE_URL", value)
            }
            None => std::env::remove_var("LIBRARY_CLIENT_BASE_URL"),
        }
        test();
        std::env::remove_var("LIBRARY_CLIENT_BASE_URL");
    }

    #[test]
    fn data_url_follows_the_client_route() {
        with_base_url(Some("https://planetlibrary.example"), || {
            assert_eq!(
                data_url("quantum-box", "corporate", "data_123"),
                "https://planetlibrary.example/quantum-box/corporate/data/data_123"
            );
        });
    }

    #[test]
    fn data_url_drops_a_trailing_slash_from_the_configured_base() {
        with_base_url(Some("https://planetlibrary.example/"), || {
            assert_eq!(
                data_url("org", "repo", "data_1"),
                "https://planetlibrary.example/org/repo/data/data_1"
            );
        });
    }

    #[test]
    fn data_url_falls_back_to_the_dev_server_when_unset() {
        with_base_url(None, || {
            assert_eq!(
                data_url("org", "repo", "data_1"),
                "http://localhost:5173/org/repo/data/data_1"
            );
        });
    }

    #[test]
    fn data_url_ignores_a_blank_configured_base() {
        with_base_url(Some("   "), || {
            assert_eq!(
                data_url("org", "repo", "data_1"),
                "http://localhost:5173/org/repo/data/data_1"
            );
        });
    }
}
