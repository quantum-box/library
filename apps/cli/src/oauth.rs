//! Browser sign-in for `library auth login`.
//!
//! The Library API names its authorization server in
//! `/.well-known/oauth-protected-resource`, so the CLI discovers it rather
//! than hard-coding one. The flow is the usual one for a native app
//! (RFC 8252): register a public client (RFC 7591), open the browser on
//! the authorize endpoint with PKCE (RFC 7636), receive the code on a
//! loopback port, and exchange it. The MCP-resource token is sent as a
//! Bearer token only to /mcp; the refresh token keeps the session alive
//! without another browser trip.

use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// Scopes asked for at sign-in. The resulting token is issued for the
/// MCP resource and is only sent to the MCP endpoint.
const SCOPES: &str = "openid profile email mcp:read mcp:write";
/// Give up waiting for the browser after this long.
const LOGIN_TIMEOUT: Duration = Duration::from_secs(300);
/// Refresh this long before the access token actually expires, so a
/// an MCP request never starts with a token that dies mid-request.
const REFRESH_MARGIN_SECS: i64 = 60;

/// A signed-in session, saved in the profile next to the API URL.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OAuthSession {
    pub issuer: String,
    pub token_endpoint: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revocation_endpoint: Option<String>,
    pub client_id: String,
    /// RFC 8707 resource the tokens are issued for.
    pub resource: String,
    pub access_token: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    /// Unix seconds. Absent when the server did not say.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,
}

impl OAuthSession {
    pub fn needs_refresh(&self, now: i64) -> bool {
        self.expires_at.is_some_and(|expires_at| {
            now + REFRESH_MARGIN_SECS >= expires_at
        })
    }
}

