//! Authorization for the generic Yjs relay (`/ws`).
//!
//! A browser WebSocket cannot carry an `Authorization` header, so a client
//! first trades its Library bearer token for a one-time ticket at
//! `POST /ws/ticket`, then opens `/ws?ticket=...`. The ticket names the one
//! room it admits. A room is only issued to the user it is named after:
//! `tenant:{tenant}:workspace:{workspace}:user:{actor}:records`, where
//! `actor` is the caller's Library user id as library-api's `me` returns it.
use crate::{model::*, tickets};
use library_worker_common::{
    api_request, fetch_timeout, header, json, random_id, request_json,
    response_json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json as value, Value};
use worker::*;

/// The only relay surface a ticket can name.
const SURFACE: &str = "records";
const ME_QUERY: &str = "query LibrarySyncRoomActor { me { id } }";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomTicket {
    pub room: String,
    pub expires_at: i64,
}
impl RoomTicket {
    pub fn valid(&self) -> bool {
        bounded(&self.room, 512)
            && self.expires_at > 0
            && self.expires_at <= MAX_SAFE as i64
    }
}

/// The room segment for a Library user id. `buildUserStorageScope` in
/// `src/app/kitConfig.ts` replaces characters outside `[A-Za-z0-9_-]` with
/// `-`, which can map two ids to one name, so an id that would need that is
/// refused instead: no two users can be issued the same room. Library ids
/// (`us_` + ULID) never need it.
pub fn actor_segment(user_id: &str) -> Option<&str> {
    token(user_id, 1, 128).then_some(user_id)
}

fn scope_segment(value: &str) -> bool {
    (1..=128).contains(&value.len())
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_.-".contains(&c))
}

/// Whether `room` is the records room of the user `user_id`.
pub fn owned_room(room: &str, user_id: &str) -> bool {
    let Some(actor) = actor_segment(user_id) else {
        return false;
    };
    let parts: Vec<&str> = room.split(':').collect();
    matches!(
        parts.as_slice(),
        ["tenant", tenant, "workspace", workspace, "user", user, surface]
            if scope_segment(tenant)
                && scope_segment(workspace)
                && *user == actor
                && *surface == SURFACE
    )
}

/// Browsers only see a failed upgrade as close 1006, so each refusal is logged
/// with its status and a fixed reason. Never log a ticket or a credential.
fn refuse(
    event: &str,
    status: u16,
    reason: &str,
    message: &str,
) -> Result<Response> {
    console_warn!(
        "{{\"event\":\"{event}\",\"status\":{status},\"reason\":\"{reason}\"}}"
    );
    json(&value!({ "error": message }), status)
}

/// Resolves the caller's Library user id with their own credentials.
async fn caller(
    env: &Env,
    request: &Request,
    token: &str,
) -> std::result::Result<String, u16> {
    let headers = Headers::new();
    let set =
        |name: &str, v: &str| headers.set(name, v).map_err(|_| 502u16);
    set("authorization", token)?;
    set("content-type", "application/json")?;
    set(
        "x-photon-request-id",
        &header(request, "x-photon-request-id")
            .or_else(|| header(request, "x-request-id"))
            .or_else(|| random_id(18).ok())
            .unwrap_or_default(),
    )?;
    for name in ["x-platform-id", "x-operator-id"] {
        if let Some(v) = header(request, name).filter(|s| bounded(s, 512)) {
            set(name, &v)?;
        }
    }
    let req = api_request(
        &format!("{}/v1/graphql", crate::auth::api_base(env)),
        Method::Post,
        headers,
        Some(&value!({ "query": ME_QUERY })),
    )
    .map_err(|_| 502u16)?;
    let mut response =
        fetch_timeout(req, 15_000).await.map_err(|_| 502u16)?;
    match response.status_code() {
        200..=299 => {}
        401 | 403 => return Err(response.status_code()),
        400..=499 => return Err(401),
        _ => return Err(502),
    }
    let body: Value = response_json(&mut response, 64 * 1024)
        .await
        .map_err(|_| 502u16)?;
    // A token Library does not know answers 200 with an error and no user.
    body["data"]["me"]["id"]
        .as_str()
        .filter(|s| bounded(s, 512))
        .map(str::to_owned)
        .ok_or(401)
}

