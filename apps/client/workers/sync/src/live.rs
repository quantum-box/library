use crate::{
    auth, checkpoint as journal,
    document::{valid_update, Document},
    model::*,
    relay,
    storage::*,
    tickets,
};
use futures::{lock::Mutex, FutureExt};
use library_worker_common::{
    api_request, fetch_timeout, hash, header, json, now, request_json,
    response_json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json as value, Value};
use std::{cell::RefCell, collections::HashMap};
use worker::*;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Initialization {
    #[serde(with = "serde_wasm_bindgen::preserve")]
    update: js_sys::ArrayBuffer,
    record_version: String,
}

/// Browsers only see a failed upgrade as close 1006, so every refused join is
/// logged for `wrangler tail` with its status and a fixed reason. Never log
/// tickets, session ids, bodies or authorization values here.
fn refuse(status: u16, reason: &str, message: &str) -> Result<Response> {
    console_warn!(
        "{{\"event\":\"live_join_refused\",\"status\":{status},\"reason\":\"{reason}\"}}"
    );
    Response::error(message, status)
}
/// A 409 naming another generation is a hop the edge follows, not a refusal.
fn redirect(
    reason: &str,
    message: &str,
    generation: &str,
) -> Result<Response> {
    console_log!(
        "{{\"event\":\"live_join_redirected\",\"reason\":\"{reason}\"}}"
    );
    let mut response = Response::error(message, 409)?;
    response.headers_mut().set(GENERATION, generation)?;
    Ok(response)
}
/// Why a session may not join, follow or replace a room state holding
/// `body_hash` at `record_version`: a stale ticket must never roll a room
/// back, and two bodies at one version cannot be ordered. `None` when the
/// session holds the same body or a strictly newer version.
fn refusal(
    session: &Session,
    record_version: &str,
    body_hash: &str,
) -> Option<&'static str> {
    if session.body_hash == body_hash
        || newer(&session.record_version, Some(record_version))
    {
        None
    } else if newer(record_version, Some(&session.record_version)) {
        Some("stale_ticket")
    } else {
        Some("unordered_version")
    }
}
/// Transient API answers. 401 recovers as well: the socket closes at its
/// session expiry and the reconnect presents a fresh credential. Other 4xx
/// (invalid body, permission, missing record, rejected patch) repeat for the
/// same operation id, so they stay terminal instead of retrying forever.
fn retryable(status: u16) -> bool {
    matches!(status, 401 | 408 | 429 | 500..=599)
}
/// A checkpoint whose outcome is unknown or transiently failed. The client
/// resends the identical frame; see `CHECKPOINT_RETRY`.
fn retry(
    sender: &WebSocket,
    message: &str,
    id: Option<&str>,
    reason: &str,
    status: Option<u16>,
) {
    let status = status.map_or_else(|| "null".into(), |s| s.to_string());
    console_warn!(
        "{{\"event\":\"live_checkpoint_retry\",\"reason\":\"{reason}\",\"status\":{status}}}"
    );
    relay::error(sender, message, id, Some(CHECKPOINT_RETRY));
}

