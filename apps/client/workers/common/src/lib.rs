use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use futures::StreamExt;
use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use wasm_bindgen::{JsCast, JsValue};
use worker::*;

const COMPONENT: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'!')
    .remove(b'~')
    .remove(b'*')
    .remove(b'\'')
    .remove(b'(')
    .remove(b')');

pub fn encode(value: &str) -> String {
    utf8_percent_encode(value, COMPONENT).to_string()
}

pub fn decode(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    for (i, &byte) in bytes.iter().enumerate() {
        if byte == b'%'
            && (i + 2 >= bytes.len()
                || !bytes[i + 1].is_ascii_hexdigit()
                || !bytes[i + 2].is_ascii_hexdigit())
        {
            return None;
        }
    }
    percent_encoding::percent_decode_str(value)
        .decode_utf8()
        .ok()
        .map(|s| s.into_owned())
}

pub fn variable(env: &Env, name: &str) -> Option<String> {
    env.var(name).ok().map(|v| v.to_string())
}

pub fn header(req: &Request, name: &str) -> Option<String> {
    req.headers().get(name).ok().flatten()
}

pub fn json(value: &serde_json::Value, status: u16) -> Result<Response> {
    Ok(Response::from_json(value)?.with_status(status))
}

pub fn now() -> i64 {
    js_sys::Date::now() as i64
}

pub fn random_id(bytes: usize) -> Result<String> {
    let scope: web_sys::WorkerGlobalScope =
        js_sys::global().unchecked_into();
    let mut buffer = vec![0; bytes];
    scope
        .crypto()?
        .get_random_values_with_u8_array(&mut buffer)?;
    Ok(URL_SAFE_NO_PAD.encode(buffer))
}

pub fn hash(value: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(value.as_bytes()))
}

/// Stop reading as soon as the cap is exceeded; never buffer an unbounded API body.
pub async fn bounded_stream(
    mut stream: ByteStream,
    limit: usize,
) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        if bytes.len().saturating_add(chunk.len()) > limit {
            return Err(Error::RustError("body too large".into()));
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

pub async fn request_json(
    req: &mut Request,
    limit: usize,
) -> Result<serde_json::Value> {
    Ok(serde_json::from_slice(
        &bounded_stream(req.stream()?, limit).await?,
    )?)
}

pub async fn response_json<T: DeserializeOwned>(
    response: &mut Response,
    limit: usize,
) -> Result<T> {
    Ok(serde_json::from_slice(
        &bounded_stream(response.stream()?, limit).await?,
    )?)
}

pub async fn fetch_timeout(
    request: Request,
    timeout_ms: u32,
) -> Result<Response> {
    let signal = AbortSignal::from(web_sys::AbortSignal::timeout_with_u32(
        timeout_ms,
    ));
    Fetch::Request(request).send_with_signal(&signal).await
}

pub fn api_request(
    url: &str,
    method: Method,
    headers: Headers,
    body: Option<&serde_json::Value>,
) -> Result<Request> {
    let mut init = RequestInit::new();
    init.with_method(method)
        .with_headers(headers)
        .with_redirect(RequestRedirect::Manual);
    if let Some(body) = body {
        init.with_body(Some(JsValue::from_str(&body.to_string())));
    }
    Request::new_with_init(url, &init)
}
