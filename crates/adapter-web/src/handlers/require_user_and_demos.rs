//! Resolve the signed-in user and the demos by slug, or render an error page.

/// Resolve the signed-in user and the demos by slug, or render an error page.
macro_rules! require_user_and_demos {
    ($state:expr, $headers:expr, $lang:expr, $slug:expr) => {{
        let Some(user) =
            crate::handlers::current_user::current_user(&$state, &$headers).await
        else {
            return crate::handlers::render_error::render_error(
                $lang,
                None,
                "sign in first".to_string(),
            );
        };
        match $state.services.demoi.by_slug(&$slug).await {
            Ok(Some(d)) => (user, d),
            Ok(None) => {
                return crate::handlers::render_error::render_error(
                    $lang,
                    Some(user.handle),
                    "no such demos".to_string(),
                )
            }
            Err(e) => {
                return crate::handlers::render_error::render_error(
                    $lang,
                    Some(user.handle),
                    e.to_string(),
                )
            }
        }
    }};
}

pub(crate) use require_user_and_demos;
