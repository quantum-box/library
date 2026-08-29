//! MCP HTTP+SSE transport.
//!
//! `POST /mcp` already speaks JSON-RPC over a plain request/response pair,
//! which serves clients that can hold a single round trip. Editors and
//! agent runtimes that predate Streamable HTTP instead expect the
//! two-endpoint SSE transport: they open a long-lived `GET /sse` stream,
//! read the `endpoint` event to learn where to post, and then send every
//! JSON-RPC request to that URL while the answers come back down the
//! stream.
//!
//! Both transports authenticate and execute through
//! [`crate::handler::mcp::dispatch_rpc`], so the tool set and the rules
//! about which tools need credentials cannot drift between them.

use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;

use axum::{
    extract::{Extension, Query},
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Response,
    },
    Json,
};
use futures_util::stream::{self, Stream, StreamExt};
use once_cell::sync::Lazy;
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;

use crate::app::LibraryApp;
use crate::handler::mcp::{
    dispatch_rpc, sse_requires_auth, unauthorized_response, JsonRpcRequest,
};
use crate::sdk_auth::SdkAuthApp;

/// Ceiling on concurrently open SSE sessions. A session is only a channel
/// and a header map, but the map is process-global and reachable without
/// credentials whenever `MCP_AUTH_REQUIRED` is off, so it needs a bound.
const MAX_SSE_SESSIONS: usize = 1024;

static MCP_SSE_SESSIONS: Lazy<Mutex<HashMap<String, McpSseSession>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

struct McpSseSession {
    /// Answers travel back to the client over the `GET /sse` stream, not
    /// as the body of the `POST /messages` that asked the question.
    sender: mpsc::UnboundedSender<Value>,
    /// Credentials presented when the stream was opened. Clients that
    /// authenticate only on `GET /sse` and post bare messages afterwards
    /// stay authenticated for the life of the session.
    authorization: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MessagesQuery {
    #[serde(alias = "session_id")]
    session_id: String,
}

/// `GET /sse` — open the event stream and hand back the URL to post to.
pub async fn mcp_sse_handler(
    headers: HeaderMap,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, Response> {
    if sse_requires_auth(&headers) {
        return Err(unauthorized_response());
    }

    let session_id = Uuid::new_v4().to_string();
    let (sender, receiver) = mpsc::unbounded_channel::<Value>();
    let authorization = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned);

    {
        let mut sessions = MCP_SSE_SESSIONS.lock().await;
        if sessions.len() >= MAX_SSE_SESSIONS {
            tracing::warn!(
                open_sessions = sessions.len(),
                "refusing MCP SSE session: too many open streams"
            );
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                "Too many open MCP SSE sessions",
            )
                .into_response());
        }
        sessions.insert(
            session_id.clone(),
            McpSseSession {
                sender,
                authorization,
            },
        );
    }

    tracing::debug!(session_id = %session_id, "opened MCP SSE session");

    let endpoint = message_endpoint(&session_id);
    let endpoint_event = stream::once(async move {
        Ok(Event::default().event("endpoint").data(endpoint))
    });

    // The guard rides along in the stream's state, so the session is
    // dropped from the registry the moment the client disconnects.
    let guard = SessionGuard {
        session_id: session_id.clone(),
    };
    let messages = stream::unfold(
        (receiver, guard),
        |(mut receiver, guard)| async move {
            let message = receiver.recv().await?;
            let event =
                Event::default().event("message").json_data(message);
            let event = match event {
                Ok(event) => event,
                Err(error) => {
                    // A tool result that will not serialize would
                    // otherwise end the stream silently.
                    tracing::error!(
                        "failed to encode MCP SSE message: {error}"
                    );
                    return None;
                }
            };
            Some((Ok(event), (receiver, guard)))
        },
    );

    Ok(Sse::new(endpoint_event.chain(messages))
        .keep_alive(KeepAlive::default()))
}

/// `POST /messages?sessionId=…` — accept one JSON-RPC request and answer
/// it over the session's open stream.
pub async fn mcp_sse_messages_handler(
    Query(query): Query<MessagesQuery>,
    headers: HeaderMap,
    Extension(library_app): Extension<Arc<LibraryApp>>,
    Extension(sdk): Extension<Arc<SdkAuthApp>>,
    Json(request): Json<JsonRpcRequest>,
) -> Response {
    let (sender, session_authorization) = {
        let sessions = MCP_SSE_SESSIONS.lock().await;
        match sessions.get(&query.session_id) {
            Some(session) => {
                (session.sender.clone(), session.authorization.clone())
            }
            None => {
                return (StatusCode::NOT_FOUND, "Unknown MCP SSE session")
                    .into_response();
            }
        }
    };

    let headers = merge_authorization(headers, session_authorization);
    let response =
        match dispatch_rpc(&headers, library_app, sdk, request).await {
            Ok(Some(response)) => response,
            // Notifications owe no answer, so nothing goes on the stream.
            Ok(None) => return StatusCode::ACCEPTED.into_response(),
            Err(challenge) => return challenge,
        };

    if sender.send(response).is_err() {
        // The receiving stream closed between the lookup above and now.
        MCP_SSE_SESSIONS.lock().await.remove(&query.session_id);
        return (StatusCode::GONE, "MCP SSE session closed")
            .into_response();
    }

    StatusCode::ACCEPTED.into_response()
}

