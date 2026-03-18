//! Middleware for handling IP injection

use std::net::SocketAddr;

use axum::{
    extract::ConnectInfo,
    http::{HeaderName, HeaderValue, Request},
    middleware::Next,
    response::Response,
};
use tracing::debug;

// Inject the client IP into request headers as X-Forwarded-For if it's not already present,
// used for enforcing rate limits.
pub async fn inject_client_ip(mut req: Request<axum::body::Body>, next: Next) -> Response {
    let cf_name: HeaderName = HeaderName::from_static("cf-connecting-ip");
    let xff_name: HeaderName = HeaderName::from_static("x-forwarded-for");

    // Determine effective client IP and source priority: CF-Connecting-IP > X-Forwarded-For (first) > socket
    let mut chosen_ip: Option<String> = None;
    let mut source = "unknown";

    if let Some(val) = req.headers().get(&cf_name) {
        if let Ok(s) = val.to_str() {
            chosen_ip = Some(s.trim().to_string());
            source = "cf-connecting-ip";
        }
    }

    if chosen_ip.is_none() {
        if let Some(val) = req.headers().get(&xff_name) {
            if let Ok(s) = val.to_str() {
                let first = s.split(',').next().map(|v| v.trim()).unwrap_or("");
                if !first.is_empty() {
                    chosen_ip = Some(first.to_string());
                    source = "x-forwarded-for";
                }
            }
        }
    }

    if chosen_ip.is_none() {
        if let Some(connect_info) = req.extensions().get::<ConnectInfo<SocketAddr>>() {
            chosen_ip = Some(connect_info.0.ip().to_string());
            source = "socket";
            // Also inject XFF so downstream extractors see it uniformly
            if let Ok(value) = HeaderValue::from_str(chosen_ip.as_ref().unwrap()) {
                let _ = req.headers_mut().insert(xff_name.clone(), value);
            }
        }
    }

    if let Some(ip) = &chosen_ip {
        debug!("client_ip_source={} ip={}", source, ip);
    } else {
        debug!("client_ip_source=unavailable");
    }

    next.run(req).await
}
