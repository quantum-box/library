//! Verification of access tokens issued by the configured Tachyon server.
use std::time::{Duration, Instant};

use jsonwebtoken::{
    decode, decode_header, jwk::JwkSet, Algorithm, DecodingKey, Validation,
};
use once_cell::sync::Lazy;
use serde::Deserialize;
use tokio::sync::Mutex;

const KEY_TTL: Duration = Duration::from_secs(300);
static KEYS: Lazy<Mutex<Option<CachedKeys>>> =
    Lazy::new(|| Mutex::new(None));

struct CachedKeys {
    url: String,
    fetched_at: Instant,
    keys: JwkSet,
}

#[derive(Clone, Debug)]
pub(super) struct Config {
    pub issuer: String,
    pub jwks_url: String,
    pub resource: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Scopes {
    pub read: bool,
    pub write: bool,
}

impl Scopes {
    pub fn allows(self, read_only: bool) -> bool {
        if read_only {
            self.read
        } else {
            self.write
        }
    }
}

#[derive(Debug, Deserialize)]
struct Claims {
    sub: String,
    scope: String,
}

#[derive(Debug)]
pub(super) struct VerifiedToken {
    pub subject: String,
    pub scopes: Scopes,
}

/// Presence, including an empty value, selects strict external verification.
/// A malformed configuration must never fall back to the legacy flow.
pub(super) fn enabled() -> bool {
    std::env::var_os("MCP_AUTHORIZATION_SERVERS").is_some()
        || std::env::var_os("MCP_AUTHORIZATION_SERVER").is_some()
}

impl Config {
    pub fn from_env() -> Result<Self, &'static str> {
        let issuer = std::env::var("MCP_AUTHORIZATION_SERVERS")
            .or_else(|_| std::env::var("MCP_AUTHORIZATION_SERVER"))
            .map_err(|_| "Missing MCP authorization server")?;
        let config = Self {
            issuer: issuer.trim().to_string(),
            jwks_url: std::env::var("MCP_OAUTH_JWKS_URL")
                .map_err(|_| "Missing MCP OAuth JWKS URL")?,
            resource: super::mcp_resource_url(),
        };
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<(), &'static str> {
        for value in [&self.issuer, &self.jwks_url, &self.resource] {
            let url =
                url::Url::parse(value).map_err(|_| "Invalid OAuth URL")?;
            if url.scheme() != "https"
                || url.host_str().is_none()
                || !url.username().is_empty()
                || url.password().is_some()
                || url.fragment().is_some()
                || value.contains(',')
            {
                return Err("OAuth URLs must be single HTTPS URLs without credentials or fragments");
            }
        }
        Ok(())
    }

    pub async fn verify(
        &self,
        token: &str,
    ) -> Result<VerifiedToken, &'static str> {
        // Refuse invalid algorithms before any network request. Never use jku/x5u.
        let header =
            decode_header(token).map_err(|_| "Invalid token header")?;
        if header.alg != Algorithm::RS256 || header.kid.is_none() {
            return Err("Expected an RS256 token with kid");
        }
        // Serialize refreshes to avoid a burst of identical JWKS requests.
        let mut cache = KEYS.lock().await;
        if !cache.as_ref().is_some_and(|entry| {
            entry.url == self.jwks_url
                && entry.fetched_at.elapsed() < KEY_TTL
        }) {
            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(5))
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .map_err(|_| "Cannot construct JWKS client")?;
            let response = client
                .get(&self.jwks_url)
                .send()
                .await
                .map_err(|_| "JWKS request failed")?
                .error_for_status()
                .map_err(|_| "JWKS endpoint failed")?;
            let keys = response
                .json::<JwkSet>()
                .await
                .map_err(|_| "Invalid JWKS")?;
            *cache = Some(CachedKeys {
                url: self.jwks_url.clone(),
                fetched_at: Instant::now(),
                keys,
            });
        }
        // An unknown kid fails closed until the short cache expires. This
        // bounds fetches for attacker-generated kids; publish rotated keys early.
        verify_with_keys(
            token,
            self,
            &cache.as_ref().ok_or("Missing JWKS")?.keys,
        )
    }
}

