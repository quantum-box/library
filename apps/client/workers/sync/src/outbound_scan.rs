use crate::auth;
use library_worker_common::{api_request, fetch_timeout, variable};
use serde_json::json;
use worker::*;

const ENABLED: &str = "EXTERNAL_SYNC_SCANNER_ENABLED";
const TOKEN: &str = "EXTERNAL_SYNC_SCANNER_TOKEN";

pub async fn run(env: &Env) -> Result<()> {
    let enabled = variable(env, ENABLED).is_some_and(|value| {
        value.trim().eq_ignore_ascii_case("true")
    });
    if !enabled {
        return Ok(());
    }
    let token = env.secret(TOKEN)?.to_string();
    let headers = Headers::new();
    headers.set("authorization", &format!("Bearer {token}"))?;
    headers.set("content-type", "application/json")?;
    let request = api_request(
        &format!(
            "{}/internal/external-sync/outbound-scan",
            auth::api_base(env)
        ),
        Method::Post,
        headers,
        Some(&json!({})),
    )?;
    let response = fetch_timeout(request, 55_000).await?;
    if !(200..300).contains(&response.status_code()) {
        return Err(format!(
            "scanner API returned HTTP {}",
            response.status_code()
        )
        .into());
    }
    Ok(())
}
