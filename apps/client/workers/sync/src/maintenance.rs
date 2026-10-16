//! Retiring the workspace-wide records room that every account used to share.
//!
//! No ticket can name that room any more (see `room_auth`), so nothing joins
//! or writes it; its storage only waits to be removed. There is no Cloudflare
//! API that deletes one Durable Object's storage, so this route does it, and
//! only for that one room. It does not exist unless the
//! `PHOTON_SYNC_MAINTENANCE_TOKEN` secret is set, and every call needs that
//! secret as a bearer token. The runbook is
//! `workers/docs/legacy-records-room-removal.md`.
//!
//! - `GET /internal/sync/legacy-room`: sizes and counts only, never content.
//! - `DELETE /internal/sync/legacy-room` with `x-confirm-room: <room>`: deletes
//!   every stored row and the alarm, and closes any socket still attached.
use crate::model::INTERNAL;
use library_worker_common::{hash, header, json};
use serde_json::{json as value, Value};
use worker::*;

/// The room `tenant:library:workspace:library-default:records`, shared by
/// every account in the default workspace until per-user rooms.
pub const LEGACY_ROOM: &str =
    "tenant:library:workspace:library-default:records";
const SECRET: &str = "PHOTON_SYNC_MAINTENANCE_TOKEN";
pub const PATH: &str = "/internal/sync/legacy-room";
pub const INSPECT: &str = "/internal/maintenance/inspect";
pub const PURGE: &str = "/internal/maintenance/purge";

fn configured(env: &Env) -> Option<String> {
    env.secret(SECRET)
        .ok()
        .map(|s| s.to_string())
        .filter(|s| s.len() >= 32)
}

/// The edge half: checks the secret and the confirmation, then asks the room.
pub async fn route(request: Request, env: &Env) -> Result<Response> {
    let Some(secret) = configured(env) else {
        return Response::error("Not found", 404);
    };
    // Compared as digests so the comparison does not leak a prefix length.
    let presented = header(&request, "authorization").unwrap_or_default();
    if hash(&presented) != hash(&format!("Bearer {secret}")) {
        console_warn!("{{\"event\":\"sync_maintenance_refused\",\"reason\":\"credentials\"}}");
        return json(&value!({"error":"Unauthorized"}), 401);
    }
    let path = match request.method() {
        Method::Get => INSPECT,
        Method::Delete
            if header(&request, "x-confirm-room").as_deref()
                == Some(LEGACY_ROOM) =>
        {
            PURGE
        }
        Method::Delete => {
            return json(
                &value!({"error":"x-confirm-room must name the legacy room","room":LEGACY_ROOM}),
                400,
            )
        }
        _ => return Response::error("Method not allowed", 405),
    };
    let headers = Headers::new();
    headers.set(INTERNAL, "1")?;
    let mut init = RequestInit::new();
    init.with_method(Method::Post).with_headers(headers);
    let response = env
        .durable_object("PHOTON_SYNC_ROOMS")?
        .get_by_name(LEGACY_ROOM)?
        .fetch_with_request(Request::new_with_init(
            &format!("https://sync.internal{path}"),
            &init,
        )?)
        .await?;
    console_log!(
        "{{\"event\":\"sync_maintenance\",\"action\":\"{}\",\"status\":{}}}",
        if path == PURGE { "purge" } else { "inspect" },
        response.status_code()
    );
    Ok(response)
}

/// The room half: what is stored, by size only.
pub async fn inspect(state: &State) -> Result<Value> {
    let storage = state.storage();
    let snapshot: Option<Value> = storage.get("yjs:snapshot:meta").await?;
    let updates: Option<Value> = storage.get("yjs:update:meta").await?;
    let waiting = updates.as_ref().and_then(|u| {
        Some(
            u["nextSeq"]
                .as_u64()?
                .saturating_sub(u["oldestSeq"].as_u64()?),
        )
    });
    Ok(value!({
        "room": LEGACY_ROOM,
        "empty": snapshot.is_none() && updates.is_none(),
        "snapshot": snapshot.map(|s| value!({
            "seq": s["seq"], "byteLength": s["byteLength"], "chunks": s["chunks"], "updatedAt": s["updatedAt"],
        })),
        "updates": updates.map(|u| value!({
            "oldestSeq": u["oldestSeq"], "nextSeq": u["nextSeq"], "waiting": waiting,
        })),
        "alarm": storage.get_alarm().await?,
        "sockets": state.get_websockets().len(),
    }))
}

/// The room half: removes everything this room has stored.
pub async fn purge(state: &State) -> Result<Value> {
    let before = inspect(state).await?;
    for ws in state.get_websockets() {
        let _ = ws.close(Some(1012), Some("Room retired"));
    }
    let storage = state.storage();
    storage.delete_alarm().await?;
    storage.delete_all().await?;
    Ok(value!({ "before": before, "after": inspect(state).await? }))
}
