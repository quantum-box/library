use crate::model::INTERNAL;
use library_worker_common::{
    api_request, fetch_timeout, header, json, now, request_json,
};
use serde::{Deserialize, Serialize};
use serde_json::json as value;
use worker::*;

const JOB: &str = "external-sync:job";
const MAX_BODY: usize = 4 * 1024;
const MAX_DELAY_MS: i64 = 5 * 60 * 1000;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DispatchJob {
    event_id: String,
    capability: String,
    callback_url: String,
    attempt: u32,
}

fn valid_event_id(value: &str) -> bool {
    value.starts_with("wev_")
        && (20..=64).contains(&value.len())
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || byte == b'_'
        })
}

fn valid_capability(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn valid_callback(value: &str) -> bool {
    let Ok(url) = Url::parse(value) else {
        return false;
    };
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || (url.path() != "/" && !url.path().is_empty())
    {
        return false;
    }
    let Some(host) = url.host_str() else {
        return false;
    };
    let production = url.scheme() == "https"
        && (host == "library-api.txcloud.app"
            || (host.starts_with("pr")
                && host.ends_with("--library-api.txcloud.app")
                && host
                    [2..host.len() - "--library-api.txcloud.app".len()]
                    .bytes()
                    .all(|byte| byte.is_ascii_digit())));
    let local =
        url.scheme() == "http" && matches!(host, "127.0.0.1" | "localhost");
    production || local
}

fn endpoint(job: &DispatchJob, action: &str) -> String {
    format!(
        "{}/internal/external-sync/{action}",
        job.callback_url.trim_end_matches('/')
    )
}

fn callback_request(job: &DispatchJob, action: &str) -> Result<Request> {
    let headers = Headers::new();
    headers.set("content-type", "application/json")?;
    api_request(
        &endpoint(job, action),
        Method::Post,
        headers,
        Some(&value!({
            "event_id": job.event_id,
            "capability": job.capability,
        })),
    )
}

async fn callback_fetch(
    request: Request,
    env: &Env,
    timeout_ms: u32,
) -> Result<Response> {
    if let Ok(proxy) = env.service("TXCLOUD_PROXY") {
        return proxy.fetch_request(request).await;
    }
    fetch_timeout(request, timeout_ms).await
}

pub async fn enqueue(mut request: Request, env: &Env) -> Result<Response> {
    if request.method() != Method::Post {
        return Response::error("Method not allowed", 405);
    }
    let input = match request_json(&mut request, MAX_BODY).await {
        Ok(value) => value,
        Err(_) => return Response::error("Invalid dispatch request", 400),
    };
    let Some(event_id) = input["event_id"].as_str() else {
        return Response::error("Invalid event id", 400);
    };
    let Some(capability) = input["capability"].as_str() else {
        return Response::error("Invalid capability", 400);
    };
    let Some(callback_url) = input["callback_url"].as_str() else {
        return Response::error("Invalid callback URL", 400);
    };
    if !valid_event_id(event_id)
        || !valid_capability(capability)
        || !valid_callback(callback_url)
    {
        return Response::error("Invalid dispatch request", 400);
    }

    let job = DispatchJob {
        event_id: event_id.into(),
        capability: capability.to_ascii_lowercase(),
        callback_url: callback_url.trim_end_matches('/').into(),
        attempt: 0,
    };
    let validation = match callback_fetch(
        callback_request(&job, "validate")?,
        env,
        10_000,
    )
    .await
    {
        Ok(response) => response,
        Err(_) => return Response::error("Validation unavailable", 503),
    };
    if validation.status_code() != 204 {
        return Response::error(
            "Dispatch capability was not accepted",
            if validation.status_code() == 401 {
                401
            } else {
                503
            },
        );
    }

    let headers = Headers::new();
    headers.set(INTERNAL, "1")?;
    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_headers(headers)
        .with_body(Some(wasm_bindgen::JsValue::from_str(
            &serde_json::to_string(&job)?,
        )));
    let response = env
        .durable_object("EXTERNAL_SYNC_DISPATCH")?
        .get_by_name(event_id)?
        .fetch_with_request(Request::new_with_init(
            "https://external-sync.internal/schedule",
            &init,
        )?)
        .await?;
    if response.status_code() != 202 {
        return Response::error("Durable dispatch unavailable", 503);
    }
    json(&value!({"event_id": event_id, "status": "scheduled"}), 202)
}

#[durable_object]
pub struct ExternalSyncDispatcher {
    state: State,
    env: Env,
}

impl ExternalSyncDispatcher {
    async fn reschedule(&self, mut job: DispatchJob) -> Result<()> {
        job.attempt = job.attempt.saturating_add(1);
        self.state.storage().put(JOB, &job).await?;
        let exponent = job.attempt.min(8);
        let delay = (2_i64.pow(exponent) * 1_000).min(MAX_DELAY_MS);
        self.state
            .storage()
            .set_alarm(ScheduledTime::new(js_sys::Date::new(
                &wasm_bindgen::JsValue::from_f64((now() + delay) as f64),
            )))
            .await
    }
}

impl DurableObject for ExternalSyncDispatcher {
    fn new(state: State, env: Env) -> Self {
        Self { state, env }
    }

    async fn fetch(&self, mut request: Request) -> Result<Response> {
        if request.method() != Method::Post
            || request.path() != "/schedule"
            || header(&request, INTERNAL).as_deref() != Some("1")
        {
            return Response::error("Not found", 404);
        }
        let job: DispatchJob = match serde_json::from_value(
            request_json(&mut request, MAX_BODY).await?,
        ) {
            Ok(job) => job,
            Err(_) => return Response::error("Invalid dispatch job", 400),
        };
        if !valid_event_id(&job.event_id)
            || !valid_capability(&job.capability)
            || !valid_callback(&job.callback_url)
        {
            return Response::error("Invalid dispatch job", 400);
        }
        self.state.storage().put(JOB, &job).await?;
        self.state
            .storage()
            .set_alarm(ScheduledTime::new(js_sys::Date::new(
                &wasm_bindgen::JsValue::from_f64(now() as f64),
            )))
            .await?;
        json(&value!({"status": "scheduled"}), 202)
    }

    async fn alarm(&self) -> Result<Response> {
        let storage = self.state.storage();
        let Some(job) = storage.get::<DispatchJob>(JOB).await? else {
            storage.delete_alarm().await?;
            return Response::empty();
        };
        match callback_fetch(
            callback_request(&job, "process")?,
            &self.env,
            30_000,
        )
        .await
        {
            Ok(response)
                if (200..300).contains(&response.status_code()) =>
            {
                storage.delete_all().await?;
            }
            Ok(response) => {
                console_warn!(
                    "external sync callback returned HTTP {} for {}",
                    response.status_code(),
                    job.event_id
                );
                self.reschedule(job).await?;
            }
            Err(error) => {
                console_warn!(
                    "external sync callback failed for {}: {}",
                    job.event_id,
                    error
                );
                self.reschedule(job).await?;
            }
        }
        Response::empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn callback_allowlist_is_narrow() {
        assert!(valid_callback("https://library-api.txcloud.app"));
        assert!(valid_callback("https://pr355--library-api.txcloud.app"));
        assert!(valid_callback("http://127.0.0.1:50055"));
        assert!(!valid_callback(
            "https://library-api.txcloud.app.evil.test"
        ));
        assert!(!valid_callback("https://library-api.txcloud.app/path"));
    }
}
