use crate::{document::Document, model::INTERNAL};
use futures::lock::Mutex;
use library_worker_common::header;
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
pub fn error(
    ws: &WebSocket,
    message: &str,
    operation: Option<&str>,
    stale: bool,
) {
    let mut frame = json!({"type":"live-error","message":message});
    if let Some(id) = operation {
        frame["operation_id"] = json!(id)
    }
    if stale {
        frame["code"] = json!("CHECKPOINT_STALE")
    }
    send(ws, &frame);
}

#[durable_object]
pub struct PhotonSyncRoom {
    state: State,
    doc: Mutex<Document>,
}
impl DurableObject for PhotonSyncRoom {
    fn new(state: State, _env: Env) -> Self {
        Self {
            state,
            doc: Mutex::new(Document::default()),
        }
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
        let mut doc = self.doc.lock().await;
        if doc.advance(&self.state.storage()).await? {
            binary(&self.state, &doc.snapshot(), None)
        }
        let pair = WebSocketPair::new()?;
        self.state.accept_web_socket(&pair.server);
        pair.server.send_with_bytes(doc.snapshot())?;
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
                let mut doc = self.doc.lock().await;
                if doc.advance(&self.state.storage()).await? {
                    binary(&self.state, &doc.snapshot(), None)
                }
                if doc
                    .append(&self.state.storage(), &bytes, vec![], vec![])
                    .await?
                {
                    binary(&self.state, &bytes, Some(&sender))
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
        let mut doc = self.doc.lock().await;
        if doc.advance(&self.state.storage()).await? {
            binary(&self.state, &doc.snapshot(), None)
        }
        Response::empty()
    }
}
