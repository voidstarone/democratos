//! Build the HTTP router for the application services and write gateway.

use std::sync::Arc;

use app::{GovernanceWrites, Services, SessionSigner};
use axum::{
    extract::DefaultBodyLimit,
    middleware::{from_fn, from_fn_with_state},
    routing::{get, post},
    Router,
};

use crate::app_state::AppState;
use crate::security_headers::security_headers;
use crate::{dev, handlers, rate_limit};

/// Build the HTTP router for the given application services and write gateway.
#[allow(clippy::too_many_arguments)]
pub fn router(
    services: Services,
    writes: Arc<dyn GovernanceWrites>,
    session: SessionSigner,
    dev_mode: bool,
    secure_cookies: bool,
    dev_unlock_secret: Option<Arc<str>>,
) -> Router {
    // One process-wide limiter, shared by the middleware across all connections.
    let limiter = Arc::new(rate_limit::rate_limiter::RateLimiter::new());
    Router::new()
        .route("/", get(handlers::index::index))
        .route("/top", get(handlers::top_page::top_page))
        .route("/signin", get(handlers::signin_page::signin_page))
        .route("/session", post(handlers::create_session::create_session))
        .route(
            "/register",
            get(handlers::register_page::register_page)
                .post(handlers::create_account::create_account),
        )
        .route("/logout", post(handlers::logout::logout))
        .route("/lang", post(handlers::set_lang::set_lang))
        .route(
            "/preferences",
            get(handlers::preferences_page::preferences_page)
                .post(handlers::set_preferences::set_preferences),
        )
        // Enrol the account's Ed25519 public signing key (browser keeps the secret).
        .route("/account/key", post(handlers::enroll_key::enroll_key))
        // Founding is a two-step petition: name it here, then gather nine
        // sign-offs on the petition's own page before the demos is born.
        .route("/found", get(handlers::found_page::found_page))
        .route("/foundings", post(handlers::start_founding::start_founding))
        .route("/found/:id", get(handlers::founding_page::founding_page))
        .route("/found/:id/sign", post(handlers::sign_founding::sign_founding))
        .route("/d/:slug", get(handlers::demos_page::demos_page))
        .route("/d/:slug/join", post(handlers::join::join))
        .route("/d/:slug/enfranchise", post(handlers::enfranchise::enfranchise))
        .route("/d/:slug/proposals", get(handlers::proposals_page::proposals_page))
        .route(
            "/d/:slug/proposals/remove",
            post(handlers::propose_remove::propose_remove),
        )
        .route(
            "/d/:slug/proposals/amend",
            post(handlers::propose_amend::propose_amend),
        )
        .route("/p/:id/vote", post(handlers::vote::vote))
        .route("/p/:id/close", post(handlers::close_proposal::close_proposal))
        // content & moderation
        .route("/d/:slug/rules", post(handlers::propose_rule::propose_rule))
        .route(
            "/d/:slug/posting-policy",
            post(handlers::propose_posting_policy::propose_posting_policy),
        )
        // Global composer: pick a community, attach many media in one post.
        .route("/submit", get(handlers::submit_page::submit_page))
        .route(
            "/posts",
            // A post may carry several uploads, so raise the body limit (only
            // here) above the per-file cap — but only to `MAX_ATTACHMENTS` files'
            // worth plus a little slack for the text fields, not the old ~300 MB.
            // The handler also persists each part to the media store as it finishes
            // streaming, so peak *memory* stays near one file regardless.
            post(handlers::create_post::create_post).layer(DefaultBodyLimit::max(
                app::MAX_UPLOAD_BYTES * handlers::max_attachments::MAX_ATTACHMENTS + 1024 * 1024,
            )),
        )
        .route("/d/:slug/reports", get(handlers::reports_page::reports_page))
        .route("/search", get(handlers::search_page::search_page))
        .route("/media/:key", get(handlers::serve_media::serve_media))
        .route("/post/:id", get(handlers::post_page::post_page))
        .route("/post/:id/vote", post(handlers::post_vote::post_vote))
        .route("/post/:id/comments", post(handlers::add_comment::add_comment))
        .route("/comment/:id/vote", post(handlers::comment_vote::comment_vote))
        .route("/post/:id/report", post(handlers::report_post::report_post))
        .route("/report/:id/trial", post(handlers::open_trial::open_trial))
        .route("/trial/:id", get(handlers::trial_page::trial_page))
        .route("/trial/:id/vote", post(handlers::jury_vote::jury_vote))
        .route("/static/app.js", get(handlers::app_js::app_js))
        .route("/static/composer.js", get(handlers::composer_js::composer_js))
        // Dev account switcher — the fake sign-in. Inert unless `--dev` *and* the
        // browser holds the unlock cookie; every handler below 404s otherwise.
        // `/dev/unlock` (dev-mode only) is how a dev browser obtains that cookie.
        .route("/static/dev.js", get(dev::dev_js::dev_js))
        .route("/dev/unlock", get(dev::unlock::unlock))
        .route("/dev/session", post(dev::dev_session::dev_session))
        .route("/dev/accounts", get(dev::accounts::accounts))
        .route("/dev/switch", post(dev::switch::switch))
        .route("/dev/create", post(dev::create::create))
        // Rate limiting runs before the handlers (outermost of the two `layer`s
        // that touch every route) so a throttled request is rejected cheaply,
        // before any Argon2 work. It reads `ConnectInfo`, so it keys on the real
        // connection peer, never a spoofable header.
        .layer(from_fn_with_state(limiter, rate_limit::rate_limit::rate_limit))
        .layer(from_fn(security_headers))
        .with_state(AppState {
            services,
            writes,
            session,
            dev_mode,
            secure_cookies,
            dev_unlock_secret,
        })
}
