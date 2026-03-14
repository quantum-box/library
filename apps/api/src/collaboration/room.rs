/// A collaborative document room.
///
/// Each room holds an in-memory Y.Doc and a set of connected
/// peers. The sync protocol runs entirely in memory; persistence
/// is triggered only on disconnect or by the manager's timer.
use std::collections::HashMap;

use tokio::sync::mpsc;
use yrs::{Doc, ReadTxn, Transact};

use crate::collaboration::protocol;

pub type PeerId = u64;
pub type PeerSender = mpsc::UnboundedSender<Vec<u8>>;

pub struct DocumentRoom {
    doc: Doc,
    peers: HashMap<PeerId, PeerSender>,
    next_id: PeerId,
    dirty: bool,
}

impl DocumentRoom {
    /// Create a new room. If `state` is provided, the Y.Doc is
    /// initialised from the persisted binary.
    pub fn new(state: Option<&[u8]>) -> Self {
        let doc = Doc::new();
        if let Some(bytes) = state {
            use yrs::updates::decoder::Decode;
            use yrs::Update;
            if let Ok(update) = Update::decode_v1(bytes) {
                let mut txn = doc.transact_mut();
                let _ = txn.apply_update(update);
            }
        }
        Self {
            doc,
            peers: HashMap::new(),
            next_id: 1,
            dirty: false,
        }
    }

    /// Register a new peer. Returns the peer id, a receiver for
    /// outgoing messages, and the initial sync messages to send.
    pub fn add_peer(
        &mut self,
    ) -> (PeerId, mpsc::UnboundedReceiver<Vec<u8>>, Vec<Vec<u8>>) {
        let id = self.next_id;
        self.next_id += 1;
        let (tx, rx) = mpsc::unbounded_channel();
        self.peers.insert(id, tx);

        // Send SyncStep1 so the client replies with its state
        let sync_step1 = protocol::encode_sync_step1(&self.doc);
        (id, rx, vec![sync_step1])
    }

    /// Remove a peer from the room.
    pub fn remove_peer(&mut self, id: PeerId) {
        self.peers.remove(&id);
    }

    /// Process a binary message from a peer and route responses.
    ///
    /// Peers whose receiver has been dropped are automatically
    /// removed to prevent memory leaks.
    pub fn handle_message(&mut self, from: PeerId, msg: &[u8]) {
        let responses = protocol::handle_message(&self.doc, msg);
        let mut dead_peers: Vec<PeerId> = Vec::new();
        for resp in responses {
            if resp.broadcast {
                self.dirty = true;
                for (&pid, tx) in &self.peers {
                    if pid != from && tx.send(resp.data.clone()).is_err() {
                        dead_peers.push(pid);
                    }
                }
            } else {
                // Send only to the originating peer
                if let Some(tx) = self.peers.get(&from) {
                    if tx.send(resp.data.clone()).is_err() {
                        dead_peers.push(from);
                    }
                }
            }
        }
        for pid in dead_peers {
            self.peers.remove(&pid);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.peers.is_empty()
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    /// Encode the full Y.Doc state for persistence.
    pub fn encode_state(&self) -> Vec<u8> {
        let txn = self.doc.transact();
        txn.encode_state_as_update_v1(&yrs::StateVector::default())
    }

    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }
}
