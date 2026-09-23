use crate::{model::*, tickets};
use library_worker_common::{
    api_request, encode, fetch_timeout, header, json, now, random_id,
    request_json, response_json, variable,
};
use serde_json::{json as value, Value};
use worker::*;

pub fn api_base(env: &Env) -> String {
    variable(env, "PHOTON_LIVE_API_BASE_URL")
        .or_else(|| variable(env, "PHOTON_CLOUD_ENGINE_BASE_URL"))
        .unwrap_or_else(|| "http://127.0.0.1:3001".into())
        .trim_end_matches('/')
        .into()
}
pub fn api_url(
    env: &Env,
    org: &str,
    repo: &str,
    data: &str,
    suffix: &str,
) -> String {
    format!(
        "{}/v1beta/repos/{}/{}/data/{}/live/{suffix}",
        api_base(env),
        encode(org),
        encode(repo),
        encode(data)
    )
}
pub fn session_headers(session: &Session) -> Result<Headers> {
    let headers = Headers::new();
    headers.set("authorization", &session.authorization)?;
    headers.set("content-type", "application/json")?;
    headers
        .set("x-photon-request-id", &format!("live_{}", random_id(18)?))?;
    if let Some(v) = &session.platform_id {
        headers.set("x-platform-id", v)?;
    }
    if let Some(v) = &session.operator_id {
        headers.set("x-operator-id", v)?;
    }
    Ok(headers)
}
fn normalize(payload: Value) -> Option<Authorization> {
    let mut identity = Identity {
        room_id: String::new(),
        tenant: canonical(&payload, "tenant", "tenant_id")?,
        database: canonical(&payload, "database", "database_id")?,
        data: canonical(&payload, "data", "data_id")?,
        property: canonical(&payload, "property", "property_id")?,
        format: field(&payload, "format")?,
    };
    let body = payload["body"]
        .as_str()
        .filter(|s| s.len() <= MAX_BODY)?
        .to_owned();
    let actor_id = canonical(&payload, "actor_id", "actorId")?;
    let record_version =
        canonical(&payload, "record_version", "recordVersion")?;
    identity.room_id = field(&payload, "room_id").unwrap_or_else(|| {
        format!(
            "live:{}",
            library_worker_common::hash(
                &value!([
                    identity.tenant,
                    identity.database,
                    identity.data,
                    identity.property,
                    identity.format
                ])
                .to_string()
            )
        )
    });
    Some(Authorization {
        identity,
        actor_id,
        body,
        record_version,
    })
}
pub async fn authorize(
    env: &Env,
    org: &str,
    repo: &str,
    data: &str,
    property: &str,
    headers: Headers,
) -> std::result::Result<Authorization, u16> {
    let req = api_request(
        &api_url(env, org, repo, data, "authorize"),
        Method::Post,
        headers,
        Some(&value!({"property_id":property})),
    )
    .map_err(|_| 502u16)?;
    let mut response =
        fetch_timeout(req, 15_000).await.map_err(|_| 502u16)?;
    match response.status_code() {
        200..=299 => normalize(
            response_json(&mut response, MAX_API)
                .await
                .map_err(|_| 502u16)?,
        )
        .ok_or(502),
        400..=499 => Err(response.status_code()),
        _ => Err(502),
    }
}
pub async fn current(
    env: &Env,
    session: &Session,
) -> Option<Authorization> {
    let auth = authorize(
        env,
        &session.org,
        &session.repo,
        &session.identity.data,
        &session.identity.property,
        session_headers(session).ok()?,
    )
    .await
    .ok()?;
    (auth.identity == session.identity).then_some(auth)
}
fn allowed(req: &Request, env: &Env) -> bool {
    let Some(origin) = header(req, "origin") else {
        return false;
    };
    variable(env, "PHOTON_LIVE_ALLOWED_ORIGINS")
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .any(|s| !s.is_empty() && s == origin)
}
fn cors(req: &Request, env: &Env, response: &mut Response) -> Result<()> {
    let headers = response.headers_mut();
    headers.set("access-control-allow-methods", "GET,POST,OPTIONS")?;
    headers.set("access-control-allow-headers","authorization,content-type,x-platform-id,x-operator-id,x-request-id,x-photon-request-id")?;
    headers.set("access-control-expose-headers", "x-photon-request-id")?;
    headers.set("vary", "Origin")?;
    if allowed(req, env) {
        headers.set(
            "access-control-allow-origin",
            &header(req, "origin").unwrap_or_default(),
        )?;
    }
    Ok(())
}
pub async fn route(mut request: Request, env: &Env) -> Result<Response> {
    let enabled = variable(env, "PHOTON_LIVE_ENABLED").is_some_and(|s| {
        matches!(
            s.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    });
    let mut response = if !enabled {
        json(
            &value!({"code":"LIVE_DISABLED","error":"Photon Live is disabled"}),
            404,
        )?
    } else if !allowed(&request, env) {
        json(&value!({"error":"Live origin is not allowed"}), 403)?
    } else if request.method() == Method::Options {
        Response::empty()?.with_status(204)
    } else if request.path() == "/live/session" {
        create(&mut request, env).await?
    } else {
        open(&request, env).await?
    };
    // Upgrade responses carry the original WebSocket and cannot be reconstructed.
    if response.status_code() != 101 {
        let headers = response.headers().clone();
        response = response.with_headers(headers);
        cors(&request, env, &mut response)?;
    }
    Ok(response)
}
async fn create(request: &mut Request, env: &Env) -> Result<Response> {
    if request.method() != Method::Post {
        return json(&value!({"error":"Method not allowed"}), 405);
    }
    let input = request_json(request, 64 * 1024).await.ok();
    let fields = input.as_ref().and_then(|v| {
        Some((
            field(v, "org")?,
            field(v, "repo")?,
            field(v, "data_id")?,
            field(v, "property_id")?,
        ))
    });
    let Some((org, repo, data, property)) = fields else {
        return json(&value!({"error":"Invalid session request"}), 400);
    };
    let Some(token) =
        header(request, "authorization").and_then(|s| bearer(&s))
    else {
        return json(&value!({"error":"Live authorization failed"}), 401);
    };
    let platform =
        header(request, "x-platform-id").filter(|s| bounded(s, 512));
    let operator =
        header(request, "x-operator-id").filter(|s| bounded(s, 512));
    let headers = Headers::new();
    headers.set("authorization", &token)?;
    headers.set("content-type", "application/json")?;
    headers.set(
        "x-photon-request-id",
        &header(request, "x-photon-request-id")
            .or_else(|| header(request, "x-request-id"))
            .unwrap_or(random_id(18)?),
    )?;
    if let Some(v) = &platform {
        headers.set("x-platform-id", v)?
    }
    if let Some(v) = &operator {
        headers.set("x-operator-id", v)?
    }
    let auth = match authorize(env, &org, &repo, &data, &property, headers)
        .await
    {
        Ok(a) => a,
        Err(status) => {
            return json(
                &value!({"error":"Live authorization failed"}),
                status,
            )
        }
    };
    let session = Session {
        identity: auth.identity.clone(),
        actor_id: auth.actor_id.clone(),
        record_version: auth.record_version.clone(),
        body_hash: body_hash(&auth.identity.format, &auth.body),
        org,
        repo,
        authorization: token,
        expires_at: now() + TTL,
        session_id: String::new(),
        platform_id: platform,
        operator_id: operator,
    };
    let ticket =
        match tickets::call(env, "issue", &serde_json::to_value(session)?)
            .await
        {
            Ok(v) => v,
            Err(_) => {
                return json(
                    &value!({"error":"Live session unavailable"}),
                    503,
                )
            }
        };
    json(
        &value!({"ticket":ticket["ticket"],"room_id":auth.identity.room_id,"actor_id":auth.actor_id,"format":auth.identity.format,"body":auth.body,"record_version":auth.record_version}),
        200,
    )
}
/// Browsers only see a failed upgrade as close 1006, so each refusal is logged
/// for `wrangler tail` with its status and a fixed reason. Never log the
/// ticket, the session or its authorization.
fn refuse(status: u16, reason: &str, message: &str) -> Result<Response> {
    console_warn!(
        "{{\"event\":\"live_open_refused\",\"status\":{status},\"reason\":\"{reason}\"}}"
    );
    json(&value!({ "error": message }), status)
}
async fn open(request: &Request, env: &Env) -> Result<Response> {
    if request.method() != Method::Get {
        return refuse(405, "method", "Method not allowed");
    }
    if !header(request, "upgrade")
        .is_some_and(|s| s.eq_ignore_ascii_case("websocket"))
    {
        return refuse(426, "not_websocket", "Expected WebSocket upgrade");
    }
    let ticket = request
        .url()?
        .query_pairs()
        .find(|(k, _)| k == "ticket")
        .map(|(_, v)| v.into_owned())
        .unwrap_or_default();
    if !token(&ticket, 40, 64) {
        return refuse(
            401,
            "malformed_ticket",
            "Invalid or expired Live ticket",
        );
    }
    let session =
        match tickets::call(env, "consume", &value!({"id":ticket})).await {
            Ok(v) => serde_json::from_value::<Session>(v).ok(),
            Err(_) => {
                return refuse(
                    503,
                    "ticket_store_unavailable",
                    "Live session unavailable",
                )
            }
        };
    let Some(session) =
        session.filter(|s| s.valid(false) && s.expires_at > now())
    else {
        return refuse(
            401,
            "unknown_ticket",
            "Invalid or expired Live ticket",
        );
    };
    let headers = Headers::new();
    headers.set("upgrade", "websocket")?;
    headers.set(INTERNAL, "1")?;
    headers.set(SESSION, &session.header())?;
    let mut room = session.identity.room_id.clone();
    for hop in 0..8 {
        headers.set(TARGET, &room)?;
        let req = api_request(
            "https://live.internal/live/internal-ws",
            Method::Get,
            headers.clone(),
            None,
        )?;
        let response = env
            .durable_object("PHOTON_LIVE_ROOMS")?
            .get_by_name(&room)?
            .fetch_with_request(req)
            .await?;
        let status = response.status_code();
        if status != 409 {
            if status != 101 {
                // The room logged its own reason as live_join_refused.
                console_warn!(
                    "{{\"event\":\"live_open_refused\",\"status\":{status},\"reason\":\"room\",\"hops\":{hop}}}"
                );
            }
            return Ok(response);
        }
        let generation = response
            .headers()
            .get(GENERATION)?
            .filter(|s| generated(s) && s != &room);
        match generation {
            Some(next) => room = next,
            None => {
                console_warn!(
                    "{{\"event\":\"live_open_refused\",\"status\":409,\"reason\":\"room\",\"hops\":{hop}}}"
                );
                return Ok(response);
            }
        }
    }
    refuse(
        409,
        "generation_limit",
        "Live room generation limit reached",
    )
}
