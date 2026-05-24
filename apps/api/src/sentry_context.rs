use std::net::{IpAddr, SocketAddr};

use axum::extract::{ConnectInfo, MatchedPath, Request};
use axum::http::HeaderMap;
use axum::middleware::Next;
use axum::response::Response;
use sentry::protocol::IpAddress;
use tachyon_sdk::auth::{MultiTenancyAction, ServiceAccount, User};
use telemetry::http::REQUEST_ID_HEADER;

use crate::handler::library_executor_extractor::{
    LibraryExecutor, LibraryExecutorKind,
};

const X_FORWARDED_FOR: &str = "x-forwarded-for";
const X_REAL_IP: &str = "x-real-ip";
const USER_AGENT: &str = "user-agent";

#[derive(Debug, Clone, Copy)]
pub struct SentryRequestContext {
    pub client_ip: Option<IpAddr>,
}

pub async fn sentry_request_context_middleware(
    mut request: Request,
    next: Next,
) -> Response {
    let context = configure_request_scope(&request);
    request.extensions_mut().insert(context);
    next.run(request).await
}

pub fn configure_request_scope<B>(
    request: &Request<B>,
) -> SentryRequestContext {
    let endpoint = endpoint(request);
    let method = request.method().to_string();
    let request_id = header_value(request.headers(), REQUEST_ID_HEADER);
    let user_agent = header_value(request.headers(), USER_AGENT);
    let client_ip = client_ip(request.headers(), remote_addr(request));

    sentry::configure_scope(|scope| {
        clear_request_scope(scope);
        scope.set_tag("endpoint", format!("{method} {endpoint}"));
        if let Some(request_id) = request_id {
            scope.set_tag("request_id", request_id);
        }
        if let Some(user_agent) = user_agent {
            scope.set_tag("user_agent", user_agent);
        }
        if let Some(client_ip) = client_ip {
            scope.set_tag("client.ip", client_ip.to_string());
        }
    });

    SentryRequestContext { client_ip }
}

pub fn configure_multitenancy_scope(
    multi_tenancy: &tachyon_sdk::auth::MultiTenancy,
) {
    let tenant_id = multi_tenancy
        .operator_id()
        .or_else(|| multi_tenancy.platform_id());

    if let Some(tenant_id) = tenant_id {
        sentry::configure_scope(|scope| {
            scope.set_tag("tenant_id", tenant_id.to_string());
        });
    }
}

pub fn configure_executor_scope(
    executor: &LibraryExecutor,
    context: Option<SentryRequestContext>,
) {
    match &executor.inner {
        LibraryExecutorKind::User(user) => {
            configure_user_scope(user, context);
        }
        LibraryExecutorKind::ServiceAccount(sa) => {
            configure_service_account_scope(sa);
        }
        LibraryExecutorKind::None => {
            clear_auth_scope();
        }
    }
}

fn configure_user_scope(
    user: &User,
    context: Option<SentryRequestContext>,
) {
    sentry::configure_scope(|scope| {
        scope.set_tag("user_id", user.id().to_string());
        if let Some(email) = user.email() {
            scope.set_tag("user_email", email);
        }
        scope.set_user(Some(sentry::User {
            id: Some(user.id().to_string()),
            email: user.email().map(ToOwned::to_owned),
            username: Some(user.username().to_string()),
            ip_address: context
                .and_then(|context| context.client_ip)
                .map(IpAddress::Exact),
            ..Default::default()
        }));
    });
}

fn configure_service_account_scope(service_account: &ServiceAccount) {
    sentry::configure_scope(|scope| {
        scope.set_tag("tenant_id", service_account.tenant_id().to_string());
        scope.set_tag(
            "service_account_id",
            service_account.id().to_string(),
        );
    });
}

fn clear_request_scope(scope: &mut sentry::Scope) {
    scope.remove_tag("endpoint");
    scope.remove_tag("request_id");
    scope.remove_tag("user_agent");
    scope.remove_tag("client.ip");
    scope.remove_tag("tenant_id");
    scope.remove_tag("user_id");
    scope.remove_tag("user_email");
    scope.remove_tag("service_account_id");
    scope.set_user(None);
}

fn clear_auth_scope() {
    sentry::configure_scope(|scope| {
        scope.remove_tag("tenant_id");
        scope.remove_tag("user_id");
        scope.remove_tag("user_email");
        scope.remove_tag("service_account_id");
        scope.set_user(None);
    });
}

fn endpoint<B>(request: &Request<B>) -> String {
    request
        .extensions()
        .get::<MatchedPath>()
        .map(MatchedPath::as_str)
        .unwrap_or_else(|| request.uri().path())
        .to_string()
}

fn remote_addr<B>(request: &Request<B>) -> Option<SocketAddr> {
    request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|connect_info| connect_info.0)
}

fn client_ip(
    headers: &HeaderMap,
    remote_addr: Option<SocketAddr>,
) -> Option<IpAddr> {
    forwarded_for_ip(headers)
        .or_else(|| header_ip(headers, X_REAL_IP))
        .or_else(|| remote_addr.map(|addr| addr.ip()))
}

fn forwarded_for_ip(headers: &HeaderMap) -> Option<IpAddr> {
    header_value(headers, X_FORWARDED_FOR).and_then(|value| {
        value
            .split(',')
            .map(str::trim)
            .find(|part| !part.is_empty())
            .and_then(parse_ip)
    })
}

fn header_ip(headers: &HeaderMap, name: &str) -> Option<IpAddr> {
    header_value(headers, name).and_then(parse_ip)
}

fn header_value<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name)?.to_str().ok().filter(|v| !v.is_empty())
}

fn parse_ip(value: &str) -> Option<IpAddr> {
    value.parse::<IpAddr>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn client_ip_prefers_forwarded_for_then_real_ip_then_remote_addr() {
        let remote = Some(SocketAddr::from(([192, 0, 2, 3], 443)));
        let mut headers = HeaderMap::new();
        headers.insert(X_REAL_IP, HeaderValue::from_static("192.0.2.2"));
        headers.insert(
            X_FORWARDED_FOR,
            HeaderValue::from_static("198.51.100.1, 192.0.2.1"),
        );

        assert_eq!(
            client_ip(&headers, remote),
            Some("198.51.100.1".parse().unwrap())
        );

        headers.remove(X_FORWARDED_FOR);
        assert_eq!(
            client_ip(&headers, remote),
            Some("192.0.2.2".parse().unwrap())
        );

        headers.remove(X_REAL_IP);
        assert_eq!(
            client_ip(&headers, remote),
            Some("192.0.2.3".parse().unwrap())
        );
    }

    #[test]
    fn forwarded_for_ignores_empty_leading_values() {
        let mut headers = HeaderMap::new();
        headers.insert(
            X_FORWARDED_FOR,
            HeaderValue::from_static(" , 203.0.113.9"),
        );

        assert_eq!(
            forwarded_for_ip(&headers),
            Some("203.0.113.9".parse().unwrap())
        );
    }
}