#[derive(Debug, Deserialize)]
struct ProtectedResourceMetadata {
    resource: String,
    #[serde(default)]
    authorization_servers: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct AuthorizationServerMetadata {
    issuer: String,
    authorization_endpoint: String,
    token_endpoint: String,
    registration_endpoint: Option<String>,
    revocation_endpoint: Option<String>,
    #[serde(default)]
    code_challenge_methods_supported: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RegistrationResponse {
    client_id: String,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
}

pub fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn random_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// RFC 7636 S256 challenge for a verifier.
fn code_challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

fn http() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent(concat!("library-cli/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(30))
        .build()
        .context("failed to build the HTTP client")
}

async fn get_json<T: for<'de> Deserialize<'de>>(
    http: &reqwest::Client,
    url: &str,
) -> Result<T> {
    let response = http
        .get(url)
        .send()
        .await
        .with_context(|| format!("GET {url} failed"))?;
    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    if !status.is_success() {
        bail!("GET {url} returned {status}: {text}");
    }
    serde_json::from_str(&text)
        .with_context(|| format!("GET {url} returned unexpected JSON"))
}

/// RFC 8414 puts an issuer path after the well-known metadata prefix.
fn authorization_server_metadata_url(issuer: &str) -> Result<String> {
    let issuer_url = url::Url::parse(issuer)
        .context("authorization server issuer is not a valid URL")?;
    if !matches!(issuer_url.scheme(), "http" | "https")
        || issuer_url.host_str().is_none()
        || issuer_url.query().is_some()
        || issuer_url.fragment().is_some()
    {
        bail!("authorization server issuer must be an HTTP(S) URL without query or fragment");
    }
    let origin = issuer_url.origin().ascii_serialization();
    let path = issuer_url.path().trim_end_matches('/');
    Ok(format!(
        "{origin}/.well-known/oauth-authorization-server{path}"
    ))
}

/// Find the authorization server the API trusts.
async fn discover(
    http: &reqwest::Client,
    api_base_url: &str,
) -> Result<(String, AuthorizationServerMetadata)> {
    let prm: ProtectedResourceMetadata = get_json(
        http,
        &format!("{api_base_url}/.well-known/oauth-protected-resource"),
    )
    .await
    .context("the Library API does not advertise browser sign-in")?;
    let issuer = prm
        .authorization_servers
        .first()
        .ok_or_else(|| {
            anyhow!("the Library API names no authorization server")
        })?
        .trim_end_matches('/')
        .to_string();
    let metadata_url = authorization_server_metadata_url(&issuer)?;
    let metadata: AuthorizationServerMetadata =
        get_json(http, &metadata_url).await?;
    if metadata.issuer.trim_end_matches('/') != issuer {
        bail!(
            "authorization server metadata names issuer {}, expected {issuer}",
            metadata.issuer
        );
    }
    if !metadata
        .code_challenge_methods_supported
        .iter()
        .any(|m| m == "S256")
    {
        bail!("the authorization server does not support PKCE S256");
    }
    Ok((prm.resource, metadata))
}

async fn register_client(
    http: &reqwest::Client,
    registration_endpoint: &str,
    redirect_uri: &str,
) -> Result<String> {
    let response = http
        .post(registration_endpoint)
        .json(&serde_json::json!({
            "client_name": "Library CLI",
            "redirect_uris": [redirect_uri],
            "grant_types": ["authorization_code", "refresh_token"],
            "response_types": ["code"],
            "token_endpoint_auth_method": "none",
            "scope": SCOPES,
        }))
        .send()
        .await
        .context("client registration failed to reach the server")?;
    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    if !status.is_success() {
        bail!("client registration returned {status}: {text}");
    }
    let registered: RegistrationResponse = serde_json::from_str(&text)
        .context("client registration returned unexpected JSON")?;
    Ok(registered.client_id)
}

async fn token_request(
    http: &reqwest::Client,
    token_endpoint: &str,
    form: &[(&str, &str)],
) -> Result<TokenResponse> {
    let response = http
        .post(token_endpoint)
        .form(form)
        .send()
        .await
        .context("the token endpoint could not be reached")?;
    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    if !status.is_success() {
        bail!("the token endpoint returned {status}: {text}");
    }
    serde_json::from_str(&text)
        .context("the token endpoint returned unexpected JSON")
}

fn open_browser(url: &str) -> bool {
    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("open").arg(url).status();
    #[cfg(target_os = "windows")]
    let result = std::process::Command::new("cmd")
        .args(["/C", "start", "", url])
        .status();
    #[cfg(all(unix, not(target_os = "macos")))]
    let result = std::process::Command::new("xdg-open").arg(url).status();
    result.map(|s| s.success()).unwrap_or(false)
}

/// What came back to the loopback redirect.
#[derive(Debug, PartialEq, Eq)]
enum Callback {
    Code(String),
    Error(String),
    /// Not the redirect (a favicon request, a stray probe): keep waiting.
    Ignored,
}

fn parse_callback(request_line: &str, expected_state: &str) -> Callback {
    let Some(target) = request_line.split_whitespace().nth(1) else {
        return Callback::Ignored;
    };
    let Ok(url) = url::Url::parse(&format!("http://127.0.0.1{target}"))
    else {
        return Callback::Ignored;
    };
    if url.path() != "/callback" {
        return Callback::Ignored;
    }
    let param = |name: &str| {
        url.query_pairs()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.into_owned())
    };
    if param("state").as_deref() != Some(expected_state) {
        return Callback::Error(
            "state did not match; sign-in was not started by this command"
                .into(),
        );
    }
    if let Some(error) = param("error") {
        let detail = param("error_description").unwrap_or_default();
        return Callback::Error(
            format!("{error} {detail}").trim().to_string(),
        );
    }
    match param("code") {
        Some(code) if !code.is_empty() => Callback::Code(code),
        _ => Callback::Error("the redirect carried no code".into()),
    }
}

async fn wait_for_code(
    listener: TcpListener,
    state: &str,
) -> Result<String> {
    loop {
        let (mut stream, _) = listener.accept().await?;
        let mut buffer = Vec::with_capacity(8192);
        let mut request_line_complete = false;
        while buffer.len() < 8192 {
            let mut chunk = [0u8; 1024];
            let limit = (8192 - buffer.len()).min(chunk.len());
            let n = stream.read(&mut chunk[..limit]).await?;
            if n == 0 {
                break;
            }
            buffer.extend_from_slice(&chunk[..n]);
            if buffer.windows(2).any(|pair| pair == b"\r\n") {
                request_line_complete = true;
                break;
            }
        }
        let request = String::from_utf8_lossy(&buffer);
        let first_line = request.lines().next().unwrap_or_default();

        let outcome = if request_line_complete {
            parse_callback(first_line, state)
        } else {
            Callback::Error(
                "the callback request line was incomplete or too long"
                    .into(),
            )
        };
        let (status, message) = match &outcome {
            Callback::Code(_) => ("200 OK", "Library CLI にログインしました。このタブは閉じてかまいません。"),
            Callback::Error(_) => ("400 Bad Request", "ログインに失敗しました。ターミナルを確認してください。"),
            Callback::Ignored => ("404 Not Found", ""),
        };
        let body = format!(
            "<!doctype html><meta charset=\"utf-8\"><title>Library CLI</title><p>{message}</p>"
        );
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let _ = stream.write_all(response.as_bytes()).await;
        let _ = stream.shutdown().await;

        match outcome {
            Callback::Code(code) => return Ok(code),
            Callback::Error(error) => bail!("sign-in failed: {error}"),
            Callback::Ignored => continue,
        }
    }
}

/// Run the whole browser sign-in and return the session to save.
pub async fn browser_login(
    api_base_url: &str,
    open: bool,
) -> Result<OAuthSession> {
    let http = http()?;
    let (resource, metadata) = discover(&http, api_base_url).await?;
    let registration_endpoint =
        metadata.registration_endpoint.as_deref().ok_or_else(|| {
            anyhow!(
            "the authorization server does not allow client registration"
        )
        })?;

    // Bind first, so the redirect URI names a port that is really ours.
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .context("failed to open a local port for the sign-in redirect")?;
    let redirect_uri = format!(
        "http://127.0.0.1:{}/callback",
        listener.local_addr()?.port()
    );

    let client_id =
        register_client(&http, registration_endpoint, &redirect_uri)
            .await?;
    let verifier = random_token();
    let state = random_token();

    let mut authorize = url::Url::parse(&metadata.authorization_endpoint)
        .context("the authorize endpoint is not a URL")?;
    authorize
        .query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", &client_id)
        .append_pair("redirect_uri", &redirect_uri)
        .append_pair("scope", SCOPES)
        .append_pair("state", &state)
        .append_pair("code_challenge", &code_challenge(&verifier))
        .append_pair("code_challenge_method", "S256")
        .append_pair("resource", &resource);

    eprintln!("ブラウザでログインしてください:\n  {authorize}");
    if open && !open_browser(authorize.as_str()) {
        eprintln!("(ブラウザを開けませんでした。上のURLを開いてください)");
    }

    let code = tokio::time::timeout(
        LOGIN_TIMEOUT,
        wait_for_code(listener, &state),
    )
    .await
    .map_err(|_| anyhow!("timed out waiting for the browser sign-in"))??;

    let tokens = token_request(
        &http,
        &metadata.token_endpoint,
        &[
            ("grant_type", "authorization_code"),
            ("code", &code),
            ("redirect_uri", &redirect_uri),
            ("client_id", &client_id),
            ("code_verifier", &verifier),
            ("resource", &resource),
        ],
    )
    .await?;

    Ok(OAuthSession {
        issuer: metadata.issuer,
        token_endpoint: metadata.token_endpoint,
        revocation_endpoint: metadata.revocation_endpoint,
        client_id,
        resource,
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
        expires_at: tokens.expires_in.map(|s| now_unix() + s),
    })
}

/// Swap the refresh token for a new access token.
pub async fn refresh(session: &OAuthSession) -> Result<OAuthSession> {
    let refresh_token =
        session.refresh_token.as_deref().ok_or_else(|| {
            anyhow!(
                "the sign-in has expired; run `library auth login` again"
            )
        })?;
    let tokens = token_request(
        &http()?,
        &session.token_endpoint,
        &[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", &session.client_id),
            ("resource", &session.resource),
        ],
    )
    .await
    .map_err(|e| {
        anyhow!("{e}\nthe sign-in could not be renewed; run `library auth login` again")
    })?;
    Ok(OAuthSession {
        access_token: tokens.access_token,
        // Servers that rotate refresh tokens send a new one; others keep
        // the old one valid.
        refresh_token: tokens
            .refresh_token
            .or_else(|| session.refresh_token.clone()),
        expires_at: tokens.expires_in.map(|s| now_unix() + s),
        ..session.clone()
    })
}

/// Best-effort sign-out on the server. Local logout proceeds either way.
pub async fn revoke(session: &OAuthSession) {
    let (Some(endpoint), Some(token)) =
        (&session.revocation_endpoint, &session.refresh_token)
    else {
        return;
    };
    if let Ok(http) = http() {
        let _ = http
            .post(endpoint)
            .form(&[
                ("token", token.as_str()),
                ("token_type_hint", "refresh_token"),
                ("client_id", session.client_id.as_str()),
            ])
            .send()
            .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn challenge_matches_rfc_7636_example() {
        // RFC 7636 Appendix B.
        assert_eq!(
            code_challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn random_tokens_are_long_and_distinct() {
        let a = random_token();
        assert!(a.len() >= 43, "PKCE verifiers need 43+ characters");
        assert_ne!(a, random_token());
    }

    #[test]
    fn callback_returns_the_code_for_the_matching_state() {
        assert_eq!(
            parse_callback(
                "GET /callback?code=abc&state=s1 HTTP/1.1",
                "s1"
            ),
            Callback::Code("abc".into())
        );
    }

    #[test]
    fn callback_rejects_a_foreign_state() {
        assert!(matches!(
            parse_callback(
                "GET /callback?code=abc&state=other HTTP/1.1",
                "s1"
            ),
            Callback::Error(_)
        ));
    }

    #[test]
    fn callback_reports_a_denied_sign_in() {
        let outcome = parse_callback(
            "GET /callback?error=access_denied&error_description=no&state=s1 HTTP/1.1",
            "s1",
        );
        assert_eq!(outcome, Callback::Error("access_denied no".into()));
    }

    #[test]
    fn unrelated_requests_are_ignored() {
        assert_eq!(
            parse_callback("GET /favicon.ico HTTP/1.1", "s1"),
            Callback::Ignored
        );
        assert_eq!(parse_callback("", "s1"), Callback::Ignored);
    }

    fn session(expires_at: Option<i64>) -> OAuthSession {
        OAuthSession {
            issuer: "https://issuer".into(),
            token_endpoint: "https://issuer/token".into(),
            revocation_endpoint: None,
            client_id: "c".into(),
            resource: "https://api/mcp".into(),
            access_token: "a".into(),
            refresh_token: Some("r".into()),
            expires_at,
        }
    }

    #[test]
    fn refresh_starts_a_minute_before_expiry() {
        assert!(!session(Some(1_000)).needs_refresh(900));
        assert!(session(Some(1_000)).needs_refresh(950));
        assert!(session(Some(1_000)).needs_refresh(2_000));
        assert!(!session(None).needs_refresh(2_000));
    }
}
