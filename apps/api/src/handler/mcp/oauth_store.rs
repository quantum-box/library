//! Shared OAuth state. No process-local fallback: every Lambda uses the
//! Library pool, including the isolated database selected for previews.
use std::sync::Arc;

use openssl::symm::{decrypt_aead, encrypt_aead, Cipher};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{MySqlPool, Row};

#[derive(Clone)]
pub struct McpOAuthStore {
    pool: Arc<MySqlPool>,
}

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct McpOAuthClient {
    pub redirect_uris: Vec<String>,
    pub token_endpoint_auth_method: String,
    pub grant_types: Vec<String>,
    pub response_types: Vec<String>,
}

// Deliberately no Debug: this payload contains a bearer token.
#[derive(Serialize, Deserialize)]
pub(super) struct McpOAuthCode {
    pub client_id: String,
    pub redirect_uri: String,
    pub code_challenge: String,
    pub scope: Option<String>,
    pub access_token: String,
    pub expires_in: i64,
}

impl McpOAuthStore {
    pub fn new(pool: Arc<MySqlPool>) -> Self {
        Self { pool }
    }

    pub(super) async fn register(
        &self,
        client_id: &str,
        client: &McpOAuthClient,
    ) -> anyhow::Result<()> {
        sqlx::query("INSERT INTO mcp_oauth_clients (client_id, metadata) VALUES (?, ?)")
            .bind(client_id)
            .bind(sqlx::types::Json(client))
            .execute(self.pool.as_ref())
            .await?;
        Ok(())
    }

    pub(super) async fn client(
        &self,
        client_id: &str,
    ) -> anyhow::Result<Option<McpOAuthClient>> {
        let row: Option<(sqlx::types::Json<McpOAuthClient>,)> =
            sqlx::query_as("SELECT metadata FROM mcp_oauth_clients WHERE client_id = ?")
                .bind(client_id)
                .fetch_optional(self.pool.as_ref())
                .await?;
        Ok(row.map(|(client,)| client.0))
    }

    pub(super) async fn issue(
        &self,
        code: &str,
        payload: &McpOAuthCode,
    ) -> anyhow::Result<()> {
        let encrypted = seal(code, payload)?;
        // Bound cleanup work per successful sign-in. Expiry checks below
        // remain authoritative even when no further sign-ins take place.
        sqlx::query("DELETE FROM mcp_oauth_codes WHERE expires_at <= UTC_TIMESTAMP(6) LIMIT 100")
            .execute(self.pool.as_ref()).await?;
        sqlx::query("INSERT INTO mcp_oauth_codes (code_hash, payload, expires_at, token_expires_at) VALUES (?, ?, TIMESTAMPADD(SECOND, 600, UTC_TIMESTAMP(6)), TIMESTAMPADD(SECOND, ?, UTC_TIMESTAMP(6)))")
            .bind(code_hash(code))
            .bind(encrypted)
            .bind(payload.expires_in)
            .execute(self.pool.as_ref()).await?;
        Ok(())
    }

    pub(super) async fn redeem(
        &self,
        code: &str,
        client_id: &str,
        redirect_uri: &str,
        verifier: &str,
    ) -> anyhow::Result<Option<McpOAuthCode>> {
        let started = std::time::Instant::now();
        let hash = code_hash(code);
        let row = sqlx::query("SELECT payload, TIMESTAMPDIFF(SECOND, UTC_TIMESTAMP(6), token_expires_at) AS remaining FROM mcp_oauth_codes WHERE code_hash = ? AND expires_at > UTC_TIMESTAMP(6) AND token_expires_at > UTC_TIMESTAMP(6)")
            .bind(&hash).fetch_optional(self.pool.as_ref()).await?;
        let Some(row) = row else { return Ok(None) };
        let mut payload =
            unseal(code, &row.try_get::<Vec<u8>, _>("payload")?)?;
        if payload.client_id != client_id
            || payload.redirect_uri != redirect_uri
            || !super::verify_pkce(verifier, &payload.code_challenge)
        {
            return Ok(None);
        }
        payload.expires_in = row.try_get("remaining")?;
        if payload.expires_in <= 0 {
            return Ok(None);
        }
        // A conditional DELETE is the atomic claim. Concurrent Lambda
        // instances may read the same row, but only one can release it.
        let consumed = sqlx::query("DELETE FROM mcp_oauth_codes WHERE code_hash = ? AND expires_at > UTC_TIMESTAMP(6) AND token_expires_at > UTC_TIMESTAMP(6)")
            .bind(hash).execute(self.pool.as_ref()).await?.rows_affected();
        // Include time spent waiting for the DB claim in the reported TTL.
        payload.expires_in -= started.elapsed().as_secs() as i64;
        Ok((consumed == 1 && payload.expires_in > 0).then_some(payload))
    }
}

fn code_hash(code: &str) -> Vec<u8> {
    Sha256::digest(code.as_bytes()).to_vec()
}

