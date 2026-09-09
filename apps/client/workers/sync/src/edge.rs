use crate::{auth, model::INTERNAL};
use library_worker_common::{
    bounded_stream, fetch_timeout, header, json, now, random_id, variable,
};
use serde_json::{json as value, Value};
use std::{cell::RefCell, collections::VecDeque};
use worker::*;
const PATHS: [&str; 3] =
    ["/api/engine/push", "/api/engine/pull", "/api/engine/debug"];
// Diagnostic history only: bounded, non-authoritative, and never contains
// credentials or document content. Preserves the existing debug endpoint.
thread_local! {static LOGS:RefCell<VecDeque<Value>>=const{RefCell::new(VecDeque::new())};}
fn base(env: &Env) -> String {
    variable(env, "PHOTON_CLOUD_ENGINE_BASE_URL")
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "http://127.0.0.1:3001".into())
        .trim_end_matches('/')
        .into()
}
fn cors(response: &mut Response) -> Result<()> {
    let h = response.headers_mut();
    h.set("access-control-allow-origin", "*")?;
    h.set("access-control-allow-methods", "GET,POST,OPTIONS")?;
    h.set(
        "access-control-allow-headers",
        "authorization,content-type,x-request-id,x-photon-request-id",
    )?;
    h.set("access-control-expose-headers", "x-photon-request-id")?;
    Ok(())
}
fn record(log: Value) {
    console_log!("{}", log);
    LOGS.with(|logs| {
        let mut logs = logs.borrow_mut();
        logs.push_front(log);
        logs.truncate(100);
    });
}
struct CountedStream {
    stream: ByteStream,
    log: Option<Value>,
    bytes: usize,
    started: i64,
}
impl CountedStream {
    fn finish(&mut self, cancelled: bool) {
        if let Some(mut log) = self.log.take() {
            log["responseBytes"] = value!(self.bytes);
            log["durationMs"] = value!(now() - self.started);
            if cancelled {
                log["error"] = value!("response stream cancelled");
            }
            record(log);
        }
    }
}
impl futures::Stream for CountedStream {
    type Item = Result<Vec<u8>>;
    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let next = std::pin::Pin::new(&mut self.stream).poll_next(cx);
        match &next {
            std::task::Poll::Ready(Some(Ok(bytes))) => {
                self.bytes += bytes.len()
            }
            std::task::Poll::Ready(None) => self.finish(false),
            std::task::Poll::Ready(Some(Err(_))) => self.finish(true),
            _ => {}
        }
        next
    }
}
impl Drop for CountedStream {
    fn drop(&mut self) {
        self.finish(true);
    }
}
fn counted(
    response: Response,
    mut log: Value,
    started: i64,
) -> Result<Response> {
    let (builder, body) = response.into_parts();
    match body {
        ResponseBody::Stream(stream) => {
            let mut source =
                Response::from_body(ResponseBody::Stream(stream))?;
            builder.from_stream(CountedStream {
                stream: source.stream()?,
                log: Some(log),
                bytes: 0,
                started,
            })
        }
        body => {
            log["responseBytes"] = value!(match &body {
                ResponseBody::Body(bytes) => bytes.len(),
                _ => 0,
            });
            record(log);
            Ok(builder.body(body))
        }
    }
}