#[durable_object]
pub struct PhotonLiveRoom {
    state: State,
    env: Env,
    doc: Mutex<Document>,
    checkpoints: Mutex<()>,
    sessions: RefCell<HashMap<String, Session>>,
}
impl PhotonLiveRoom {
    fn generation_id(&self) -> String {
        self.state.id().to_string()
    }
    async fn resolve_reference(
        &self,
        request: &Request,
    ) -> Result<Option<Session>> {
        let reference = header(request, SESSION)
            .filter(|s| s.len() <= 32 * 1024)
            .and_then(|s| decode64(&s, 32 * 1024))
            .and_then(|b| serde_json::from_slice::<Session>(&b).ok())
            .filter(|s| s.valid(true) && s.expires_at > now());
        let Some(reference) = reference else {
            return Ok(None);
        };
        let session =
            tickets::session(&self.env, &reference.session_id).await?;
        Ok(session.filter(|s| s.reference() == reference.reference()))
    }
    async fn resolve_socket(&self, socket: &WebSocket) -> Option<Session> {
        let attachment = socket
            .deserialize_attachment::<Attachment>()
            .ok()
            .flatten()?;
        if attachment.kind != "photon-live"
            || attachment.expires_at <= now()
        {
            return None;
        }
        if let Some(cached) = self
            .sessions
            .borrow()
            .get(&attachment.session_id)
            .filter(|s| {
                s.expires_at == attachment.expires_at
                    && s.expires_at > now()
            })
            .cloned()
        {
            return Some(cached);
        }
        let session = tickets::session(&self.env, &attachment.session_id)
            .await
            .ok()
            .flatten()
            .filter(|s| s.expires_at == attachment.expires_at)?;
        self.sessions
            .borrow_mut()
            .insert(session.session_id.clone(), session.clone());
        Some(session)
    }
    async fn accept_pointer(
        &self,
        session: &Session,
        generation: String,
    ) -> Result<String> {
        let session = session.clone();
        transaction(&self.state.storage(), move |tx| {
            async move {
                let existing: Option<Pointer> =
                    tx_get(&tx, POINTER).await?;
                if let Some(existing) = existing.filter(Pointer::valid) {
                    if !newer(
                        &session.record_version,
                        Some(&existing.record_version),
                    ) && (session.record_version
                        != existing.record_version
                        || existing.body_hash != session.body_hash)
                    {
                        return Ok(existing.room_id);
                    }
                }
                tx_put(
                    &tx,
                    POINTER,
                    &Pointer {
                        room_id: generation.clone(),
                        body_hash: session.body_hash,
                        record_version: session.record_version,
                    },
                )
                .await?;
                Ok(generation)
            }
            .boxed_local()
        })
        .await
    }
    /// Commits a generation for the session's body in the base room's pointer.
    /// Returns the room the pointer names afterwards (a newer session may
    /// already have moved it further), or the refusal when nothing committed.
    async fn rotate(
        &self,
        session: &Session,
        target: &str,
    ) -> Result<std::result::Result<String, Response>> {
        let generation = generation(session);
        if !generated(&generation) {
            return refuse(
                409,
                "invalid_generation",
                "Invalid room generation",
            )
            .map(Err);
        }
        if target == session.identity.room_id {
            let _guard = self.doc.lock().await;
            return self.accept_pointer(session, generation).await.map(Ok);
        }
        let headers = Headers::new();
        headers.set(INTERNAL, "1")?;
        headers.set(SESSION, &session.header())?;
        let request = api_request(
            "https://live.internal/live/internal-pointer",
            Method::Post,
            headers,
            Some(
                &value!({"room_id":generation,"body_hash":session.body_hash,"record_version":session.record_version}),
            ),
        )?;
        let mut response = self
            .env
            .durable_object("PHOTON_LIVE_ROOMS")?
            .get_by_name(&session.identity.room_id)?
            .fetch_with_request(request)
            .await?;
        if response.status_code() != 200 {
            return refuse(
                503,
                "pointer_unavailable",
                "Live room generation unavailable",
            )
            .map(Err);
        }
        let pointer: Pointer =
            response_json(&mut response, 64 * 1024).await?;
        if !pointer.valid() {
            return refuse(
                503,
                "pointer_invalid",
                "Live room generation unavailable",
            )
            .map(Err);
        }
        Ok(Ok(pointer.room_id))
    }
    /// Replaces this room with a generation seeded from the session's newer
    /// canonical body. Dirty rooms rotate too, otherwise one external write
    /// makes the room unjoinable forever. Unsaved Yjs edits stay in this
    /// generation's storage and are never merged into the successor; every
    /// connected client also still holds them in its own Y.Doc when the 4410
    /// close tells it the canonical body changed.
    async fn replace(
        &self,
        session: &Session,
        target: &str,
    ) -> Result<Response> {
        let accepted = match self.rotate(session, target).await? {
            Ok(accepted) => accepted,
            Err(refusal) => return Ok(refusal),
        };
        if accepted == target {
            // The pointer names this very room for the session's body, so
            // its metadata disagrees with the state it was named for. Only a
            // worker from before 2026-09-23 seeded a generation from a stale
            // ticket; the next canonical write moves the pointer past it.
            return refuse(409, "generation_mismatch", "Live body changed");
        }
        // Peers are closed only after the pointer names another room. When
        // the rotation fails they keep editing here instead of reconnecting
        // into the same refusal.
        self.retire(session, target, &accepted).await;
        redirect("body_changed", "Live body changed", &accepted)
    }
    async fn retire(
        &self,
        session: &Session,
        target: &str,
        successor: &str,
    ) {
        // Under the room lock: a join that read the old pointer either sees
        // the marker or is already accepted and closed below.
        let _guard = self.doc.lock().await;
        // The base room's pointer already redirects later joins.
        if target != session.identity.room_id
            && self.state.storage().put(RETIRED, successor).await.is_err()
        {
            console_warn!(
                "{{\"event\":\"live_generation_retire_failed\"}}"
            );
        }
        for ws in self.state.get_websockets() {
            let _ = ws.close(
                Some(4410),
                Some("Live canonical body changed; reconnect required"),
            );
        }
    }
    async fn recover(
        &self,
        doc: &mut Document,
        mut meta: Metadata,
    ) -> Result<Metadata> {
        let storage = self.state.storage();
        let Some(raw) = get_raw(&storage, INIT).await? else {
            return Ok(meta);
        };
        if meta.initialized {
            storage.delete(INIT).await?;
            return Ok(meta);
        }
        let init: Initialization = serde_wasm_bindgen::from_value(raw)?;
        let update = bytes(&init.update.into())?;
        if !bounded(&init.record_version, 512) || !valid_update(&update) {
            return Err("Invalid pending initialization".into());
        }
        meta.initialized = true;
        meta.version = 0;
        meta.saved_version = Some(0);
        meta.record_version = Some(init.record_version);
        if !doc
            .append(
                &storage,
                &update,
                vec![(META.into(), to_js(&meta)?)],
                vec![INIT.into()],
            )
            .await?
        {
            return Err("Live initialization failed".into());
        }
        relay::binary(&self.state, &update, None);
        Ok(meta)
    }
    async fn initialize(
        &self,
        sender: &WebSocket,
        session: &Session,
        update: Vec<u8>,
    ) -> Result<()> {
        let mut doc = self.doc.lock().await;
        let storage = self.state.storage();
        let Some(meta) = journal::metadata(&storage)
            .await?
            .filter(|m| m.identity == session.identity)
        else {
            relay::error(sender, "Live room identity mismatch", None, None);
            return Ok(());
        };
        let meta = self.recover(&mut doc, meta).await?;
        if meta.initialized {
            relay::send(
                sender,
                &meta.ready(session, &self.generation_id()),
            );
            return Ok(());
        }
        let pending = Initialization {
            update: js_sys::Uint8Array::from(update.as_slice()).buffer(),
            record_version: session.record_version.clone(),
        };
        storage.put_raw(INIT, to_js(&pending)?).await?;
        let initialized = self.recover(&mut doc, meta).await?;
        relay::broadcast(
            &self.state,
            &initialized.ready(session, &self.generation_id()),
        );
        Ok(())
    }
    async fn update(
        &self,
        sender: &WebSocket,
        session: &Session,
        update: Vec<u8>,
    ) -> Result<()> {
        if !valid_update(&update) {
            relay::error(
                sender,
                "Invalid or oversized Live update",
                None,
                None,
            );
            return Ok(());
        }
        let mut doc = self.doc.lock().await;
        let storage = self.state.storage();
        let Some(mut meta) = journal::metadata(&storage)
            .await?
            .filter(|m| m.identity == session.identity)
        else {
            relay::error(sender, "Live room identity mismatch", None, None);
            return Ok(());
        };
        if !meta.initialized {
            relay::error(
                sender,
                "Live document is not initialized",
                None,
                None,
            );
            return Ok(());
        }
        if meta.version >= MAX_SAFE {
            relay::error(sender, "Live version limit reached", None, None);
            return Ok(());
        }
        meta.version += 1;
        if doc.advance(&storage).await? {
            relay::binary(&self.state, &doc.snapshot(), None)
        }
        if !doc
            .append(
                &storage,
                &update,
                vec![(META.into(), to_js(&meta)?)],
                vec![],
            )
            .await?
        {
            return Err("Live update failed".into());
        }
        relay::binary(&self.state, &update, Some(sender));
        relay::broadcast(
            &self.state,
            &value!({"type":"live-version","version":meta.version}),
        );
        Ok(())
    }
    async fn replay(
        &self,
        sender: &WebSocket,
        session: &Session,
        version: u64,
        fingerprint: &str,
        id: &str,
        result: Saved,
    ) -> Result<()> {
        let storage = self.state.storage();
        let Some(mut meta) = journal::metadata(&storage)
            .await?
            .filter(|m| m.identity == session.identity)
        else {
            relay::error(
                sender,
                "Live room identity mismatch",
                Some(id),
                None,
            );
            return Ok(());
        };
        let pending = journal::pending(&storage).await?;
        if result.fingerprint != fingerprint
            || result.version != version
            || pending.as_ref().is_some_and(|p| {
                p.operation_id == id && p.fingerprint != fingerprint
            })
        {
            relay::error(
                sender,
                "Checkpoint operation was reused",
                Some(id),
                None,
            );
            return Ok(());
        }
        if newer(&result.record_version, meta.record_version.as_deref()) {
            meta.record_version = Some(result.record_version.clone());
            meta.body_hash = result.body_hash.clone();
            meta.saved_version =
                Some(meta.saved().max(result.version.min(meta.version)));
            journal::put_metadata(&storage, &meta).await?;
        }
        if let Some(p) = pending.filter(|p| p.operation_id == id) {
            journal::delete_pending(&storage, &p).await?;
        }
        relay::send(sender, &result.frame());
        Ok(())
    }
    async fn prepare(
        &self,
        sender: &WebSocket,
        session: &Session,
        version: u64,
        body: &str,
        id: &str,
        authorized: Option<&Authorization>,
    ) -> Result<Option<(Pending, String)>> {
        let storage = self.state.storage();
        let fingerprint = hash(body);
        let Some(mut meta) = journal::metadata(&storage)
            .await?
            .filter(|m| m.identity == session.identity)
        else {
            relay::error(
                sender,
                "Live room identity mismatch",
                Some(id),
                None,
            );
            return Ok(None);
        };
        if !meta.initialized {
            relay::error(
                sender,
                "Live document is not initialized",
                Some(id),
                None,
            );
            return Ok(None);
        }
        let pending = journal::pending(&storage).await?;
        if let Some(result) = journal::read_result(&storage, id).await? {
            self.replay(sender, session, version, &fingerprint, id, result)
                .await?;
            return Ok(None);
        }
        if let Some(p) = pending.as_ref().filter(|p| p.operation_id == id) {
            if p.version != version || p.fingerprint != fingerprint {
                relay::error(
                    sender,
                    "Checkpoint operation was reused",
                    Some(id),
                    None,
                );
                return Ok(None);
            }
            let body = journal::read_body(&storage, p).await?;
            // This resend can commit as late as the first request could.
            let p = journal::mark(
                &storage,
                p,
                Some(now() + CHECKPOINT_IN_DOUBT),
            )
            .await?
            .ok_or("Live checkpoint recovery failed")?;
            return Ok(Some((p, body)));
        }
        let authorized =
            authorized.ok_or("Live checkpoint recovery failed")?;
        if authorized.identity != session.identity {
            return Err("Live checkpoint recovery failed".into());
        }
        let canonical_hash =
            body_hash(&authorized.identity.format, &authorized.body);
        let proposed_hash = body_hash(&meta.identity.format, body);
        if let Some(p) = pending {
            // A replacement must describe the current working document. Never
            // erase the crash journal on a stale request.
            if version != meta.version {
                relay::error(
                    sender,
                    "Checkpoint is behind the working version",
                    Some(id),
                    (version < meta.version).then_some(CHECKPOINT_STALE),
                );
                return Ok(None);
            }
            if authorized.record_version == p.expected_record_version {
                // An unchanged version proves the reservation never committed
                // only once no request for it can still be running at the
                // API. Until then its owner's identical retry settles it
                // through the API's idempotency; replacing it here could
                // discard a save that commits a moment later.
                if p.in_doubt() {
                    retry(
                        sender,
                        "Another checkpoint is still being confirmed",
                        Some(id),
                        "reservation_in_doubt",
                        None,
                    );
                    return Ok(None);
                }
                if canonical_hash != meta.body_hash {
                    journal::delete_pending(&storage, &p).await?;
                    self.conflict(id);
                    return Ok(None);
                }
                journal::delete_pending(&storage, &p).await?;
                if newer(
                    &authorized.record_version,
                    meta.record_version.as_deref(),
                ) {
                    meta.record_version =
                        Some(authorized.record_version.clone())
                }
                meta.body_hash = canonical_hash;
                journal::put_metadata(&storage, &meta).await?;
            } else if journal::committed(
                &p,
                &canonical_hash,
                &authorized.record_version,
            ) {
                // The reservation committed but its ACK was lost. Its owner's
                // retry replays this result; the replacement continues
                // against the committed body.
                let saved = journal::settle(
                    &storage,
                    &mut meta,
                    &p,
                    &authorized.record_version,
                )
                .await?;
                relay::broadcast(&self.state, &saved.frame());
            } else if canonical_hash == meta.body_hash
                && newer(
                    &authorized.record_version,
                    Some(&p.expected_record_version),
                )
            {
                // Only the record version moved (a title or property save):
                // the body the reservation was based on is still canonical,
                // so nothing conflicts. The reservation can never commit
                // against its old version; its owner resends under a new
                // operation id and this one continues at the new version.
                journal::delete_pending(&storage, &p).await?;
                relay::broadcast(
                    &self.state,
                    &relay::error_frame(
                        "Record version changed; resend the checkpoint",
                        Some(&p.operation_id),
                        Some(CHECKPOINT_STALE),
                    ),
                );
                if newer(
                    &authorized.record_version,
                    meta.record_version.as_deref(),
                ) {
                    meta.record_version =
                        Some(authorized.record_version.clone())
                }
                journal::put_metadata(&storage, &meta).await?;
            } else {
                journal::delete_pending(&storage, &p).await?;
                self.conflict(id);
                return Ok(None);
            }
        } else {
            if canonical_hash != meta.body_hash {
                // This exact body is already canonical at a newer version:
                // its own CAS committed after the reservation was replaced,
                // or another writer stored the same body. Adopt it; it is
                // reported saved below without a CAS.
                if canonical_hash != proposed_hash
                    || !newer(
                        &authorized.record_version,
                        meta.record_version.as_deref(),
                    )
                {
                    self.conflict(id);
                    return Ok(None);
                }
                meta.body_hash = canonical_hash;
            }
            if newer(
                &authorized.record_version,
                meta.record_version.as_deref(),
            ) {
                meta.record_version =
                    Some(authorized.record_version.clone())
            }
            journal::put_metadata(&storage, &meta).await?;
        }
        let Some(record_version) = meta.record_version.clone() else {
            relay::error(
                sender,
                "Live document is not initialized",
                Some(id),
                None,
            );
            return Ok(None);
        };
        if version != meta.version {
            relay::error(
                sender,
                "Checkpoint is behind the working version",
                Some(id),
                (version < meta.version).then_some(CHECKPOINT_STALE),
            );
            return Ok(None);
        }
        if proposed_hash == meta.body_hash {
            meta.saved_version = Some(meta.version);
            journal::put_metadata(&storage, &meta).await?;
            let saved = Saved {
                version: meta.version,
                operation_id: id.into(),
                record_version,
                body_hash: meta.body_hash,
                fingerprint,
                expires_at: now() + RESULT_TTL,
            };
            journal::save_result(&storage, saved.clone()).await?;
            relay::send(sender, &saved.frame());
            return Ok(None);
        }
        let pending = Pending {
            version,
            operation_id: id.into(),
            expected_record_version: record_version,
            body_hash: proposed_hash,
            fingerprint,
            body_byte_length: body.len(),
            chunk_count: body.len().div_ceil(BODY_CHUNK),
            in_doubt_until: Some(now() + CHECKPOINT_IN_DOUBT),
        };
        journal::reserve(&storage, pending.clone(), body.into()).await?;
        Ok(Some((pending, body.into())))
    }
    fn conflict(&self, id: &str) {
        relay::broadcast(
            &self.state,
            &value!({"type":"live-conflict","operation_id":id}),
        );
    }
    /// The API refused the CAS. That is not always a conflict: another writer
    /// (e.g. the ordinary save fallback) may have stored the same body first,
    /// or the API no longer had this operation's record, and a save that
    /// left the body alone moves only the record version. Settle or rebase
    /// those instead of reporting a conflict the participants cannot resolve.
    async fn conflicted(
        &self,
        sender: &WebSocket,
        session: &Session,
        id: &str,
        pending: &Pending,
    ) -> Result<()> {
        // No document/storage lock crosses a network boundary.
        let current = auth::current(&self.env, session).await;
        let storage = self.state.storage();
        let _guard = self.doc.lock().await;
        if let Some(result) = journal::read_result(&storage, id).await? {
            // A join settled this reservation while the request was in flight.
            relay::send(sender, &result.frame());
            return Ok(());
        }
        let Some(current) = current else {
            // A lost ACK cannot be ruled out without the canonical body; keep
            // the reservation rather than report a conflict that may be false.
            // The API has decided this operation, so it is no longer in doubt.
            journal::mark(&storage, pending, None).await?;
            retry(
                sender,
                "Live checkpoint failed",
                Some(id),
                "conflict_unverified",
                Some(409),
            );
            return Ok(());
        };
        let Some(mut meta) = journal::metadata(&storage)
            .await?
            .filter(|m| m.identity == session.identity)
        else {
            return Ok(());
        };
        if let Some(p) =
            journal::pending(&storage).await?.filter(|p| p == pending)
        {
            let canonical_hash =
                body_hash(&current.identity.format, &current.body);
            if journal::committed(
                &p,
                &canonical_hash,
                &current.record_version,
            ) {
                let saved = journal::settle(
                    &storage,
                    &mut meta,
                    &p,
                    &current.record_version,
                )
                .await?;
                relay::broadcast(&self.state, &saved.frame());
                return Ok(());
            }
            journal::delete_pending(&storage, &p).await?;
            if canonical_hash == meta.body_hash
                && newer(
                    &current.record_version,
                    Some(&p.expected_record_version),
                )
            {
                // Only the record version moved (a title or property save)
                // while this request was out; the body it was based on is
                // still canonical. The API recorded a conflict for this
                // operation id, so the client resends its newest body under a
                // new id against the adopted version.
                if newer(
                    &current.record_version,
                    meta.record_version.as_deref(),
                ) {
                    meta.record_version = Some(current.record_version);
                }
                journal::put_metadata(&storage, &meta).await?;
                relay::error(
                    sender,
                    "Record version changed; resend the checkpoint",
                    Some(id),
                    Some(CHECKPOINT_STALE),
                );
                return Ok(());
            }
        }
        self.conflict(id);
        Ok(())
    }
    async fn checkpoint(
        &self,
        sender: &WebSocket,
        session: &Session,
        frame: &Value,
    ) -> Result<()> {
        let id = field(frame, "operation_id");
        let version = frame["version"].as_u64().filter(|v| *v <= MAX_SAFE);
        let body = frame["body"].as_str().filter(|b| b.len() <= MAX_BODY);
        let (Some(id), Some(version), Some(body)) =
            (id.as_deref(), version, body)
        else {
            relay::error(sender, "Invalid checkpoint", id.as_deref(), None);
            return Ok(());
        };
        let _queue = self.checkpoints.lock().await;
        if session.expires_at <= now() {
            let _ = sender.close(Some(4401), Some("Live session expired"));
            return Ok(());
        }
        let storage = self.state.storage();
        let needs_authorization = {
            let _guard = self.doc.lock().await;
            let Some(meta) = journal::metadata(&storage)
                .await?
                .filter(|m| m.identity == session.identity)
            else {
                relay::error(
                    sender,
                    "Live room identity mismatch",
                    Some(id),
                    None,
                );
                return Ok(());
            };
            if !meta.initialized {
                relay::error(
                    sender,
                    "Live document is not initialized",
                    Some(id),
                    None,
                );
                return Ok(());
            }
            let pending = journal::pending(&storage).await?;
            let result = journal::read_result(&storage, id).await?;
            result.is_none()
                && pending.as_ref().is_none_or(|p| p.operation_id != id)
        };
        // No document/storage lock crosses a network boundary.
        let authorized = if needs_authorization {
            Some(
                auth::current(&self.env, session)
                    .await
                    .ok_or("Live checkpoint recovery failed")?,
            )
        } else {
            None
        };
        let prepared = {
            let _guard = self.doc.lock().await;
            self.prepare(
                sender,
                session,
                version,
                body,
                id,
                authorized.as_ref(),
            )
            .await?
        };
        let Some((pending, body)) = prepared else {
            return Ok(());
        };
        let request = api_request(
            &auth::api_url(
                &self.env,
                &session.org,
                &session.repo,
                &session.identity.data,
                "checkpoint",
            ),
            Method::Post,
            auth::session_headers(session)?,
            Some(
                &value!({"property_id":session.identity.property,"operation_id":id,"expected_record_version":pending.expected_record_version,"format":session.identity.format,"body":body}),
            ),
        )?;
        // From here on the CAS may have committed even when no answer comes
        // back. The reservation is kept on every unknown outcome, in doubt
        // until `CHECKPOINT_IN_DOUBT` has passed so no other operation replaces
        // it meanwhile. The API records each operation id with its decision
        // and replays that decision for an identical request, so resending
        // the same operation (same expected version and body, re-read from
        // this journal) returns the original `record_version`, not a conflict.
        let mut response =
            match fetch_timeout(request, CHECKPOINT_TIMEOUT).await {
                Ok(response) => response,
                Err(_) => {
                    retry(
                        sender,
                        "Live checkpoint failed",
                        Some(id),
                        "timeout_or_network",
                        None,
                    );
                    return Ok(());
                }
            };
        let status = response.status_code();
        if status == 409 {
            return self.conflicted(sender, session, id, &pending).await;
        }
        if !(200..300).contains(&status) {
            // A 5xx can come from a gateway while the API is still running the
            // request, so the reservation stays in doubt. Any other status is
            // the API's own answer, and it applied nothing.
            if !(500..=599).contains(&status) {
                let _guard = self.doc.lock().await;
                if retryable(status) {
                    journal::mark(&storage, &pending, None).await?;
                } else {
                    // The same operation repeats the same refusal: it never
                    // commits, so nothing is left to recover.
                    journal::delete_pending(&storage, &pending).await?;
                }
            }
            if retryable(status) {
                retry(
                    sender,
                    "Live checkpoint failed",
                    Some(id),
                    "api_status",
                    Some(status),
                );
            } else {
                console_warn!(
                    "{{\"event\":\"live_checkpoint_failed\",\"status\":{status}}}"
                );
                relay::error(
                    sender,
                    "Live checkpoint failed",
                    Some(id),
                    None,
                );
            }
            return Ok(());
        }
        let record_version = response_json::<Value>(&mut response, MAX_API)
            .await
            .ok()
            .and_then(|v| canonical(&v, "record_version", "recordVersion"));
        let Some(record_version) = record_version else {
            retry(
                sender,
                "Live checkpoint failed",
                Some(id),
                "missing_record_version",
                Some(status),
            );
            return Ok(());
        };
        let result = Saved {
            version: pending.version,
            operation_id: id.into(),
            record_version,
            body_hash: body_hash(&session.identity.format, &body),
            fingerprint: pending.fingerprint.clone(),
            expires_at: now() + RESULT_TTL,
        };
        let _guard = self.doc.lock().await;
        let Some(mut meta) = journal::metadata(&storage)
            .await?
            .filter(|m| m.identity == session.identity)
        else {
            return Ok(());
        };
        let current_pending = journal::pending(&storage).await?;
        journal::save_result(&storage, result.clone()).await?;
        if newer(&result.record_version, meta.record_version.as_deref())
            || (current_pending.as_ref() == Some(&pending)
                && meta.record_version.as_deref()
                    == Some(&result.record_version))
        {
            meta.record_version = Some(result.record_version.clone());
            meta.body_hash = result.body_hash.clone();
        }
        if current_pending.as_ref() == Some(&pending) {
            meta.saved_version =
                Some(meta.saved().max(pending.version.min(meta.version)));
            journal::delete_pending(&storage, &pending).await?;
        }
        journal::put_metadata(&storage, &meta).await?;
        relay::broadcast(&self.state, &result.frame());
        Ok(())
    }
}

