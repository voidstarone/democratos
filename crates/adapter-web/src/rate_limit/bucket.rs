use std::time::Duration;

use crate::rate_limit::auth_max_requests::AUTH_MAX_REQUESTS;
use crate::rate_limit::auth_window::AUTH_WINDOW;
use crate::rate_limit::write_max_requests::WRITE_MAX_REQUESTS;
use crate::rate_limit::write_window::WRITE_WINDOW;

/// Which allowance a request falls under.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Bucket {
    /// `POST /session` and `POST /register`.
    Auth,
    /// Any other state-changing POST.
    Write,
}

impl Bucket {
    /// The `(limit, window)` pair for this bucket.
    pub(crate) fn limits(self) -> (u32, Duration) {
        match self {
            Bucket::Auth => (AUTH_MAX_REQUESTS, AUTH_WINDOW),
            Bucket::Write => (WRITE_MAX_REQUESTS, WRITE_WINDOW),
        }
    }
}
