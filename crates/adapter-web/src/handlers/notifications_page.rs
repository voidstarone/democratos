//! The notifications page: a member's recent mentions and jury summons.

use axum::{
    extract::State,
    http::HeaderMap,
    response::{IntoResponse, Redirect, Response},
};
use domain::NotificationKind;

use crate::handlers::current_user::current_user;
use crate::handlers::handle_of::handle_of;
use crate::handlers::render::render;
use crate::handlers::resolve_lang::resolve_lang;
use crate::i18n::lang::Lang;
use crate::views::notification_row::NotificationRow;
use crate::views::notifications_view::NotificationsView;
use crate::AppState;

/// `GET /notifications` — the signed-in member's recent notifications, newest
/// first. Opening the page marks them all seen, so the toolbar badge clears.
pub async fn notifications_page(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let lang = resolve_lang(&headers);
    let Some(user) = current_user(&state, &headers).await else {
        return Redirect::to("/signin").into_response();
    };

    let notes = state
        .services
        .notifications(user.id)
        .await
        .unwrap_or_default();

    let mut rows = Vec::new();
    for n in &notes {
        rows.push(NotificationRow {
            href: href_for(&n.kind),
            summary: summary_for(&state, lang, &n.kind).await,
            is_unseen: !n.seen,
        });
    }

    // Seeing the list clears the unread badge.
    let _ = state.services.mark_notifications_seen(user.id).await;

    render(NotificationsView {
        t: lang.strings(),
        lang: lang.code(),
        current_user: Some(user.handle),
        rows,
    })
}

/// The same-site link a notification points at.
fn href_for(kind: &NotificationKind) -> String {
    match kind {
        NotificationKind::Mention { post, comment, .. } => match comment {
            Some(c) => format!("/post/{}#c{}", post.0, c.0),
            None => format!("/post/{}", post.0),
        },
        NotificationKind::JurySummons { trial, .. } => format!("/trial/{}", trial.0),
        NotificationKind::TrialComment { trial, .. } => format!("/trial/{}", trial.0),
    }
}

/// The localized one-line summary for a notification.
async fn summary_for(state: &AppState, lang: Lang, kind: &NotificationKind) -> String {
    let t = lang.strings();
    match kind {
        NotificationKind::Mention { by, .. } => {
            let who = handle_of(state, *by).await;
            format!("@{who} {}", t.notif_mentioned_you)
        }
        NotificationKind::JurySummons { .. } => t.notif_jury_summons.to_string(),
        NotificationKind::TrialComment { by, .. } => {
            let who = handle_of(state, *by).await;
            format!("@{who} {}", t.notif_trial_comment)
        }
    }
}
