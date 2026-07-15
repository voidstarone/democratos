//! `POST /dev/trial/as` — assume a trial-theater seat's session.
//!
//! Unlike the general puppet switcher ([`super::switch`]), this can point the
//! session at an *enfranchised* account — a juror must be a voter. To keep that
//! from becoming a "become any real member" tool, it is scoped to members of the
//! demo court (the seeded throwaway community): a non-participant id is refused.

use axum::{
    extract::{Form, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use domain::UserId;

use crate::dev::act_as_form::ActAsForm;
use crate::dev::dev_unlocked::dev_unlocked;
use crate::dev::seed_trial::DEMO_COURT_SLUG;
use crate::handlers::redirect_with_cookie::redirect_with_cookie;
use crate::handlers::safe_next::safe_next;
use crate::handlers::uid_cookie::uid_cookie;
use crate::AppState;

pub async fn act_as(
    State(state): State<AppState>,
    headers: HeaderMap,
    Form(form): Form<ActAsForm>,
) -> Response {
    if !dev_unlocked(&state, &headers) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let svc = &state.services;
    // Scope: the target must belong to the demo court. This bounds session
    // assumption to seeded demo accounts — never an arbitrary real member.
    let Some(demos) = svc.demoi.by_slug(DEMO_COURT_SLUG).await.ok().flatten() else {
        return (StatusCode::NOT_FOUND, "nothing seeded yet").into_response();
    };
    let target = UserId(form.id);
    if svc
        .memberships
        .get(target, demos.id)
        .await
        .ok()
        .flatten()
        .is_none()
    {
        return (StatusCode::FORBIDDEN, "not a demo-court participant").into_response();
    }
    let next = safe_next(&form.next).unwrap_or_else(|| "/dev/trial".to_string());
    redirect_with_cookie(
        &next,
        uid_cookie(
            &state.session,
            target.0,
            svc.clock.now().0,
            state.secure_cookies,
        ),
    )
}
