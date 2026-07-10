/// Strict allowance for the credential endpoints (`/session`, `/register`): at
/// most this many attempts per IP per [`AUTH_WINDOW`]. Sized to allow a human's
/// fat-fingered retries while throttling automated guessing and the Argon2 cost
/// that rides on each attempt.
pub const AUTH_MAX_REQUESTS: u32 = 10;
