/// Name of the cookie that unlocks the fake sign-in. Its presence with the
/// expected value is the second gate; because only a `--dev` server sets it (and
/// `--dev` is itself the first gate), a fixed sentinel value is sufficient.
pub const DEV_COOKIE: &str = "dev_login";
