use crate::{model::*, room_auth::RoomTicket, storage::*};
use futures::{lock::Mutex, FutureExt};
use library_worker_common::{
    api_request, json, now, random_id, request_json, response_json,
};
use serde_json::{json as value, Value};
use worker::*;

const TICKETS: &str = "live:ticket:";
const SESSIONS: &str = "live:session:";
/// One-time tickets for the generic Yjs relay (`/ws`); see `room_auth`.
const ROOM_TICKETS: &str = "sync:ticket:";

/// When a stored row expires, or `None` for a row that is not valid.
type Expiry = fn(&wasm_bindgen::JsValue) -> Option<i64>;

fn live_expiry(raw: &wasm_bindgen::JsValue) -> Option<i64> {
    serde_wasm_bindgen::from_value::<Session>(raw.clone())
        .ok()
        .filter(|s| s.valid(false))
        .map(|s| s.expires_at)
}
fn room_expiry(raw: &wasm_bindgen::JsValue) -> Option<i64> {
    serde_wasm_bindgen::from_value::<RoomTicket>(raw.clone())
        .ok()
        .filter(RoomTicket::valid)
        .map(|t| t.expires_at)
}

pub async fn call(env: &Env, action: &str, value: &Value) -> Result<Value> {
    let stub = env
        .durable_object("PHOTON_LIVE_TICKETS")?
        .get_by_name("live-ticket-store")?;
    let mut response = stub
        .fetch_with_request(api_request(
            &format!("https://tickets.internal/{action}"),
            Method::Post,
            Headers::new(),
            Some(value),
        )?)
        .await?;
    if response.status_code() != 200 {
        return Err("Live ticket store unavailable".into());
    }
    response_json(&mut response, 64 * 1024).await
}
pub async fn session(env: &Env, id: &str) -> Result<Option<Session>> {
    let value = call(env, "session", &value!({"id":id})).await?;
    Ok(serde_json::from_value::<Session>(value)
        .ok()
        .filter(|s| s.valid(false) && s.expires_at > now()))
}
pub async fn set_alarm_at(storage: &Storage, at: i64) -> Result<()> {
    // workers-rs interprets integer ScheduledTime arguments as offsets, unlike
    // the JavaScript storage API. A Date explicitly carries an epoch timestamp.
    storage
        .set_alarm(ScheduledTime::new(js_sys::Date::new(
            &wasm_bindgen::JsValue::from_f64(at as f64),
        )))
        .await
}
pub async fn schedule(storage: &Storage, at: i64) -> Result<()> {
    if storage
        .get_alarm()
        .await?
        .is_none_or(|current| at < current)
    {
        set_alarm_at(storage, at).await?;
    }
    Ok(())
}

