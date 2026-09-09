//! Yjs v1 wire compatibility and bounded, restartable snapshot compaction.
use crate::{model::MAX_UPDATE, storage::*};
use futures::FutureExt;
use library_worker_common::now;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;
use worker::*;
use yrs::{
    updates::decoder::Decode, Doc, ReadTxn, StateVector, Transact, Update,
};

const SNAPSHOT: &str = "yjs:snapshot:bytes";
const SNAP_META: &str = "yjs:snapshot:meta";
const UPDATE_META: &str = "yjs:update:meta";
const CHUNK: usize = 96 * 1024;
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SnapshotMeta {
    seq: u64,
    byte_length: usize,
    updated_at: String,
    #[serde(default)]
    chunks: usize,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateMeta {
    next_seq: u64,
    oldest_seq: u64,
}
fn update_key(seq: u64) -> String {
    format!("yjs:update:{seq:012}")
}
fn chunk_key(index: usize) -> String {
    format!("yjs:snapshot:chunk:{index:06}")
}
fn apply(doc: &Doc, bytes: &[u8]) -> Result<()> {
    let update = Update::decode_v1(bytes)
        .map_err(|_| Error::RustError("Invalid Yjs update".into()))?;
    doc.transact_mut()
        .apply_update(update)
        .map_err(|_| Error::RustError("Invalid Yjs update".into()))
}
pub fn valid_update(bytes: &[u8]) -> bool {
    !bytes.is_empty()
        && bytes.len() <= MAX_UPDATE
        && apply(&Doc::with_client_id(1), bytes).is_ok()
}
#[derive(Default)]
pub struct Document {
    doc: Option<Doc>,
    pub behind: bool,
}
impl Document {
    pub fn snapshot(&self) -> Vec<u8> {
        self.doc
            .as_ref()
            .expect("hydrated document")
            .transact()
            .encode_state_as_update_v1(&StateVector::default())
    }
    pub async fn advance(&mut self, storage: &Storage) -> Result<bool> {
        if self.doc.is_some() && !self.behind {
            return Ok(false);
        }
        if self.doc.is_none() {
            // This relay never authors CRDT structures; a fixed local client ID
            // avoids platform randomness without changing incoming client IDs.
            let doc = Doc::with_client_id(1);
            let meta: Option<SnapshotMeta> = storage.get(SNAP_META).await?;
            let mut snapshot = Vec::new();
            if let Some(meta) = meta.filter(|m| m.chunks > 0) {
                for index in 0..meta.chunks {
                    let Some(raw) =
                        get_raw(storage, &chunk_key(index)).await?
                    else {
                        return Err("Missing Yjs snapshot chunk".into());
                    };
                    snapshot.extend(bytes(&raw)?);
                }
                if snapshot.len() != meta.byte_length {
                    return Err("Invalid Yjs snapshot size".into());
                }
            } else if let Some(raw) = get_raw(storage, SNAPSHOT).await? {
                snapshot = bytes(&raw)?;
            }
            if !snapshot.is_empty() {
                apply(&doc, &snapshot)?;
            }
            self.doc = Some(doc);
        }
        match self.catch_up(storage).await {
            Ok(changed) => Ok(changed),
            Err(e) => {
                self.doc = None;
                Err(e)
            }
        }
    }
    async fn catch_up(&mut self, storage: &Storage) -> Result<bool> {
        let started = now();
        let seq = storage
            .get::<SnapshotMeta>(SNAP_META)
            .await?
            .map_or(0, |m| m.seq);
        let mut meta = storage
            .get::<UpdateMeta>(UPDATE_META)
            .await?
            .unwrap_or(UpdateMeta {
                next_seq: seq + 1,
                oldest_seq: seq + 1,
            });
        let mut cursor = meta.oldest_seq;
        let mut batches = 0;
        let mut changed = false;
        self.behind = false;
        while cursor < meta.next_seq {
            let start = update_key(cursor);
            let end = update_key(meta.next_seq);
            let rows = entries(
                storage
                    .list_with_options(
                        ListOptions::new()
                            .start(&start)
                            .end(&end)
                            .limit(64),
                    )
                    .await?,
            )?;
            if rows.is_empty() {
                return Err("Missing Yjs update log".into());
            }
            let mut through = cursor - 1;
            for (key, raw) in rows {
                let Some(seq) = key
                    .strip_prefix("yjs:update:")
                    .and_then(|s| s.parse::<u64>().ok())
                else {
                    continue;
                };
                if seq >= meta.next_seq {
                    continue;
                }
                // Match legacy behavior for individual malformed update rows.
                if bytes(&raw)
                    .and_then(|b| {
                        apply(self.doc.as_ref().expect("hydrated"), &b)
                    })
                    .is_ok()
                {
                    changed = true;
                } else {
                    console_warn!("{{\"event\":\"corrupt_yjs_update\",\"sequence\":{}}}", seq);
                }
                through = through.max(seq);
            }
            if through < cursor {
                return Err("Invalid Yjs update log".into());
            }
            cursor = through + 1;
            batches += 1;
            // Bound CPU and row work even when the clock does not advance
            // during synchronous Wasm execution.
            let spent = now() - started >= 3000 || batches >= 8;
            if cursor >= meta.next_seq || spent || batches >= 8 {
                meta = self.compact(storage, through).await?;
                cursor = cursor.max(meta.oldest_seq);
                batches = 0;
            }
            if spent && cursor < meta.next_seq {
                self.behind = true;
                break;
            }
        }
        if self.behind {
            crate::tickets::schedule(storage, now() + 1000).await?;
        }
        Ok(changed)
    }
    pub async fn append(
        &mut self,
        storage: &Storage,
        update: &[u8],
        extra: Vec<(String, JsValue)>,
        delete: Vec<String>,
    ) -> Result<bool> {
        if !valid_update(update) {
            return Ok(false);
        }
        self.advance(storage).await?;
        apply(self.doc.as_ref().expect("hydrated"), update)?;
        let data = binary(update);
        let result = transaction(storage, move |tx| {
            async move {
                let mut meta = tx_get::<UpdateMeta>(&tx, UPDATE_META)
                    .await?
                    .unwrap_or(UpdateMeta {
                        next_seq: 1,
                        oldest_seq: 1,
                    });
                let seq = meta.next_seq;
                meta.next_seq = seq
                    .checked_add(1)
                    .ok_or("Yjs sequence limit reached")?;
                meta.oldest_seq = meta.oldest_seq.min(seq);
                let mut rows = extra;
                rows.push((update_key(seq), data));
                rows.push((UPDATE_META.into(), to_js(&meta)?));
                tx_batch(&tx, rows).await?;
                tx_delete(&tx, delete).await?;
                Ok((meta, seq))
            }
            .boxed_local()
        })
        .await;
        let (meta, seq) = match result {
            Ok(v) => v,
            Err(e) => {
                self.doc = None;
                return Err(e);
            }
        };
        if !self.behind && meta.next_seq - meta.oldest_seq > 50 {
            self.compact(storage, seq).await?;
        }
        Ok(true)
    }
    async fn compact(
        &self,
        storage: &Storage,
        through: u64,
    ) -> Result<UpdateMeta> {
        let snapshot = self.snapshot();
        let chunk_count = snapshot.len().div_ceil(CHUNK);
        let meta = SnapshotMeta {
            seq: through,
            byte_length: snapshot.len(),
            updated_at: js_sys::Date::new_0().to_iso_string().into(),
            chunks: chunk_count,
        };
        transaction(storage, move |tx| {
            async move {
                let previous: Option<SnapshotMeta> =
                    tx_get(&tx, SNAP_META).await?;
                let mut updates = tx_get::<UpdateMeta>(&tx, UPDATE_META)
                    .await?
                    .unwrap_or(UpdateMeta {
                        next_seq: through + 1,
                        oldest_seq: 1,
                    });
                let mut deleted = vec![SNAPSHOT.into()];
                for index in chunk_count..previous.map_or(0, |p| p.chunks) {
                    deleted.push(chunk_key(index));
                }
                for seq in updates.oldest_seq..=through {
                    deleted.push(update_key(seq));
                }
                updates.oldest_seq = through + 1;
                updates.next_seq = updates.next_seq.max(updates.oldest_seq);
                let mut rows: Vec<_> = snapshot
                    .chunks(CHUNK)
                    .enumerate()
                    .map(|(i, b)| (chunk_key(i), binary(b)))
                    .collect();
                rows.push((SNAP_META.into(), to_js(&meta)?));
                rows.push((UPDATE_META.into(), to_js(&updates)?));
                tx_batch(&tx, rows).await?;
                tx_delete(&tx, deleted).await?;
                Ok(updates)
            }
            .boxed_local()
        })
        .await
    }
}