impl DurableObject for PhotonLiveRoom {
    fn new(state: State, env: Env) -> Self {
        Self {
            state,
            env,
            doc: Mutex::new(Document::default()),
            checkpoints: Mutex::new(()),
            sessions: RefCell::new(HashMap::new()),
        }
    }
    async fn fetch(&self, mut request: Request) -> Result<Response> {
        if header(&request, INTERNAL).as_deref() != Some("1") {
            return refuse(404, "not_internal", "Not found");
        }
        let Some(session) = self.resolve_reference(&request).await? else {
            return refuse(401, "unknown_session", "Unauthorized");
        };
        if request.path() == "/live/internal-pointer" {
            if request.method() != Method::Post {
                return refuse(405, "pointer_method", "Method not allowed");
            }
            let input = request_json(&mut request, 64 * 1024).await?;
            let candidate =
                field(&input, "room_id").filter(|s| generated(s));
            if candidate.is_none()
                || input["body_hash"] != session.body_hash
                || input["record_version"] != session.record_version
            {
                return refuse(
                    400,
                    "invalid_pointer",
                    "Invalid room pointer",
                );
            }
            let _guard = self.doc.lock().await;
            let accepted = self
                .accept_pointer(&session, candidate.expect("validated"))
                .await?;
            let pointer: Pointer = self
                .state
                .storage()
                .get(POINTER)
                .await?
                .ok_or("Missing generation pointer")?;
            if accepted != pointer.room_id {
                return refuse(
                    503,
                    "pointer_unavailable",
                    "Live room generation unavailable",
                );
            }
            return json(&serde_json::to_value(pointer)?, 200);
        }
        if !header(&request, "upgrade")
            .is_some_and(|v| v.eq_ignore_ascii_case("websocket"))
        {
            return refuse(
                426,
                "not_websocket",
                "Expected WebSocket upgrade",
            );
        }
        let target = header(&request, TARGET)
            .filter(|s| bounded(s, 512))
            .unwrap_or_else(|| session.identity.room_id.clone());
        let mut doc = self.doc.lock().await;
        let storage = self.state.storage();
        // The base room owns the generation pointer; a superseded generation
        // remembers its successor. The edge follows either hop.
        let successor = if target == session.identity.room_id {
            storage
                .get::<Pointer>(POINTER)
                .await?
                .filter(|p| p.valid() && p.room_id != target)
                .map(|p| p.room_id)
        } else {
            storage
                .get::<String>(RETIRED)
                .await?
                .filter(|s| generated(s) && *s != target)
        };
        if let Some(successor) = successor {
            // The successor would refuse an older ticket; say so here.
            if let Some(reason) = generation_state(&successor)
                .and_then(|(_, v, h)| refusal(&session, &v, &h))
            {
                return refuse(409, reason, "Live body changed");
            }
            return redirect(
                "generation_changed",
                "Live room generation changed",
                &successor,
            );
        }
        let mut meta = match journal::metadata(&storage).await? {
            Some(meta) if meta.identity == session.identity => meta,
            Some(_) => {
                return refuse(403, "identity_mismatch", "Forbidden")
            }
            None => {
                if get_raw(&storage, META).await?.is_some() {
                    return refuse(403, "invalid_metadata", "Forbidden");
                }
                // A generation starts from the canonical state it is named
                // for, never from whichever session arrives first: a stale
                // ticket that followed the pointer just after it moved must be
                // refused, not seed the successor with its old body. Nothing
                // is stored until a join is accepted.
                let (record_version, body_hash) =
                    if target == session.identity.room_id {
                        (
                            session.record_version.clone(),
                            session.body_hash.clone(),
                        )
                    } else {
                        match generation_state(&target) {
                            Some((room, v, h))
                                if room == session.identity.room_id =>
                            {
                                (v, h)
                            }
                            _ => {
                                return refuse(
                                    409,
                                    "invalid_generation",
                                    "Invalid room generation",
                                )
                            }
                        }
                    };
                Metadata {
                    identity: session.identity.clone(),
                    initialized: false,
                    version: 0,
                    record_version: Some(record_version),
                    saved_version: Some(0),
                    body_hash,
                }
            }
        };
        if meta.body_hash != session.body_hash {
            // Only a strictly newer canonical version may replace this body;
            // after a refusal the client re-requests a session (fresh body
            // and version) and joins again.
            let refused = match meta.record_version.as_deref() {
                Some(v) => refusal(&session, v, &meta.body_hash),
                // Legacy metadata without a version can be replaced only
                // before it holds a document.
                None if meta.initialized => Some("unversioned_room"),
                None => None,
            };
            if let Some(reason) = refused {
                return refuse(409, reason, "Live body changed");
            }
            // A reservation that is now canonical is this room's own
            // checkpoint whose ACK was lost, not an external change. An
            // unreadable journal cannot be settled; rotating leaves it here.
            let committed =
                journal::pending(&storage).await.ok().flatten().filter(
                    |p| {
                        journal::committed(
                            p,
                            &session.body_hash,
                            &session.record_version,
                        )
                    },
                );
            let Some(pending) = committed else {
                drop(doc);
                return self.replace(&session, &target).await;
            };
            let saved = journal::settle(
                &storage,
                &mut meta,
                &pending,
                &session.record_version,
            )
            .await?;
            relay::broadcast(&self.state, &saved.frame());
        }
        if meta
            .record_version
            .as_ref()
            .is_some_and(|v| newer(&session.record_version, Some(v)))
        {
            meta.record_version = Some(session.record_version.clone());
        }
        journal::put_metadata(&storage, &meta).await?;
        let meta = self.recover(&mut doc, meta).await?;
        if doc.advance(&storage).await? {
            relay::binary(&self.state, &doc.snapshot(), None)
        }
        let pair = WebSocketPair::new()?;
        self.state.accept_web_socket(&pair.server);
        pair.server.serialize_attachment(Attachment {
            kind: "photon-live".into(),
            session_id: session.session_id.clone(),
            expires_at: session.expires_at,
        })?;
        self.sessions
            .borrow_mut()
            .insert(session.session_id.clone(), session.clone());
        tickets::schedule(&storage, session.expires_at).await?;
        pair.server.send_with_bytes(doc.snapshot())?;
        relay::presence(&self.state, None);
        relay::send(
            &pair.server,
            &meta.ready(&session, &self.generation_id()),
        );
        Response::from_websocket(pair.client)
    }
    async fn websocket_message(
        &self,
        sender: WebSocket,
        message: WebSocketIncomingMessage,
    ) -> Result<()> {
        let Some(session) = self.resolve_socket(&sender).await else {
            let _ = sender.close(Some(4401), Some("Live session expired"));
            return Ok(());
        };
        match message {
            WebSocketIncomingMessage::Binary(bytes) => {
                if self.update(&sender, &session, bytes).await.is_err() {
                    relay::error(&sender, "Live update failed", None, None)
                }
            }
            WebSocketIncomingMessage::String(text) => {
                let frame = if text.len() <= MAX_TEXT {
                    serde_json::from_str::<Value>(&text).ok()
                } else {
                    None
                };
                let Some(frame) = frame.filter(|f| f["type"].is_string())
                else {
                    relay::error(
                        &sender,
                        "Invalid Live message",
                        None,
                        None,
                    );
                    return Ok(());
                };
                match frame["type"].as_str().unwrap_or_default() {
                    "awareness" => {
                        if frame["update"]
                            .as_str()
                            .filter(|s| bounded(s, 64 * 1024))
                            .and_then(|s| decode64(s, 64 * 1024))
                            .is_none()
                        {
                            relay::error(
                                &sender,
                                "Invalid awareness update",
                                None,
                                None,
                            );
                            return Ok(());
                        }
                        let _guard = self.doc.lock().await;
                        if journal::metadata(&self.state.storage())
                            .await?
                            .is_some_and(|m| {
                                m.initialized
                                    && m.identity == session.identity
                            })
                        {
                            relay::text(&self.state, &sender, &text)
                        }
                    }
                    "live-initialize" => {
                        let update = frame["update"]
                            .as_str()
                            .filter(|s| bounded(s, MAX_UPDATE * 2))
                            .and_then(|s| decode64(s, MAX_UPDATE))
                            .filter(|b| valid_update(b));
                        match update {
                            Some(update) => {
                                if self
                                    .initialize(&sender, &session, update)
                                    .await
                                    .is_err()
                                {
                                    relay::error(
                                        &sender,
                                        "Live initialization failed",
                                        None,
                                        None,
                                    )
                                }
                            }
                            None => relay::error(
                                &sender,
                                "Invalid initialization update",
                                None,
                                None,
                            ),
                        }
                    }
                    "live-checkpoint"
                        if self
                            .checkpoint(&sender, &session, &frame)
                            .await
                            .is_err() =>
                    {
                        // Authorization or storage failed. A reservation is
                        // removed only once its outcome is known, so the
                        // identical frame is safe to retry.
                        retry(
                            &sender,
                            "Live checkpoint recovery failed",
                            frame["operation_id"].as_str(),
                            "recovery_failed",
                            None,
                        )
                    }
                    _ => {}
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
        relay::presence(&self.state, Some(&ws));
        Ok(())
    }
    async fn websocket_error(
        &self,
        ws: WebSocket,
        _error: Error,
    ) -> Result<()> {
        relay::presence(&self.state, Some(&ws));
        Ok(())
    }
    async fn alarm(&self) -> Result<Response> {
        let mut expired = Vec::new();
        {
            let mut doc = self.doc.lock().await;
            let storage = self.state.storage();
            let mut next = journal::cleanup(&storage).await?;
            if doc.advance(&storage).await? {
                relay::binary(&self.state, &doc.snapshot(), None)
            }
            if doc.behind {
                next = Some(next.unwrap_or(now() + 1000).min(now() + 1000))
            }
            for ws in self.state.get_websockets() {
                match ws
                    .deserialize_attachment::<Attachment>()
                    .ok()
                    .flatten()
                    .filter(|a| a.kind == "photon-live")
                {
                    Some(a) if a.expires_at > now() => {
                        next =
                            Some(next.map_or(a.expires_at, |n| {
                                n.min(a.expires_at)
                            }))
                    }
                    a => {
                        let _ = ws.close(
                            Some(4401),
                            Some("Live session expired"),
                        );
                        if let Some(a) = a {
                            expired.push(a.session_id);
                        }
                    }
                }
            }
            if let Some(next) = next {
                tickets::set_alarm_at(&storage, next).await?;
            } else {
                storage.delete_alarm().await?;
            }
        }
        self.sessions
            .borrow_mut()
            .retain(|_, s| s.expires_at > now());
        for id in expired {
            let _ = tickets::call(&self.env, "delete", &value!({"id":id}))
                .await;
        }
        Response::empty()
    }
}
