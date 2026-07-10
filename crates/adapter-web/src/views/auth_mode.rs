/// Which credential form the auth page shows.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AuthMode {
    /// Email + password sign-in.
    SignIn,
    /// Handle + email + password sign-up.
    Register,
}

impl AuthMode {
    /// `true` when rendering the registration form. Askama templates read this
    /// to branch, since they can't match on the enum directly.
    pub fn is_register(&self) -> bool {
        matches!(self, AuthMode::Register)
    }
}