fn encryption_key(code: &str) -> [u8; 32] {
    // Domain separation keeps the encryption key distinct from the stored
    // lookup hash. The random authorization code is never persisted.
    Sha256::digest(
        format!("library-mcp-oauth-payload-v1:{code}").as_bytes(),
    )
    .into()
}

fn seal(code: &str, payload: &McpOAuthCode) -> anyhow::Result<Vec<u8>> {
    let mut nonce = [0u8; 12];
    openssl::rand::rand_bytes(&mut nonce)?;
    let mut tag = [0u8; 16];
    let ciphertext = encrypt_aead(
        Cipher::aes_256_gcm(),
        &encryption_key(code),
        Some(&nonce),
        b"library-mcp-oauth-v1",
        &serde_json::to_vec(payload)?,
        &mut tag,
    )?;
    Ok([nonce.as_slice(), tag.as_slice(), ciphertext.as_slice()].concat())
}

fn unseal(code: &str, encrypted: &[u8]) -> anyhow::Result<McpOAuthCode> {
    anyhow::ensure!(encrypted.len() >= 28, "Invalid OAuth payload");
    let plaintext = decrypt_aead(
        Cipher::aes_256_gcm(),
        &encryption_key(code),
        Some(&encrypted[..12]),
        b"library-mcp-oauth-v1",
        &encrypted[28..],
        &encrypted[12..28],
    )?;
    Ok(serde_json::from_slice(&plaintext)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};

    const VERIFIER: &str =
        "test-verifier-with-at-least-forty-three-characters";

    fn payload() -> McpOAuthCode {
        McpOAuthCode {
            client_id: "test-client".into(),
            redirect_uri: "https://example.test/callback".into(),
            code_challenge: URL_SAFE_NO_PAD
                .encode(Sha256::digest(VERIFIER)),
            scope: Some("openid".into()),
            access_token: "fictional-access-token".into(),
            expires_in: 3600,
        }
    }

    #[test]
    fn payload_requires_the_original_code_and_rejects_tampering() {
        let sealed = seal("random-code", &payload()).unwrap();
        assert!(!sealed
            .windows(22)
            .any(|w| w == b"fictional-access-token"));
        assert_eq!(
            unseal("random-code", &sealed).unwrap().access_token,
            "fictional-access-token"
        );
        assert!(unseal("wrong-code", &sealed).is_err());
        let mut corrupt = sealed.clone();
        corrupt[28] ^= 1;
        assert!(unseal("random-code", &corrupt).is_err());
        assert!(unseal("random-code", &sealed[..12]).is_err());
        assert_ne!(
            encryption_key("random-code").as_slice(),
            code_hash("random-code")
        );
    }

    #[sqlx::test(migrations = false)]
    #[ignore = "requires a disposable MySQL server via DATABASE_URL"]
    async fn shared_oauth_state(pool: MySqlPool) {
        sqlx::raw_sql(include_str!("../../../migrations/20260907000000_create_mcp_oauth_state.up.sql"))
            .execute(&pool).await.unwrap();
        let second_pool = sqlx::mysql::MySqlPoolOptions::new()
            .connect_with((*pool.connect_options()).clone())
            .await
            .unwrap();
        let first = McpOAuthStore::new(Arc::new(pool.clone()));
        let second = McpOAuthStore::new(Arc::new(second_pool));
        let client = McpOAuthClient {
            redirect_uris: vec![payload().redirect_uri],
            token_endpoint_auth_method: "none".into(),
            grant_types: vec!["authorization_code".into()],
            response_types: vec!["code".into()],
        };
        first.register("test-client", &client).await.unwrap();
        assert_eq!(
            second
                .client("test-client")
                .await
                .unwrap()
                .unwrap()
                .redirect_uris,
            client.redirect_uris
        );
        assert!(second.client("TEST-CLIENT").await.unwrap().is_none());
        assert!(second.client("missing-client").await.unwrap().is_none());
        first.issue("code-one", &payload()).await.unwrap();
        drop(first); // Registration and code survive the issuing instance.
        for (id, redirect, verifier) in [
            ("wrong-client", "https://example.test/callback", VERIFIER),
            ("test-client", "https://wrong.test/callback", VERIFIER),
            (
                "test-client",
                "https://example.test/callback",
                "wrong-verifier",
            ),
        ] {
            assert!(second
                .redeem("code-one", id, redirect, verifier)
                .await
                .unwrap()
                .is_none());
        }
        let redeemed = second
            .redeem(
                "code-one",
                "test-client",
                "https://example.test/callback",
                VERIFIER,
            )
            .await
            .unwrap()
            .unwrap();
        assert_eq!(redeemed.access_token, "fictional-access-token");
        assert!(redeemed.expires_in > 0 && redeemed.expires_in <= 3600);
        assert!(second
            .redeem(
                "code-one",
                "test-client",
                "https://example.test/callback",
                VERIFIER
            )
            .await
            .unwrap()
            .is_none());

        second.issue("race-code", &payload()).await.unwrap();
        let third = McpOAuthStore::new(Arc::new(pool.clone()));
        let (a, b) = tokio::join!(
            second.redeem(
                "race-code",
                "test-client",
                "https://example.test/callback",
                VERIFIER
            ),
            third.redeem(
                "race-code",
                "test-client",
                "https://example.test/callback",
                VERIFIER
            ),
        );
        assert_eq!(
            usize::from(a.unwrap().is_some())
                + usize::from(b.unwrap().is_some()),
            1
        );

        second.issue("expired-code", &payload()).await.unwrap();
        sqlx::query("UPDATE mcp_oauth_codes SET expires_at = UTC_TIMESTAMP(6) WHERE code_hash = ?")
            .bind(code_hash("expired-code")).execute(&pool).await.unwrap();
        assert!(second
            .redeem(
                "expired-code",
                "test-client",
                "https://example.test/callback",
                VERIFIER
            )
            .await
            .unwrap()
            .is_none());
        let mut expired_token = payload();
        expired_token.expires_in = -1;
        second.issue("expired-token", &expired_token).await.unwrap();
        assert!(second
            .redeem(
                "expired-token",
                "test-client",
                "https://example.test/callback",
                VERIFIER
            )
            .await
            .unwrap()
            .is_none());
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM mcp_oauth_codes WHERE code_hash = ?",
        )
        .bind(code_hash("expired-code"))
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count, 0, "new issuance cleans expired codes");

        // Exercise the handlers with independent store instances too.
        use super::super::{
            mcp_oauth_authorize, mcp_oauth_register, mcp_oauth_token,
        };
        use axum::{
            extract::{Extension, Form, Query},
            http::StatusCode,
        };
        let response = mcp_oauth_register(
            Extension(second.clone()),
            axum::Json(
                serde_json::from_value(serde_json::json!({
                    "redirect_uris": ["https://example.test/callback"]
                }))
                .unwrap(),
            ),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 8192)
            .await
            .unwrap();
        let registration: serde_json::Value =
            serde_json::from_slice(&body).unwrap();
        assert_eq!(
            registration["grant_types"],
            serde_json::json!(["authorization_code"])
        );
        let client_id = registration["client_id"].as_str().unwrap();
        let authorize = serde_json::json!({
            "response_type": "code", "client_id": client_id,
            "redirect_uri": "https://example.test/callback",
            "code_challenge": payload().code_challenge,
            "code_challenge_method": "S256"
        });
        let authorize = serde_json::from_value(authorize).unwrap();
        assert!(super::super::validate_authorize_request(
            &third, &authorize
        )
        .await
        .is_ok());
        let response =
            mcp_oauth_authorize(Extension(third.clone()), Query(authorize))
                .await;
        let body = axum::body::to_bytes(response.into_body(), 65536)
            .await
            .unwrap();
        assert!(!String::from_utf8(body.to_vec())
            .unwrap()
            .contains("not registered"));
        let mut bound = payload();
        bound.client_id = client_id.into();
        second.issue("handler-code", &bound).await.unwrap();
        sqlx::query("UPDATE mcp_oauth_codes SET token_expires_at = TIMESTAMPADD(SECOND, 120, UTC_TIMESTAMP(6)) WHERE code_hash = ?")
            .bind(code_hash("handler-code")).execute(&pool).await.unwrap();
        let token_request = serde_json::json!({
            "grant_type": "authorization_code", "code": "handler-code",
            "client_id": client_id, "redirect_uri": bound.redirect_uri,
            "code_verifier": VERIFIER
        });
        for field in ["client_id", "redirect_uri", "code_verifier"] {
            let mut missing = token_request.clone();
            missing.as_object_mut().unwrap().remove(field);
            let response = mcp_oauth_token(
                Extension(third.clone()),
                Form(serde_json::from_value(missing).unwrap()),
            )
            .await;
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        }
        let response = mcp_oauth_token(
            Extension(third.clone()),
            Form(serde_json::from_value(token_request.clone()).unwrap()),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 8192)
            .await
            .unwrap();
        let token: serde_json::Value =
            serde_json::from_slice(&body).unwrap();
        assert_eq!(token["access_token"], "fictional-access-token");
        assert!(token["expires_in"].as_i64().unwrap() <= 120);
        let response = mcp_oauth_token(
            Extension(second),
            Form(serde_json::from_value(token_request).unwrap()),
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        // Missing tables must fail closed rather than create process-local state.
        sqlx::raw_sql(include_str!("../../../migrations/20260907000000_create_mcp_oauth_state.down.sql"))
            .execute(&pool).await.unwrap();
        let response = mcp_oauth_register(
            Extension(third),
            axum::Json(
                serde_json::from_value(serde_json::json!({
                    "redirect_uris": ["https://example.test/callback"]
                }))
                .unwrap(),
            ),
        )
        .await;
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }
}