pub async fn fetch(mut request: Request, env: Env) -> Result<Response> {
    let path = request.path();
    if path == "/" {
        return json(
            &value!({"status":"ok","service":"library-client-sync"}),
            200,
        );
    }
    if path == "/live/session" || path == "/live/ws" {
        return auth::route(request, &env).await;
    }
    let mut response = if request.method() == Method::Options {
        Response::empty()?.with_status(204)
    } else if path == "/api/health" {
        json(
            &value!({"status":"ok","backend":"cloudflare-durable-object","edge":"photon-edge-worker","cloudEngineBaseUrl":base(&env)}),
            200,
        )?
    } else if path == "/__debug/sync" {
        json(
            &value!({"edge":{"status":"ok","role":"photon-edge-worker","cloudEngineBaseUrl":base(&env),"engineProxyPaths":PATHS,"logLimit":100},"logs":LOGS.with(|l|l.borrow().clone())}),
            200,
        )?
    } else if PATHS.contains(&path.as_str()) {
        let start = now();
        let id = header(&request, "x-photon-request-id")
            .or_else(|| header(&request, "x-request-id"))
            .unwrap_or(random_id(18)?);
        let target = format!("{}{path}", base(&env));
        let bytes =
            if matches!(request.method(), Method::Get | Method::Head) {
                Ok(Vec::new())
            } else {
                bounded_stream(request.stream()?, 1024 * 1024).await
            };
        let request_bytes =
            bytes.as_ref().map_or(1024 * 1024 + 1, Vec::len);
        let result = match bytes {
            Err(_) => json(
                &value!({"error":"request body too large","maxBytes":1024*1024}),
                413,
            )?,
            Ok(bytes) => {
                let h = Headers::new();
                h.set(
                    "content-type",
                    &header(&request, "content-type")
                        .unwrap_or_else(|| "application/json".into()),
                )?;
                h.set("x-photon-request-id", &id)?;
                if let Some(token) = header(&request, "authorization")
                    .or_else(|| {
                        variable(&env, "PHOTON_EDGE_SERVICE_TOKEN")
                            .map(|v| format!("Bearer {v}"))
                    })
                {
                    h.set("authorization", &token)?;
                }
                let mut init = RequestInit::new();
                init.with_method(request.method())
                    .with_headers(h)
                    .with_redirect(RequestRedirect::Manual);
                if !matches!(request.method(), Method::Get | Method::Head) {
                    init.with_body(Some(
                        js_sys::Uint8Array::from(bytes.as_slice()).into(),
                    ));
                }
                match fetch_timeout(
                    Request::new_with_init(&target, &init)?,
                    30_000,
                )
                .await
                {
                    Ok(response) => response,
                    Err(_) => json(
                        &value!({"error":"cloud engine proxy failed"}),
                        502,
                    )?,
                }
            }
        };
        let status = result.status_code();
        let log = value!({"event":"engine_proxy","id":random_id(18)?,"requestId":id,"timestamp":js_sys::Date::new_0().to_iso_string().as_string(),"method":request.method().to_string(),"path":path,"target":target,"status":status,"durationMs":now()-start,"requestBytes":request_bytes,"ok":(200..300).contains(&status)});
        if path == "/api/engine/push" && (200..300).contains(&status) {
            let h = Headers::new();
            h.set(INTERNAL, "1")?;
            let mut init = RequestInit::new();
            init.with_method(Method::Post).with_headers(h);
            let notification = Request::new_with_init(
                "https://sync.internal/internal/engine-changed",
                &init,
            )?;
            if env
                .durable_object("PHOTON_SYNC_ROOMS")?
                .get_by_name("records")?
                .fetch_with_request(notification)
                .await
                .is_err()
            {
                console_warn!(
                    "{{\"event\":\"engine_changed_broadcast_failed\"}}"
                );
            }
        }
        // Preserve the upstream response stream; diagnostics do not buffer it.
        let headers = result.headers().clone();
        let mut result = counted(result.with_headers(headers), log, start)?;
        result.headers_mut().set("x-photon-request-id", &id)?;
        result
    } else if path == "/ws" {
        let room = request
            .url()?
            .query_pairs()
            .find(|(k, _)| k == "room")
            .map(|(_, v)| v.into_owned())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "records".into());
        return env
            .durable_object("PHOTON_SYNC_ROOMS")?
            .get_by_name(&room)?
            .fetch_with_request(request)
            .await;
    } else {
        return Response::error("Not found", 404);
    };
    cors(&mut response)?;
    Ok(response)
}