#[durable_object]
pub struct PhotonLiveTicketStore {
    state: State,
    lock: Mutex<()>,
}
impl DurableObject for PhotonLiveTicketStore {
    fn new(state: State, _env: Env) -> Self {
        Self {
            state,
            lock: Mutex::new(()),
        }
    }
    async fn fetch(&self, mut request: Request) -> Result<Response> {
        if request.method() != Method::Post {
            return Response::error("Not found", 404);
        }
        let input = match request_json(&mut request, 64 * 1024).await {
            Ok(v) => v,
            Err(_) => {
                return Response::error("Invalid ticket request", 400)
            }
        };
        let _guard = self.lock.lock().await;
        let storage = self.state.storage();
        match request.path().as_str() {
            "/issue" => {
                let mut session: Session =
                    match serde_json::from_value(input) {
                        Ok(s) => s,
                        Err(_) => {
                            return Response::error("Invalid session", 400)
                        }
                    };
                if !session.valid(false) {
                    return Response::error("Invalid session", 400);
                }
                let ticket = random_id(32)?;
                session.session_id = random_id(18)?;
                session.expires_at = now() + TTL;
                storage
                    .put_raw(
                        &format!("{TICKETS}{ticket}"),
                        to_js(&session)?,
                    )
                    .await?;
                schedule(&storage, session.expires_at).await?;
                json(
                    &value!({"ticket":ticket,"sessionId":session.session_id}),
                    200,
                )
            }
            "/issue-room" => {
                let Some(room) =
                    input["room"].as_str().filter(|s| bounded(s, 512))
                else {
                    return Response::error("Invalid room ticket", 400);
                };
                let ticket = random_id(32)?;
                let entry = RoomTicket {
                    room: room.to_owned(),
                    expires_at: now() + TTL,
                };
                storage
                    .put_raw(
                        &format!("{ROOM_TICKETS}{ticket}"),
                        to_js(&entry)?,
                    )
                    .await?;
                schedule(&storage, entry.expires_at).await?;
                json(&value!({ "ticket": ticket }), 200)
            }
            "/consume-room" => {
                let id = input["id"].as_str().unwrap_or_default();
                if !token(id, 40, 64) {
                    return json(&Value::Null, 200);
                }
                let key = format!("{ROOM_TICKETS}{id}");
                let entry = transaction(&storage, move |tx| {
                    async move {
                        let raw = tx_raw(&tx, &key).await?;
                        tx.delete(&key).await?;
                        Ok(raw
                            .and_then(|v| {
                                serde_wasm_bindgen::from_value::<
                                        RoomTicket,
                                    >(v)
                                    .ok()
                            })
                            .filter(|t| t.valid() && t.expires_at > now()))
                    }
                    .boxed_local()
                })
                .await?;
                json(&serde_json::to_value(entry)?, 200)
            }
            "/consume" | "/session" | "/delete" => {
                let id = input["id"].as_str().unwrap_or_default();
                let consume = request.path() == "/consume";
                if !token(id, if consume { 40 } else { 20 }, 64) {
                    return json(&Value::Null, 200);
                }
                let key = format!(
                    "{}{id}",
                    if consume { TICKETS } else { SESSIONS }
                );
                if request.path() == "/delete" {
                    storage.delete(&key).await?;
                    return json(&Value::Null, 200);
                }
                let session = transaction(&storage, move |tx| {
                    async move {
                        let raw = tx_raw(&tx, &key).await?;
                        let session = raw
                            .and_then(|v| {
                                serde_wasm_bindgen::from_value::<Session>(v)
                                    .ok()
                            })
                            .filter(|s| {
                                s.valid(false)
                                    && s.expires_at > now()
                                    && token(&s.session_id, 20, 64)
                            });
                        if consume || session.is_none() {
                            tx.delete(&key).await?;
                        }
                        if consume {
                            if let Some(s) = &session {
                                tx_put(
                                    &tx,
                                    &format!("{SESSIONS}{}", s.session_id),
                                    s,
                                )
                                .await?;
                            }
                        }
                        Ok(session)
                    }
                    .boxed_local()
                })
                .await?;
                if let Some(s) = &session {
                    schedule(&storage, s.expires_at).await?;
                }
                json(&serde_json::to_value(session)?, 200)
            }
            _ => Response::error("Not found", 404),
        }
    }
    async fn alarm(&self) -> Result<Response> {
        let _guard = self.lock.lock().await;
        let storage = self.state.storage();
        let mut next = None::<i64>;
        let prefixes: [(&str, &str, Expiry); 3] = [
            (TICKETS, "live:cleanup:cursor:ticket", live_expiry),
            (SESSIONS, "live:cleanup:cursor:session", live_expiry),
            (ROOM_TICKETS, "sync:cleanup:cursor:ticket", room_expiry),
        ];
        for (prefix, cursor_key, expiry) in prefixes {
            let cursor: Option<String> = storage.get(cursor_key).await?;
            let start = cursor.as_ref().map(|s| format!("{s}\0"));
            let mut options = ListOptions::new().prefix(prefix).limit(128);
            if let Some(s) = &start {
                options = options.start(s);
            }
            let rows = entries(storage.list_with_options(options).await?)?;
            for (key, raw) in &rows {
                match expiry(raw).filter(|at| *at > now()) {
                    Some(at) => next = Some(next.map_or(at, |n| n.min(at))),
                    None => {
                        storage.delete(key).await?;
                    }
                }
            }
            if rows.len() == 128 {
                storage
                    .put(cursor_key, &rows.last().expect("nonempty").0)
                    .await?;
                next = Some(
                    next.map_or(now() + 1000, |n| n.min(now() + 1000)),
                );
            } else if cursor.is_some() {
                storage.delete(cursor_key).await?;
                next = Some(
                    next.map_or(now() + 1000, |n| n.min(now() + 1000)),
                );
            }
        }
        if let Some(next) = next {
            set_alarm_at(&storage, next).await?;
        } else {
            storage.delete_alarm().await?;
        }
        Response::empty()
    }
}
