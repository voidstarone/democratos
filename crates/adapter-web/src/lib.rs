//! Democratos web adapter — a driving adapter delivering the application over
//! HTTP as server-rendered, progressively-enhanced, translatable HTML.
//!
//! It exposes [`router()`]; the composition root supplies a fully-wired
//! [`app::Services`] and runs the server. The web layer holds no business logic
//! and no knowledge of which store backs the application.

mod app_state;
mod dev;
mod handlers;
mod i18n;
mod rate_limit;
mod router;
mod security_headers;
mod serve;
mod serve_local;
mod views;

pub use app_state::AppState;
pub use router::router;
pub use serve::serve;
pub use serve_local::serve_local;
