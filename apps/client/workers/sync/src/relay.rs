use crate::{
    document::{self, Document},
    model::INTERNAL,
};
use library_worker_common::{header, now};
use serde_json::{json, Value};
use worker::*;

pub fn send(ws: &WebSocket, value: &Value) {
    let _ = ws.send_with_str(value.to_string());
}
pub fn broadcast(state: &State, value: &Value) {
    for ws in state.get_websockets() {
        send(&ws, value)
    }
}
pub fn binary(state: &State, update: &[u8], except: Option<&WebSocket>) {
    for ws in state.get_websockets() {
        if except != Some(&ws) {
            let _ = ws.send_with_bytes(update);
        }
    }
}
pub fn presence(state: &State, except: Option<&WebSocket>) {
    let sockets: Vec<_> = state
        .get_websockets()
        .into_iter()
        .filter(|s| except != Some(s))
        .collect();
    let frame = json!({"type":"presence","onlineCount":sockets.len()});
    for ws in sockets {
        send(&ws, &frame)
    }
}
pub fn text(state: &State, sender: &WebSocket, text: &str) {
    for ws in state.get_websockets() {
        if ws != *sender {
            let _ = ws.send_with_str(text);
        }
    }
}
/// `code` is one of the recoverable `CHECKPOINT_*` codes; `None` is terminal.
pub fn error_frame(
    message: &str,
    operation: Option<&str>,
    code: Option<&str>,
) -> Value {
    let mut frame = json!({"type":"live-error","message":message});
    if let Some(id) = operation {
        frame["operation_id"] = json!(id)
    }
    if let Some(code) = code {
        frame["code"] = json!(code)
    }
    frame
}
pub fn error(
    ws: &WebSocket,
    message: &str,
    operation: Option<&str>,
    code: Option<&str>,
) {
    send(ws, &error_frame(message, operation, code));
}

/// Logged updates a room lets build up before an alarm folds them into its
/// snapshot.
const COMPACT_AFTER: u64 = 50;

/// A relay for workspace-wide Yjs state (records, views, presence).
///
/// It never builds the document to serve a request. A joining client is sent
/// the stored snapshot and logged updates as they are, and an update from a
/// client is validated, logged and relayed. Building the document -- decoding
/// the snapshot, applying the log, encoding it again -- grows with the room,
/// and done on every join it outlasted the request for a large one, so no
/// one could join at all. It happens only when an alarm compacts the log.
#[durable_object]
pub struct PhotonSyncRoom {
    state: State,
}
impl PhotonSyncRoom {
    async fn compact_later(&self, waiting: u64) -> Result<()> {
        if waiting > COMPACT_AFTER {
            crate::tickets::schedule(&self.state.storage(), now() + 1000)
                .await?;
        }
        Ok(())
    }
}
impl DurableObject for PhotonSyncRoom {
    fn new(state: State, _env: Env) -> Self {
        Self { state }
    }
    async fn fetch(&self, request: Request) -> Result<Response> {
        if request.path() == "/internal/engine-changed"
            && header(&request, INTERNAL).as_deref() == Some("1")
        {
            broadcast(&self.state, &json!({"type":"engine-changed"}));
            return Response::empty();
        }
        if !header(&request, "upgrade")
            .is_some_and(|v| v.eq_ignore_ascii_case("websocket"))
        {
            return Response::error("Expected WebSocket upgrade", 426);
        }
        // Accepted before the state is read: an update another client sends
        // while the reads are in flight is relayed to this socket too.
        // Otherwise it could be logged after the reads and relayed before
        // the accept, reaching this client by neither path. An update that
        // arrives both ways is applied twice, which Yjs ignores.
        let pair = WebSocketPair::new()?;
        self.state.accept_web_socket(&pair.server);
        let stored = match document::stored(&self.state.storage()).await {
            Ok(stored) => stored,
            Err(error) => {
                let _ =
                    pair.server.close(Some(1011), Some("Room unavailable"));
                return Err(error);
            }
        };
        // The client treats its first binary frame as the room's state, so
        // an empty room still sends one.
        if stored.snapshot.is_empty() {
            pair.server.send_with_bytes(document::EMPTY_UPDATE)?;
        } else {
            pair.server.send_with_bytes(&stored.snapshot)?;
        }
        for update in &stored.updates {
            pair.server.send_with_bytes(update)?;
        }
        self.compact_later(stored.updates.len() as u64).await?;
        presence(&self.state, None);
        Response::from_websocket(pair.client)
    }
    async fn websocket_message(
        &self,
        sender: WebSocket,
        message: WebSocketIncomingMessage,
    ) -> Result<()> {
        match message {
            WebSocketIncomingMessage::String(value) => {
                if serde_json::from_str::<Value>(&value).ok().is_some_and(
                    |v| {
                        v["type"] == "awareness"
                            || v["type"] == "engine-changed"
                    },
                ) {
                    text(&self.state, &sender, &value)
                }
            }
            WebSocketIncomingMessage::Binary(bytes) => {
                if let Some(waiting) =
                    document::append_logged(&self.state.storage(), &bytes)
                        .await?
                {
                    binary(&self.state, &bytes, Some(&sender));
                    self.compact_later(waiting).await?;
                }
            }
        }
        Ok(())
    }
    async fn websocket_close(
        &self,
        ws: WebSocket,
        _code: usize,
        _reason: String,
        _clean: bool,
    ) -> Result<()> {
        presence(&self.state, Some(&ws));
        Ok(())
    }
    async fn websocket_error(
        &self,
        ws: WebSocket,
        _error: Error,
    ) -> Result<()> {
        presence(&self.state, Some(&ws));
        Ok(())
    }
    async fn alarm(&self) -> Result<Response> {
        // Built for this compaction only, and dropped with it: clients
        // already have every update, which was relayed as it arrived. A log
        // too long for one pass leaves the document behind and schedules the
        // next alarm itself.
        let mut doc = Document::default();
        doc.advance(&self.state.storage()).await?;
        Response::empty()
    }
}
