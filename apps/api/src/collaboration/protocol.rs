/// y-websocket compatible sync protocol.
///
/// Message format (binary WebSocket frames):
/// - `[0x00][sync_step][payload]` — sync messages
/// - `[0x01][awareness_update]` — awareness messages
///
/// Sync steps:
/// - SyncStep1 (0): sender's state vector (VarUint8Array)
/// - SyncStep2 (1): update/diff for the received state vector
/// - SyncUpdate (2): incremental update after initial sync
use crate::collaboration::encoding::{
    read_var_bytes, read_var_uint, write_var_bytes, write_var_uint,
};
use yrs::updates::decoder::Decode;
use yrs::updates::encoder::Encode;
use yrs::{Doc, ReadTxn, StateVector, Transact, Update};

pub const MSG_SYNC: u8 = 0;
pub const MSG_AWARENESS: u8 = 1;

pub const SYNC_STEP1: u8 = 0;
pub const SYNC_STEP2: u8 = 1;
pub const SYNC_UPDATE: u8 = 2;

/// A response message to send back to one or more peers.
pub struct Response {
    /// Message bytes to send.
    pub data: Vec<u8>,
    /// If true, broadcast to all peers except sender.
    /// If false, send only to the sender.
    pub broadcast: bool,
}

/// Encode a SyncStep1 message containing the doc's state vector.
pub fn encode_sync_step1(doc: &Doc) -> Vec<u8> {
    let txn = doc.transact();
    let sv = txn.state_vector().encode_v1();
    let mut buf = Vec::new();
    write_var_uint(&mut buf, MSG_SYNC as u64);
    write_var_uint(&mut buf, SYNC_STEP1 as u64);
    write_var_bytes(&mut buf, &sv);
    buf
}

/// Encode a SyncStep2 message containing the diff for a given
/// remote state vector.
pub fn encode_sync_step2(doc: &Doc, remote_sv: &[u8]) -> Vec<u8> {
    let sv = StateVector::decode_v1(remote_sv).unwrap_or_default();
    let txn = doc.transact();
    let update = txn.encode_state_as_update_v1(&sv);
    let mut buf = Vec::new();
    write_var_uint(&mut buf, MSG_SYNC as u64);
    write_var_uint(&mut buf, SYNC_STEP2 as u64);
    write_var_bytes(&mut buf, &update);
    buf
}

/// Encode a SyncUpdate message for broadcasting an incremental
/// update.
pub fn encode_sync_update(update: &[u8]) -> Vec<u8> {
    let mut buf = Vec::new();
    write_var_uint(&mut buf, MSG_SYNC as u64);
    write_var_uint(&mut buf, SYNC_UPDATE as u64);
    write_var_bytes(&mut buf, update);
    buf
}

/// Encode an awareness message for broadcasting.
pub fn encode_awareness(data: &[u8]) -> Vec<u8> {
    let mut buf = Vec::new();
    write_var_uint(&mut buf, MSG_AWARENESS as u64);
    buf.extend_from_slice(data);
    buf
}

/// Handle an incoming binary message from a peer.
///
/// Returns a list of response messages with routing information.
pub fn handle_message(doc: &Doc, msg: &[u8]) -> Vec<Response> {
    if msg.is_empty() {
        return vec![];
    }
    let mut pos = 0;
    let msg_type = read_var_uint(msg, &mut pos) as u8;
    match msg_type {
        MSG_SYNC => handle_sync(doc, msg, &mut pos),
        MSG_AWARENESS => handle_awareness(msg, &mut pos),
        _ => {
            tracing::warn!(msg_type, "unknown message type");
            vec![]
        }
    }
}

fn handle_sync(doc: &Doc, msg: &[u8], pos: &mut usize) -> Vec<Response> {
    if *pos >= msg.len() {
        return vec![];
    }
    let sync_step = read_var_uint(msg, pos) as u8;
    match sync_step {
        SYNC_STEP1 => {
            // Peer sent its state vector; respond with our diff
            let remote_sv = read_var_bytes(msg, pos);
            let response = encode_sync_step2(doc, &remote_sv);
            vec![Response {
                data: response,
                broadcast: false,
            }]
        }
        SYNC_STEP2 | SYNC_UPDATE => {
            // Peer sent an update; apply it and broadcast
            let update_bytes = read_var_bytes(msg, pos);
            match Update::decode_v1(&update_bytes) {
                Ok(update) => {
                    let mut txn = doc.transact_mut();
                    if let Err(e) = txn.apply_update(update) {
                        tracing::warn!(
                            error = %e,
                            "failed to apply update"
                        );
                        return vec![];
                    }
                    drop(txn);

                    // Broadcast the raw update to other peers
                    let broadcast_msg = encode_sync_update(&update_bytes);
                    vec![Response {
                        data: broadcast_msg,
                        broadcast: true,
                    }]
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "failed to decode update"
                    );
                    vec![]
                }
            }
        }
        _ => {
            tracing::warn!(sync_step, "unknown sync step");
            vec![]
        }
    }
}

fn handle_awareness(msg: &[u8], pos: &mut usize) -> Vec<Response> {
    // Relay awareness data to all other peers as-is
    let awareness_data = msg[*pos..].to_vec();
    *pos = msg.len();
    let mut buf = Vec::new();
    write_var_uint(&mut buf, MSG_AWARENESS as u64);
    buf.extend_from_slice(&awareness_data);
    vec![Response {
        data: buf,
        broadcast: true,
    }]
}
