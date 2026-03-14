/// Manages the lifecycle of collaborative document rooms.
///
/// Rooms are created on first connection and removed when the
/// last peer disconnects (after persisting). A background task
/// periodically persists dirty rooms as a safety net.
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use tokio::sync::{mpsc, Mutex};

use crate::collaboration::persistence::DocumentPersistence;
use crate::collaboration::room::{DocumentRoom, PeerId};

pub struct DocumentManager {
    rooms: Arc<Mutex<HashMap<String, RoomHandle>>>,
    persistence: Arc<dyn DocumentPersistence>,
}

struct RoomHandle {
    room: Arc<Mutex<DocumentRoom>>,
    operator_id: String,
}

/// Build a composite room key that incorporates the operator ID
/// to guarantee tenant isolation.
fn room_key(operator_id: &str, document_key: &str) -> String {
    format!("{operator_id}:{document_key}")
}

impl DocumentManager {
    pub fn new(persistence: Arc<dyn DocumentPersistence>) -> Self {
        Self {
            rooms: Arc::new(Mutex::new(HashMap::new())),
            persistence,
        }
    }

    /// Start the background persistence task that saves dirty
    /// rooms every 30 seconds.
    pub fn start_background_persistence(self: &Arc<Self>) {
        let manager = Arc::clone(self);
        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(std::time::Duration::from_secs(30));
            loop {
                interval.tick().await;
                manager.persist_dirty_rooms().await;
            }
        });
    }

    /// Connect a peer to a document room. Creates the room if
    /// it does not exist, loading persisted state from the DB.
    ///
    /// The room key incorporates `operator_id` so that different
    /// tenants never share a room, even for identical document
    /// keys.
    pub async fn connect(
        &self,
        document_key: &str,
        operator_id: &str,
    ) -> (
        PeerId,
        mpsc::UnboundedReceiver<Vec<u8>>,
        Vec<Vec<u8>>,
        Arc<Mutex<DocumentRoom>>,
    ) {
        let key = room_key(operator_id, document_key);

        // Fast path: room already exists.
        {
            let rooms = self.rooms.lock().await;
            if let Some(handle) = rooms.get(&key) {
                let room_arc = Arc::clone(&handle.room);
                drop(rooms);
                let mut room = room_arc.lock().await;
                let (peer_id, rx, initial_msgs) = room.add_peer();
                tracing::info!(
                    document_key,
                    operator_id,
                    peer_id,
                    peers = room.peer_count(),
                    "peer connected"
                );
                return (peer_id, rx, initial_msgs, Arc::clone(&room_arc));
            }
        }

        // Slow path: load from DB without holding the rooms lock.
        let state =
            self.persistence.load(document_key).await.ok().flatten();

        // Re-acquire lock and insert (handle race where another
        // connection may have created the room in between).
        let room_arc = {
            let mut rooms = self.rooms.lock().await;
            if let Some(handle) = rooms.get(&key) {
                Arc::clone(&handle.room)
            } else {
                let room = Arc::new(Mutex::new(DocumentRoom::new(
                    state.as_deref(),
                )));
                rooms.insert(
                    key,
                    RoomHandle {
                        room: Arc::clone(&room),
                        operator_id: operator_id.to_string(),
                    },
                );
                room
            }
        };

        let mut room = room_arc.lock().await;
        let (peer_id, rx, initial_msgs) = room.add_peer();
        tracing::info!(
            document_key,
            operator_id,
            peer_id,
            peers = room.peer_count(),
            "peer connected"
        );
        (peer_id, rx, initial_msgs, Arc::clone(&room_arc))
    }

    /// Disconnect a peer. Persists and removes the room if it
    /// becomes empty.
    ///
    /// The rooms mutex is released before the async DB write to
    /// avoid blocking other operations.
    pub async fn disconnect(
        &self,
        document_key: &str,
        operator_id: &str,
        peer_id: PeerId,
    ) {
        let key = room_key(operator_id, document_key);

        // Collect persistence data while holding the lock, then
        // release before the async DB write.
        let persist_data = {
            let mut rooms = self.rooms.lock().await;
            if let Some(handle) = rooms.get(&key) {
                let mut room = handle.room.lock().await;
                room.remove_peer(peer_id);
                tracing::info!(
                    document_key,
                    peer_id,
                    peers = room.peer_count(),
                    "peer disconnected"
                );
                if room.is_empty() {
                    let state = room.encode_state();
                    let op_id = handle.operator_id.clone();
                    drop(room);
                    rooms.remove(&key);
                    Some((op_id, state))
                } else {
                    None
                }
            } else {
                None
            }
        };

        if let Some((op_id, state)) = persist_data {
            if let Err(e) =
                self.persistence.save(document_key, &op_id, &state).await
            {
                tracing::error!(
                    error = %e,
                    document_key,
                    "failed to persist document on \
                     last disconnect"
                );
            }
            tracing::info!(
                document_key,
                "room removed after last peer left"
            );
        }
    }

    /// Persist all dirty rooms (called by the background task).
    ///
    /// Dirty flags are only cleared for rooms whose state was
    /// successfully captured in the snapshot, preventing data
    /// loss when `try_lock` fails for a busy room.
    async fn persist_dirty_rooms(&self) {
        let (snapshot, captured_keys) = {
            let rooms = self.rooms.lock().await;
            let mut snapshot: Vec<(String, String, Vec<u8>)> = Vec::new();
            let mut captured: HashSet<String> = HashSet::new();
            for (key, handle) in rooms.iter() {
                if let Ok(mut room) = handle.room.try_lock() {
                    if room.is_dirty() {
                        snapshot.push((
                            key.clone(),
                            handle.operator_id.clone(),
                            room.encode_state(),
                        ));
                        room.clear_dirty();
                        captured.insert(key.clone());
                    }
                }
            }
            (snapshot, captured)
        };

        if captured_keys.is_empty() {
            return;
        }

        for (key, op_id, state) in snapshot {
            // Extract the original document_key from the
            // composite room key for persistence.
            let doc_key =
                key.split_once(':').map(|(_, dk)| dk).unwrap_or(&key);
            if let Err(e) =
                self.persistence.save(doc_key, &op_id, &state).await
            {
                tracing::error!(
                    error = %e,
                    document_key = %doc_key,
                    "background persist failed"
                );
            } else {
                tracing::debug!(
                    document_key = %doc_key,
                    "background persist ok"
                );
            }
        }
    }
}
