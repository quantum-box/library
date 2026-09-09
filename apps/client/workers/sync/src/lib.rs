mod auth;
mod checkpoint;
mod document;
mod edge;
mod live;
mod model;
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