fn verify_with_keys(
    token: &str,
    config: &Config,
    keys: &JwkSet,
) -> Result<VerifiedToken, &'static str> {
    let header =
        decode_header(token).map_err(|_| "Invalid token header")?;
    if header.alg != Algorithm::RS256 {
        return Err("Invalid token algorithm");
    }
    let kid = header.kid.ok_or("Missing token kid")?;
    let key = keys.find(&kid).ok_or("Unknown token kid")?;
    // Restrict keys to RSA signing keys, even if the JWKS contains encryption keys.
    let value = serde_json::to_value(key).map_err(|_| "Invalid JWK")?;
    if value["kty"] != "RSA"
        || value.get("alg").is_some_and(|alg| alg != "RS256")
        || value.get("use").is_some_and(|usage| usage != "sig")
        || value.get("key_ops").is_some_and(|ops| {
            !ops.as_array()
                .is_some_and(|ops| ops.iter().any(|op| op == "verify"))
        })
    {
        return Err("Invalid signing key");
    }
    let key = DecodingKey::from_jwk(key).map_err(|_| "Invalid RSA key")?;
    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_issuer(&[&config.issuer]);
    validation.set_audience(&[&config.resource]);
    validation.set_required_spec_claims(&["exp", "iss", "aud", "sub"]);
    validation.validate_nbf = true;
    validation.leeway = 0;
    let claims = decode::<Claims>(token, &key, &validation)
        .map_err(|_| "Invalid OAuth access token")?
        .claims;
    if claims.sub.is_empty() {
        return Err("Empty token subject");
    }
    let scopes = claims.scope.split_whitespace().collect::<Vec<_>>();
    Ok(VerifiedToken {
        subject: claims.sub,
        scopes: Scopes {
            read: scopes.contains(&"mcp:read"),
            write: scopes.contains(&"mcp:write"),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde_json::{json, Value};

    struct Fixture {
        encoding_key: EncodingKey,
        keys: JwkSet,
        config: Config,
    }

    impl Fixture {
        fn new() -> Self {
            let rsa = openssl::rsa::Rsa::generate(2048).unwrap();
            let encoding_key = EncodingKey::from_rsa_pem(
                &rsa.private_key_to_pem().unwrap(),
            )
            .unwrap();
            let keys = serde_json::from_value(json!({"keys": [{
                "kty": "RSA", "use": "sig", "alg": "RS256", "kid": "test-key",
                "n": URL_SAFE_NO_PAD.encode(rsa.n().to_vec()),
                "e": URL_SAFE_NO_PAD.encode(rsa.e().to_vec())
            }]})).unwrap();
            Self {
                encoding_key,
                keys,
                config: Config {
                    issuer: "https://issuer.example.test".into(),
                    jwks_url: "https://issuer.example.test/oauth2/jwks"
                        .into(),
                    resource: "https://library.example.test/mcp".into(),
                },
            }
        }

        fn claims(&self) -> Value {
            json!({"sub":"us_test", "iss":self.config.issuer,
                "aud":self.config.resource, "exp":chrono::Utc::now().timestamp() + 600,
                "scope":"openid mcp:read"})
        }

        fn token(&self, claims: &Value) -> String {
            let mut header = Header::new(Algorithm::RS256);
            header.kid = Some("test-key".into());
            encode(&header, claims, &self.encoding_key).unwrap()
        }
    }

    #[test]
    fn validates_signature_and_resource_bound_access_claims() {
        let fixture = Fixture::new();
        let token = fixture.token(&fixture.claims());
        let verified =
            verify_with_keys(&token, &fixture.config, &fixture.keys)
                .unwrap();
        assert_eq!(verified.subject, "us_test");
        assert_eq!(
            verified.scopes,
            Scopes {
                read: true,
                write: false
            }
        );
        let mut claims = fixture.claims();
        claims["aud"] = json!([
            "https://another.example.test/mcp",
            fixture.config.resource
        ]);
        claims["scope"] = json!("mcp:write");
        let verified = verify_with_keys(
            &fixture.token(&claims),
            &fixture.config,
            &fixture.keys,
        )
        .unwrap();
        assert_eq!(
            verified.scopes,
            Scopes {
                read: false,
                write: true
            }
        );
    }

    #[test]
    fn rejects_wrong_issuer_audience_expiry_nbf_and_missing_claims() {
        let fixture = Fixture::new();
        for (claim, value) in [
            ("iss", json!("https://attacker.example.test")),
            ("aud", json!("oauth_client_id")),
            ("aud", json!("https://library.example.test/mcp/")),
            ("exp", json!(chrono::Utc::now().timestamp() - 1)),
            ("nbf", json!(chrono::Utc::now().timestamp() + 600)),
            ("sub", json!("")),
        ] {
            let mut claims = fixture.claims();
            claims[claim] = value;
            assert!(
                verify_with_keys(
                    &fixture.token(&claims),
                    &fixture.config,
                    &fixture.keys
                )
                .is_err(),
                "{claim}"
            );
        }
        for claim in ["iss", "aud", "sub", "exp", "scope"] {
            let mut claims = fixture.claims();
            claims.as_object_mut().unwrap().remove(claim);
            assert!(
                verify_with_keys(
                    &fixture.token(&claims),
                    &fixture.config,
                    &fixture.keys
                )
                .is_err(),
                "missing {claim}"
            );
        }
    }

    #[test]
    fn rejects_forged_signature_algorithms_and_wrong_signing_keys() {
        let fixture = Fixture::new();
        let other = Fixture::new();
        assert!(verify_with_keys(
            &other.token(&fixture.claims()),
            &fixture.config,
            &fixture.keys
        )
        .is_err());
        let token = fixture.token(&fixture.claims());
        let mut parts =
            token.split('.').map(str::to_owned).collect::<Vec<_>>();
        let mut claims = fixture.claims();
        claims["scope"] = json!("mcp:read mcp:write");
        parts[1] =
            URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims).unwrap());
        assert!(verify_with_keys(
            &parts.join("."),
            &fixture.config,
            &fixture.keys
        )
        .is_err());
        let mut header = Header::new(Algorithm::HS256);
        header.kid = Some("test-key".into());
        let hs_token = encode(
            &header,
            &claims,
            &EncodingKey::from_secret(b"attacker"),
        )
        .unwrap();
        assert!(verify_with_keys(
            &hs_token,
            &fixture.config,
            &fixture.keys
        )
        .is_err());
        for (field, value) in
            [("kid", "unknown"), ("alg", "RS512"), ("use", "enc")]
        {
            let mut keys = serde_json::to_value(&fixture.keys).unwrap();
            keys["keys"][0][field] = json!(value);
            let keys = serde_json::from_value(keys).unwrap();
            assert!(
                verify_with_keys(&token, &fixture.config, &keys).is_err()
            );
        }
    }

    #[test]
    fn scopes_require_exact_names_and_do_not_imply_each_other() {
        let fixture = Fixture::new();
        let mut claims = fixture.claims();
        claims["scope"] = json!("openid profile mcp:reader mcp:write-all");
        let verified = verify_with_keys(
            &fixture.token(&claims),
            &fixture.config,
            &fixture.keys,
        )
        .unwrap();
        assert!(!verified.scopes.allows(true));
        assert!(!verified.scopes.allows(false));
    }

    #[test]
    fn configuration_rejects_unsafe_and_multiple_urls() {
        let fixture = Fixture::new();
        assert!(fixture.config.validate().is_ok());
        for value in [
            "",
            "http://issuer.test",
            "https://issuer.test,https://other.test",
            "https://user:pass@issuer.test",
            "https://issuer.test/#fragment",
        ] {
            let mut config = fixture.config.clone();
            config.issuer = value.into();
            assert!(config.validate().is_err(), "{value}");
            config = fixture.config.clone();
            config.jwks_url = value.into();
            assert!(config.validate().is_err(), "{value}");
            config = fixture.config.clone();
            config.resource = value.into();
            assert!(config.validate().is_err(), "{value}");
        }
    }
}
