//! Developer-only account tooling — the *fake* sign-in.
//!
//! These endpoints let a developer enumerate every test account, switch the
//! browser's session to any of them, and mint new ones by handle alone (no
//! password) — so a single browser can act out a whole demos from several points
//! of view.
//!
//! Two gates, both required, keep this inert in any real deployment:
//!
//! 1. `--dev` ([`crate::AppState::dev_mode`]) must be on. Off by default; never set in
//!    production.
//! 2. The browser must carry the [`DEV_COOKIE`](dev_cookie::DEV_COOKIE) unlock cookie, which only the
//!    `--dev` server hands out (via [`unlock`](unlock::unlock)).
//!
//! Every gated handler returns `404` when either gate is closed, so nothing here
//! is reachable — or even discoverable — otherwise. The accompanying `dev.js`
//! paints the floating switch bar and self-hides when [`accounts`](accounts::accounts) 404s.

pub mod accounts;
pub mod create;
pub mod create_form;
pub mod dev_cookie;
pub mod dev_cookie_value;
pub mod dev_js;
pub mod dev_session;
pub mod dev_unlocked;
pub mod login_as_handle;
pub mod no_content_with_cookie;
pub mod switch;
pub mod switch_form;
pub mod unlock;
pub mod unlock_query;
