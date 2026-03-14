/// Real-time collaborative editing using Yjs/yrs CRDT.
///
/// Implements y-websocket compatible protocol over axum WebSocket,
/// enabling multiple users to edit documents simultaneously.
pub mod encoding;
pub mod handler;
pub mod manager;
pub mod persistence;
pub mod protocol;
pub mod room;

pub use handler::ws_handler;
pub use manager::DocumentManager;
pub use persistence::{DocumentPersistence, SqlxDocumentPersistence};
