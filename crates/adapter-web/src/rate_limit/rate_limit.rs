use std::net::SocketAddr;
use std::time::Instant;

use axum::extract::Request;
use axum::middleware::Next;
use axum::{
    extract::{ConnectInfo, State},
    http::{Method, StatusCode},
    response::{IntoResponse, Response},
};

use crate::rate_limit::bucket::Bucket;
use crate::rate_limit::rate_limiter::RateLimiter;

/// Classify a request into a rate-limit bucket, or `None` for anything we don't
/// throttle (every non-POST method, and unknown paths handled elsewhere).
fn bucket_for(method: &Method, path: &str) -> Option<Bucket> {
    if method != Method::POST {
        return None;
    }
    match path {
        "/session" | "/register" => Some(Bucket::Auth),
        _ => Some(Bucket::Write),
    }
}

/// Axum middleware: throttle state-changing POSTs per client IP. Non-POST
/// requests pass straight through. Over the allowance returns `429` with a
/// `Retry-After`.
pub async fn rate_limit(
    State(limiter): State<std::sync::Arc<RateLimiter>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    req: Request,
    next: Next,
) -> Response {
    if let Some(bucket) = bucket_for(req.method(), req.uri().path()) {
        if !limiter.check_at(peer.ip(), bucket, Instant::now()) {
            let (_, window) = bucket.limits();
            return (
                StatusCode::TOO_MANY_REQUESTS,
                [("retry-after", window.as_secs().to_string())],
                "too many requests — please slow down and try again shortly",
            )
                .into_response();
        }
    }
    next.run(req).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_post_paths_are_bucketed() {
        assert_eq!(bucket_for(&Method::POST, "/session"), Some(Bucket::Auth));
        assert_eq!(bucket_for(&Method::POST, "/register"), Some(Bucket::Auth));
        assert_eq!(bucket_for(&Method::POST, "/p/1/vote"), Some(Bucket::Write));
        assert_eq!(bucket_for(&Method::POST, "/foundings"), Some(Bucket::Write));
        assert_eq!(bucket_for(&Method::GET, "/session"), None);
        assert_eq!(bucket_for(&Method::GET, "/"), None);
    }
}
