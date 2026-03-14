/// axum WebSocket handler for collaborative editing.
use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::Mutex;

use crate::collaboration::manager::DocumentManager;
use crate::collaboration::room::DocumentRoom;

#[derive(Clone)]
pub struct CollaborationState {
    pub manager: Arc<DocumentManager>,
}

#[derive(Deserialize)]
pub struct WsParams {
    /// Operator ID for tenant isolation.
    pub operator_id: String,
}

/// WebSocket upgrade handler.
///
/// Route: GET /ws/collab/:document_key
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Path(document_key): Path<String>,
    Query(params): Query<WsParams>,
    State(state): State<CollaborationState>,
) -> impl IntoResponse {
    let operator_id = params.operator_id.clone();
    tracing::info!(
        document_key = %document_key,
        operator_id = %operator_id,
        "WebSocket upgrade requested"
    );
    ws.on_upgrade(move |socket| {
        handle_socket(socket, document_key, operator_id, state.manager)
    })
}

async fn handle_socket(
    socket: WebSocket,
    document_key: String,
    operator_id: String,
    manager: Arc<DocumentManager>,
) {
    let (peer_id, mut rx, initial_msgs, room) =
        manager.connect(&document_key, &operator_id).await;

    let (mut ws_tx, mut ws_rx) = socket.split();

    // Send initial sync messages
    for msg in initial_msgs {
        if ws_tx.send(Message::Binary(msg)).await.is_err() {
            manager
                .disconnect(&document_key, &operator_id, peer_id)
                .await;
            return;
        }
    }

    // Spawn task: room → WebSocket
    let doc_key_clone = document_key.clone();
    let send_task = tokio::spawn(async move {
        while let Some(data) = rx.recv().await {
            if ws_tx.send(Message::Binary(data)).await.is_err() {
                break;
            }
        }
        let _ = ws_tx.close().await;
        doc_key_clone
    });

    // Main loop: WebSocket → room
    recv_loop(&mut ws_rx, &room, peer_id).await;

    // Cleanup
    send_task.abort();
    manager
        .disconnect(&document_key, &operator_id, peer_id)
        .await;
}

async fn recv_loop(
    ws_rx: &mut futures_util::stream::SplitStream<WebSocket>,
    room: &Arc<Mutex<DocumentRoom>>,
    peer_id: u64,
) {
    while let Some(result) = ws_rx.next().await {
        match result {
            Ok(Message::Binary(data)) => {
                let mut r = room.lock().await;
                r.handle_message(peer_id, &data);
            }
            Ok(Message::Close(_)) => break,
            Ok(_) => {
                // Ignore text, ping, pong
            }
            Err(e) => {
                tracing::debug!(
                    error = %e,
                    "WebSocket receive error"
                );
                break;
            }
        }
    }
}