/// `POST /ws/ticket` with `{"room": "..."}` and the caller's bearer token.
pub async fn issue(request: &mut Request, env: &Env) -> Result<Response> {
    const EVENT: &str = "sync_ticket_refused";
    if request.method() != Method::Post {
        return refuse(EVENT, 405, "method", "Method not allowed");
    }
    let room = request_json(request, 16 * 1024)
        .await
        .ok()
        .and_then(|v| field(&v, "room"));
    let Some(room) = room else {
        return refuse(
            EVENT,
            400,
            "invalid_request",
            "Invalid ticket request",
        );
    };
    let Some(token) =
        header(request, "authorization").and_then(|s| bearer(&s))
    else {
        return refuse(
            EVENT,
            401,
            "no_credentials",
            "Sync authorization failed",
        );
    };
    let user = match caller(env, request, &token).await {
        Ok(user) => user,
        Err(502) => {
            return refuse(
                EVENT,
                502,
                "api_unavailable",
                "Sync authorization unavailable",
            )
        }
        Err(status) => {
            return refuse(
                EVENT,
                status,
                "not_a_library_user",
                "Sync authorization failed",
            )
        }
    };
    if !owned_room(&room, &user) {
        return refuse(EVENT, 403, "foreign_room", "Room is not yours");
    }
    match tickets::call(env, "issue-room", &value!({ "room": room })).await
    {
        Ok(v) => {
            json(&value!({ "ticket": v["ticket"], "room": room }), 200)
        }
        Err(_) => refuse(
            EVENT,
            503,
            "ticket_store_unavailable",
            "Sync session unavailable",
        ),
    }
}

/// `GET /ws?ticket=...`: consumes the ticket and joins the room it names.
pub async fn open(request: Request, env: &Env) -> Result<Response> {
    const EVENT: &str = "sync_open_refused";
    if !header(&request, "upgrade")
        .is_some_and(|s| s.eq_ignore_ascii_case("websocket"))
    {
        return refuse(
            EVENT,
            426,
            "not_websocket",
            "Expected WebSocket upgrade",
        );
    }
    let url = request.url()?;
    let param = |name: &str| {
        url.query_pairs()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.into_owned())
    };
    let ticket = param("ticket").unwrap_or_default();
    if !token(&ticket, 40, 64) {
        return refuse(
            EVENT,
            401,
            "no_ticket",
            "Invalid or expired sync ticket",
        );
    }
    let consumed =
        match tickets::call(env, "consume-room", &value!({ "id": ticket }))
            .await
        {
            Ok(v) => serde_json::from_value::<RoomTicket>(v).ok(),
            Err(_) => {
                return refuse(
                    EVENT,
                    503,
                    "ticket_store_unavailable",
                    "Sync session unavailable",
                )
            }
        };
    let Some(consumed) = consumed.filter(RoomTicket::valid) else {
        return refuse(
            EVENT,
            401,
            "unknown_ticket",
            "Invalid or expired sync ticket",
        );
    };
    if param("room").is_some_and(|room| room != consumed.room) {
        return refuse(
            EVENT,
            403,
            "room_mismatch",
            "Ticket is for another room",
        );
    }
    env.durable_object("PHOTON_SYNC_ROOMS")?
        .get_by_name(&consumed.room)?
        .fetch_with_request(request)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROOM: &str =
        "tenant:library:workspace:library-default:user:us_01abc:records";

    #[test]
    fn a_user_owns_only_their_records_room() {
        assert!(owned_room(ROOM, "us_01abc"));
        assert!(!owned_room(ROOM, "us_01abd"));
        assert!(!owned_room(ROOM, ""));
    }

    #[test]
    fn the_shared_and_malformed_rooms_belong_to_no_one() {
        for room in [
            "tenant:library:workspace:library-default:records",
            "records",
            "tenant:library:workspace:library-default:user:us_01abc:doc",
            "tenant:library:workspace:library-default:user:us_01abc:records:x",
            "tenant::workspace:library-default:user:us_01abc:records",
            "tenant:a b:workspace:w:user:us_01abc:records",
            "team:library:workspace:library-default:user:us_01abc:records",
        ] {
            assert!(!owned_room(room, "us_01abc"), "{room}");
        }
    }

    #[test]
    fn ids_that_would_need_normalizing_get_no_room() {
        // "a.b" and "a-b" would both become "a-b" in the client.
        assert_eq!(actor_segment("a.b"), None);
        assert!(!owned_room(
            "tenant:t:workspace:w:user:a-b:records",
            "a.b"
        ));
        assert!(owned_room("tenant:t:workspace:w:user:a-b:records", "a-b"));
        assert_eq!(actor_segment(""), None);
        assert_eq!(actor_segment(" us_1"), None);
    }
}
