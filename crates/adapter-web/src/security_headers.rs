//! Conservative security headers applied to every response.

use axum::{
    extract::Request,
    http::{header::HeaderName, HeaderValue},
    middleware::Next,
    response::Response,
};

/// Attach conservative security headers to every response. Output is already
/// HTML-escaped by Askama (the primary XSS defense); these are the cheap second
/// lines: block framing (clickjacking of the vote / found / jury forms), stop
/// MIME sniffing, trim the `Referer`, force HTTPS on repeat visits, and constrain
/// base-uri / objects / form targets.
///
/// `script-src` is now `'self'` only — every previously-inline script and inline
/// event handler has been externalized into `/static/*.js`, so an injected
/// `<script>` (should escaping ever be bypassed) cannot execute. `style-src`
/// retains `'unsafe-inline'`: layout still relies on many static `style="…"`
/// attributes, and inline *styles* (unlike scripts) can't execute code, so the
/// residual risk is cosmetic. Adding a nonce here would silently disable
/// `'unsafe-inline'` (CSP2 back-compat) and break every such attribute, so the
/// two are deliberately not mixed.
///
/// `Strict-Transport-Security` is sent unconditionally. It is only honoured over
/// a TLS origin (a browser ignores it on plain HTTP), so it is harmless in local
/// HTTP development and pins HTTPS for the production deployments that terminate
/// TLS.
pub(crate) async fn security_headers(req: Request, next: Next) -> Response {
    const HEADERS: &[(&str, &str)] = &[
        ("x-frame-options", "DENY"),
        ("x-content-type-options", "nosniff"),
        ("referrer-policy", "same-origin"),
        (
            "strict-transport-security",
            "max-age=63072000; includeSubDomains",
        ),
        (
            "content-security-policy",
            "default-src 'self'; img-src 'self' https: data:; media-src 'self' https:; \
             script-src 'self'; style-src 'self' 'unsafe-inline'; \
             object-src 'none'; base-uri 'self'; frame-ancestors 'none'; form-action 'self'",
        ),
    ];
    let mut resp = next.run(req).await;
    let headers = resp.headers_mut();
    for (name, value) in HEADERS {
        headers.insert(
            HeaderName::from_static(name),
            HeaderValue::from_static(value),
        );
    }
    resp
}