/// Carry the stream's credentials onto a message that arrived without any.
/// A header on the `POST` always wins, so a client that refreshes its
/// token mid-session is not pinned to the one it opened the stream with.
fn merge_authorization(
    mut headers: HeaderMap,
    session_authorization: Option<String>,
) -> HeaderMap {
    if headers.contains_key(AUTHORIZATION) {
        return headers;
    }
    let Some(authorization) = session_authorization else {
        return headers;
    };
    if let Ok(value) = authorization.parse() {
        headers.insert(AUTHORIZATION, value);
    }
    headers
}

/// Where the client should post its requests. Relative by default, which
/// every MCP client resolves against the SSE URL it already opened.
/// Deployments that terminate the stream and the messages endpoint on
/// different hosts set `MCP_SSE_MESSAGE_ENDPOINT` to an absolute URL.
fn message_endpoint(session_id: &str) -> String {
    let base = std::env::var("MCP_SSE_MESSAGE_ENDPOINT")
        .unwrap_or_else(|_| DEFAULT_SSE_MESSAGE_ENDPOINT.to_string());
    append_session_id(&base, session_id)
}

const DEFAULT_SSE_MESSAGE_ENDPOINT: &str = "/messages";

fn append_session_id(base: &str, session_id: &str) -> String {
    let separator = if base.contains('?') { '&' } else { '?' };
    format!("{base}{separator}sessionId={session_id}")
}

struct SessionGuard {
    session_id: String,
}

impl Drop for SessionGuard {
    fn drop(&mut self) {
        let session_id = self.session_id.clone();
        tokio::spawn(async move {
            MCP_SSE_SESSIONS.lock().await.remove(&session_id);
            tracing::debug!(
                session_id = %session_id,
                "closed MCP SSE session"
            );
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Read the first chunk the SSE response emits. The stream stays
    /// open afterwards, so buffering the whole body would never return.
    async fn first_event(
        response: axum::response::Response,
    ) -> (String, axum::body::BodyDataStream) {
        let mut stream = response.into_body().into_data_stream();
        let chunk = stream
            .next()
            .await
            .expect("the endpoint event is sent immediately")
            .expect("the endpoint event is not an error");
        (String::from_utf8_lossy(&chunk).into_owned(), stream)
    }

    #[tokio::test]
    async fn opening_a_stream_announces_where_to_post_messages() {
        let response = mcp_sse_handler(HeaderMap::new())
            .await
            .expect("an anonymous stream opens when auth is not required")
            .into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let (event, stream) = first_event(response).await;

        assert!(event.contains("event: endpoint"), "got: {event}");
        assert!(
            event.contains("data: /messages?sessionId="),
            "got: {event}"
        );
        drop(stream);
    }

    #[tokio::test]
    async fn the_announced_session_is_one_the_message_endpoint_knows() {
        let response = mcp_sse_handler(HeaderMap::new())
            .await
            .expect("an anonymous stream opens when auth is not required")
            .into_response();
        let (event, stream) = first_event(response).await;

        let session_id = event
            .split("sessionId=")
            .nth(1)
            .expect("the endpoint event carries a session id")
            .trim()
            .to_string();

        assert!(
            MCP_SSE_SESSIONS.lock().await.contains_key(&session_id),
            "session {session_id} was announced but not registered"
        );
        drop(stream);
    }

    #[tokio::test]
    async fn a_stream_opened_with_credentials_remembers_them() {
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, "Bearer pk_test".parse().unwrap());

        let response = mcp_sse_handler(headers)
            .await
            .expect("an authenticated stream opens")
            .into_response();
        let (event, stream) = first_event(response).await;
        let session_id = event
            .split("sessionId=")
            .nth(1)
            .expect("the endpoint event carries a session id")
            .trim()
            .to_string();

        let sessions = MCP_SSE_SESSIONS.lock().await;
        let session =
            sessions.get(&session_id).expect("session is registered");
        assert_eq!(
            session.authorization.as_deref(),
            Some("Bearer pk_test")
        );
        drop(sessions);
        drop(stream);
    }

    #[test]
    fn the_default_endpoint_is_a_relative_path_with_the_session_id() {
        assert_eq!(
            append_session_id(DEFAULT_SSE_MESSAGE_ENDPOINT, "abc"),
            "/messages?sessionId=abc".to_string()
        );
    }

    #[test]
    fn the_session_id_appends_to_an_endpoint_that_already_has_a_query() {
        assert_eq!(
            append_session_id(
                "https://api.example.com/messages?tenant=acme",
                "abc"
            ),
            "https://api.example.com/messages?tenant=acme&sessionId=abc"
                .to_string()
        );
    }

    #[test]
    fn a_header_on_the_message_wins_over_the_stream_credentials() {
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, "Bearer fresh".parse().unwrap());

        let merged =
            merge_authorization(headers, Some("Bearer stale".to_string()));

        assert_eq!(
            merged.get(AUTHORIZATION).unwrap().to_str().unwrap(),
            "Bearer fresh"
        );
    }

    #[test]
    fn a_bare_message_inherits_the_stream_credentials() {
        let merged = merge_authorization(
            HeaderMap::new(),
            Some("Bearer stream".to_string()),
        );

        assert_eq!(
            merged.get(AUTHORIZATION).unwrap().to_str().unwrap(),
            "Bearer stream"
        );
    }

    #[test]
    fn a_bare_message_on_an_anonymous_stream_stays_anonymous() {
        let merged = merge_authorization(HeaderMap::new(), None);

        assert!(merged.get(AUTHORIZATION).is_none());
    }
}
