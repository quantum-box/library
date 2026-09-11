mod auth;
mod checkpoint;
mod document;
mod edge;
mod external_dispatch;
mod live;
mod model;
mod outbound_scan;
mod relay;
mod storage;
mod tickets;

use worker::*;
#[event(fetch)]
pub async fn fetch(
    request: Request,
    env: Env,
    _ctx: Context,
) -> Result<Response> {
    edge::fetch(request, env).await
}

#[event(scheduled)]
pub async fn scheduled(
    _event: ScheduledEvent,
    env: Env,
    _ctx: ScheduleContext,
) {
    if let Err(error) = outbound_scan::run(&env).await {
        console_error!("external sync scheduled scan failed: {error}");
    }
}
