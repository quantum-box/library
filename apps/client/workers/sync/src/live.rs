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
    async fn rotate(
        &self,
        session: &Session,
        target: &str,
    ) -> Result<Response> {
        let generation = generation(session);
        if !generated(&generation) {
            return Response::error("Invalid room generation", 409);
        }
        let accepted = if target == session.identity.room_id {
            let _guard = self.doc.lock().await;
            self.accept_pointer(session, generation).await?
        } else {
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
                return Response::error(
                    "Live room generation unavailable",
                    503,
                );
            }
            let pointer: Pointer =
                response_json(&mut response, 64 * 1024).await?;
            if !pointer.valid() {
                return Response::error(
                    "Live room generation unavailable",
                    503,
                );
            }
            pointer.room_id
        };
        let mut response = Response::error("Live body changed", 409)?;
        response.headers_mut().set(GENERATION, &accepted)?;
        Ok(response)
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
            relay::error(
                sender,
                "Live room identity mismatch",
                None,
                false,
            );
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
                false,
            );
            return Ok(());
        }
        let mut doc = self.doc.lock().await;
        let storage = self.state.storage();
        let Some(mut meta) = journal::metadata(&storage)
            .await?
            .filter(|m| m.identity == session.identity)
        else {
            relay::error(
                sender,
                "Live room identity mismatch",
                None,
                false,
            );
            return Ok(());
        };
        if !meta.initialized {
            relay::error(
                sender,
                "Live document is not initialized",
                None,
                false,
            );
            return Ok(());
        }
        if meta.version >= MAX_SAFE {
            relay::error(sender, "Live version limit reached", None, false);
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
                false,
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
                false,
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
                false,
            );
            return Ok(None);
        };
        if !meta.initialized {
            relay::error(
                sender,
                "Live document is not initialized",
                Some(id),
                false,
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
                    false,
                );
                return Ok(None);
            }
            return Ok(Some((
                p.clone(),
                journal::read_body(&storage, p).await?,
            )));
        }
        let authorized =
            authorized.ok_or("Live checkpoint recovery failed")?;
        if authorized.identity != session.identity {
            return Err("Live checkpoint recovery failed".into());
        }
        let canonical_hash =
            body_hash(&authorized.identity.format, &authorized.body);
        if let Some(p) = pending {
            // A replacement must describe the current working document. Never
            // erase the crash journal on a stale request.
            if version != meta.version {
                relay::error(
                    sender,
                    "Checkpoint is behind the working version",
                    Some(id),
                    version < meta.version,
                );
                return Ok(None);
            }
            if authorized.record_version == p.expected_record_version {
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
            } else {
                if !newer(
                    &authorized.record_version,
                    Some(&p.expected_record_version),
                ) || canonical_hash != p.body_hash
                {
                    journal::delete_pending(&storage, &p).await?;
                    self.conflict(id);
                    return Ok(None);
                }
                journal::delete_pending(&storage, &p).await?;
                meta.record_version =
                    Some(authorized.record_version.clone());
                meta.body_hash = canonical_hash;
                meta.saved_version =
                    Some(meta.saved().max(p.version.min(meta.version)));
            }
            journal::put_metadata(&storage, &meta).await?;
        } else {
            if canonical_hash != meta.body_hash {
                self.conflict(id);
                return Ok(None);
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
                false,
            );
            return Ok(None);
        };
        if version != meta.version {
            relay::error(
                sender,
                "Checkpoint is behind the working version",
                Some(id),
                version < meta.version,
            );
            return Ok(None);
        }
        let proposed_hash = body_hash(&meta.identity.format, body);
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
            relay::error(
                sender,
                "Invalid checkpoint",
                id.as_deref(),
                false,
            );
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
                    false,
                );
                return Ok(());
            };
            if !meta.initialized {
                relay::error(
                    sender,
                    "Live document is not initialized",
                    Some(id),
                    false,
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
        let mut response = match fetch_timeout(request, 15_000).await {
            Ok(response) => response,
            Err(_) => {
                relay::error(
                    sender,
                    "Live checkpoint failed",
                    Some(id),
                    false,
                );
                return Ok(());
            }
        };
        if response.status_code() == 409 {
            let _guard = self.doc.lock().await;
            journal::delete_pending(&storage, &pending).await?;
            self.conflict(id);
            return Ok(());
        }
        if !(200..300).contains(&response.status_code()) {
            relay::error(sender, "Live checkpoint failed", Some(id), false);
            return Ok(());
        }
        let record_version = response_json::<Value>(&mut response, MAX_API)
            .await
            .ok()
            .and_then(|v| canonical(&v, "record_version", "recordVersion"));
        let Some(record_version) = record_version else {
            relay::error(sender, "Live checkpoint failed", Some(id), false);
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
            return Response::error("Not found", 404);
        }
        let Some(session) = self.resolve_reference(&request).await? else {
            return Response::error("Unauthorized", 401);
        };
        if request.path() == "/live/internal-pointer" {
            if request.method() != Method::Post {
                return Response::error("Method not allowed", 405);
            }
            let input = request_json(&mut request, 64 * 1024).await?;
            let candidate =
                field(&input, "room_id").filter(|s| generated(s));
            if candidate.is_none()
                || input["body_hash"] != session.body_hash
                || input["record_version"] != session.record_version
            {
                return Response::error("Invalid room pointer", 400);
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
                return Response::error(
                    "Live room generation unavailable",
                    503,
                );
            }
            return json(&serde_json::to_value(pointer)?, 200);
        }
        if !header(&request, "upgrade")
            .is_some_and(|v| v.eq_ignore_ascii_case("websocket"))
        {
            return Response::error("Expected WebSocket upgrade", 426);
        }
        let target = header(&request, TARGET)
            .filter(|s| bounded(s, 512))
            .unwrap_or_else(|| session.identity.room_id.clone());
        let mut doc = self.doc.lock().await;
        let storage = self.state.storage();
        if target == session.identity.room_id {
            let pointer: Option<Pointer> = storage.get(POINTER).await?;
            if let Some(pointer) = pointer.filter(|p| {
                p.valid() && p.room_id != session.identity.room_id
            }) {
                let mut response =
                    Response::error("Live room generation changed", 409)?;
                response.headers_mut().set(GENERATION, &pointer.room_id)?;
                return Ok(response);
            }
        }
        let mut meta = match journal::metadata(&storage).await? {
            Some(meta) if meta.identity == session.identity => meta,
            Some(_) => return Response::error("Forbidden", 403),
            None => {
                if get_raw(&storage, META).await?.is_some() {
                    return Response::error("Forbidden", 403);
                }
                let meta = Metadata {
                    identity: session.identity.clone(),
                    initialized: false,
                    version: 0,
                    record_version: Some(session.record_version.clone()),
                    saved_version: Some(0),
                    body_hash: session.body_hash.clone(),
                };
                journal::put_metadata(&storage, &meta).await?;
                meta
            }
        };
        if meta.body_hash != session.body_hash {
            if meta.dirty()
                || meta.record_version.as_ref().is_some_and(|v| {
                    !newer(&session.record_version, Some(v))
                })
                || (meta.record_version.is_none() && meta.initialized)
            {
                return Response::error("Live body changed", 409);
            }
            for ws in self.state.get_websockets() {
                let _ = ws.close(
                    Some(4410),
                    Some("Live canonical body changed; reconnect required"),
                );
            }
            drop(doc);
            return self.rotate(&session, &target).await;
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
                    relay::error(&sender, "Live update failed", None, false)
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
                        false,
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
                                false,
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
                                        false,
                                    )
                                }
                            }
                            None => relay::error(
                                &sender,
                                "Invalid initialization update",
                                None,
                                false,
                            ),
                        }
                    }
                    "live-checkpoint"
                        if self
                            .checkpoint(&sender, &session, &frame)
                            .await
                            .is_err() =>
                    {
                        relay::error(
                            &sender,
                            "Live checkpoint recovery failed",
                            frame["operation_id"].as_str(),
                            false,
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
