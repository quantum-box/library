//! Durable checkpoint journal. Caller holds the room's storage lock, except
//! while making authorization/CAS requests. Checkpoint requests have a separate
//! queue so Yjs edits and presence remain responsive during network I/O.
use crate::{model::*, storage::*};
use futures::FutureExt;
use library_worker_common::{hash, now};
use worker::*;

pub async fn metadata(storage: &Storage) -> Result<Option<Metadata>> {
    Ok(get_raw(storage, META)
        .await?
        .and_then(|v| serde_wasm_bindgen::from_value::<Metadata>(v).ok())
        .filter(Metadata::valid)
        .map(Metadata::normalize))
}
pub async fn put_metadata(
    storage: &Storage,
    meta: &Metadata,
) -> Result<()> {
    storage.put_raw(META, to_js(meta)?).await
}
pub async fn pending(storage: &Storage) -> Result<Option<Pending>> {
    let Some(raw) = get_raw(storage, PENDING).await? else {
        return Ok(None);
    };
    let value = serde_wasm_bindgen::from_value::<Pending>(raw)
        .ok()
        .filter(Pending::valid)
        .ok_or("Live checkpoint recovery failed")?;
    Ok(Some(value))
}
pub async fn read_body(
    storage: &Storage,
    pending: &Pending,
) -> Result<String> {
    let mut buffer = Vec::with_capacity(pending.body_byte_length);
    for i in 0..pending.chunk_count {
        let chunk = bytes(
            &get_raw(storage, &body_key(i))
                .await?
                .ok_or("Missing checkpoint chunk")?,
        )?;
        if chunk.len() > BODY_CHUNK
            || buffer.len() + chunk.len() > pending.body_byte_length
        {
            return Err("Invalid checkpoint body size".into());
        }
        buffer.extend(chunk);
    }
    if buffer.len() != pending.body_byte_length {
        return Err("Incomplete checkpoint body".into());
    }
    let body = String::from_utf8(buffer)
        .map_err(|_| Error::RustError("Invalid checkpoint text".into()))?;
    if hash(&body) != pending.fingerprint {
        return Err("Invalid checkpoint fingerprint".into());
    }
    Ok(body)
}
pub async fn reserve(
    storage: &Storage,
    pending: Pending,
    body: String,
) -> Result<()> {
    transaction(storage, move |tx| {
        async move {
            let mut rows = vec![(PENDING.into(), to_js(&pending)?)];
            rows.extend(
                body.as_bytes()
                    .chunks(BODY_CHUNK)
                    .enumerate()
                    .map(|(i, b)| (body_key(i), binary(b))),
            );
            tx_batch(&tx, rows).await
        }
        .boxed_local()
    })
    .await
}
pub async fn delete_pending(
    storage: &Storage,
    expected: &Pending,
) -> Result<bool> {
    let expected = expected.clone();
    transaction(storage, move |tx| {
        async move {
            let current: Option<Pending> = tx_get(&tx, PENDING).await?;
            if current.as_ref() != Some(&expected) {
                return Ok(false);
            }
            let mut keys =
                (0..expected.chunk_count).map(body_key).collect::<Vec<_>>();
            keys.push(PENDING.into());
            tx_delete(&tx, keys).await?;
            Ok(true)
        }
        .boxed_local()
    })
    .await
}
fn valid_entry(entry: &ResultEntry) -> bool {
    bounded(&entry.operation_id, 512)
        && entry.result_key == result_key(&entry.operation_id)
        && entry.expires_at > 0
}
async fn index(tx: &Transaction) -> Result<Vec<ResultEntry>> {
    Ok(tx_raw(tx, RESULT_INDEX)
        .await?
        .and_then(|raw| {
            serde_wasm_bindgen::from_value::<Vec<ResultEntry>>(raw).ok()
        })
        .unwrap_or_default()
        .into_iter()
        .filter(valid_entry)
        .collect())
}
pub async fn save_result(
    storage: &Storage,
    mut result: Saved,
) -> Result<()> {
    if result.expires_at <= now() {
        result.expires_at = now() + RESULT_TTL
    }
    transaction(storage, move |tx| {
        async move {
            let pending: Option<Pending> = tx_get(&tx, PENDING).await?;
            let pinned = pending.as_ref().map(|p| p.operation_id.as_str());
            let key = result_key(&result.operation_id);
            let mut retained = Vec::new();
            let mut remove = Vec::new();
            for entry in index(&tx).await? {
                if entry.result_key == key
                    || entry.operation_id == result.operation_id
                {
                    continue;
                }
                if entry.expires_at <= now()
                    && Some(entry.operation_id.as_str()) != pinned
                {
                    remove.push(entry.result_key)
                } else {
                    retained.push(entry)
                }
            }
            retained.push(ResultEntry {
                operation_id: result.operation_id.clone(),
                result_key: key.clone(),
                expires_at: result.expires_at,
            });
            while retained.len() > 128 {
                let Some(i) = retained
                    .iter()
                    .position(|e| Some(e.operation_id.as_str()) != pinned)
                else {
                    break;
                };
                let removed = retained.remove(i);
                if removed.result_key != key {
                    remove.push(removed.result_key)
                }
            }
            tx_delete(&tx, remove).await?;
            tx_put(&tx, &key, &result).await?;
            tx_put(&tx, RESULT_INDEX, &retained).await
        }
        .boxed_local()
    })
    .await?;
    crate::tickets::schedule(storage, now() + RESULT_TTL).await
}
async fn delete_result(storage: &Storage, id: &str) -> Result<()> {
    let id = id.to_owned();
    transaction(storage, move |tx| {
        async move {
            let key = result_key(&id);
            tx.delete(&key).await?;
            let entries = index(&tx)
                .await?
                .into_iter()
                .filter(|e| e.operation_id != id && e.result_key != key)
                .collect::<Vec<_>>();
            tx_put(&tx, RESULT_INDEX, &entries).await
        }
        .boxed_local()
    })
    .await
}
pub async fn read_result(
    storage: &Storage,
    id: &str,
) -> Result<Option<Saved>> {
    let Some(raw) = get_raw(storage, &result_key(id)).await? else {
        return Ok(None);
    };
    let Some(mut result) = serde_wasm_bindgen::from_value::<Saved>(raw)
        .ok()
        .filter(Saved::valid)
    else {
        delete_result(storage, id).await?;
        return Ok(None);
    };
    if result.operation_id != id {
        delete_result(storage, id).await?;
        return Ok(None);
    }
    let pinned = pending(storage)
        .await?
        .is_some_and(|p| p.operation_id == id);
    if result.expires_at == 0 || (result.expires_at <= now() && pinned) {
        result.expires_at = now() + RESULT_TTL;
        save_result(storage, result.clone()).await?;
    } else if result.expires_at <= now() {
        delete_result(storage, id).await?;
        return Ok(None);
    }
    Ok(Some(result))
}
pub async fn cleanup(storage: &Storage) -> Result<Option<i64>> {
    // A bounded prefix cursor also adopts result keys written before the index.
    let cursor: Option<String> = storage.get(RESULT_CURSOR).await?;
    let start = cursor.as_ref().map(|s| format!("{s}\0"));
    let mut options = ListOptions::new().prefix(RESULT_PREFIX).limit(128);
    if let Some(start) = &start {
        options = options.start(start)
    }
    let scanned = entries(storage.list_with_options(options).await?)?;
    let pinned = pending(storage).await?.map(|p| p.operation_id);
    let mut records = Vec::new();
    let mut remove = Vec::new();
    for (key, raw) in &scanned {
        let result = serde_wasm_bindgen::from_value::<Saved>(raw.clone())
            .ok()
            .filter(|s| s.valid() && result_key(&s.operation_id) == *key);
        match result {
            Some(mut result) => {
                if result.expires_at == 0
                    || (result.expires_at <= now()
                        && Some(&result.operation_id) == pinned.as_ref())
                {
                    result.expires_at = now() + RESULT_TTL;
                    storage.put_raw(key, to_js(&result)?).await?;
                }
                if result.expires_at <= now() {
                    remove.push(key.clone())
                } else {
                    records.push(ResultEntry {
                        operation_id: result.operation_id,
                        result_key: key.clone(),
                        expires_at: result.expires_at,
                    })
                }
            }
            None => remove.push(key.clone()),
        }
    }
    let next = transaction(storage, move |tx| {
        async move {
            let mut retained = index(&tx).await?;
            for entry in records {
                retained.retain(|v| v.result_key != entry.result_key);
                retained.push(entry);
            }
            for entry in &mut retained {
                if entry.expires_at <= now()
                    && Some(&entry.operation_id) == pinned.as_ref()
                {
                    entry.expires_at = now() + RESULT_TTL;
                    if let Some(mut record) =
                        tx_get::<Saved>(&tx, &entry.result_key).await?
                    {
                        record.expires_at = entry.expires_at;
                        tx_put(&tx, &entry.result_key, &record).await?;
                    }
                }
                if entry.expires_at <= now() {
                    remove.push(entry.result_key.clone())
                }
            }
            retained.retain(|e| !remove.contains(&e.result_key));
            while retained.len() > 128 {
                let Some(i) = retained
                    .iter()
                    .position(|e| Some(&e.operation_id) != pinned.as_ref())
                else {
                    break;
                };
                remove.push(retained.remove(i).result_key);
            }
            tx_delete(&tx, remove).await?;
            tx_put(&tx, RESULT_INDEX, &retained).await?;
            Ok(retained.iter().map(|e| e.expires_at).min())
        }
        .boxed_local()
    })
    .await?;
    if scanned.len() == 128 {
        storage
            .put(RESULT_CURSOR, &scanned.last().expect("nonempty").0)
            .await?;
        Ok(Some(next.unwrap_or(now() + 1000).min(now() + 1000)))
    } else {
        if cursor.is_some() {
            storage.delete(RESULT_CURSOR).await?;
        }
        Ok(next)
    }
}
