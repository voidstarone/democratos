//! The sensitive-content review console (reviewers only).

use axum::{
    extract::State,
    http::HeaderMap,
    response::Response,
};
use domain::{ReportTarget, REVIEW_QUORUM};

use crate::handlers::current_user::current_user;
use crate::handlers::render::render;
use crate::handlers::render_error::render_error;
use crate::handlers::resolve_lang::resolve_lang;
use crate::views::media_item::MediaItem;
use crate::views::review_item::ReviewItem;
use crate::views::review_view::ReviewView;
use crate::AppState;

/// The platform-wide review queue. Visible only to accounts that have opted in to
/// sensitive-content review; the service layer enforces that gate.
pub async fn review_page(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let lang = resolve_lang(&headers);
    let Some(user) = current_user(&state, &headers).await else {
        return render_error(lang, None, "sign in first".to_string());
    };
    let cases = match state.services.list_review_queue(user.id).await {
        Ok(c) => c,
        Err(e) => return render_error(lang, Some(user.handle), e.to_string()),
    };

    let mut items = Vec::with_capacity(cases.len());
    for case in cases {
        let (kind, title, body, media) = match case.target {
            ReportTarget::Post(p) => match state.services.posts.get(p).await {
                Ok(Some(post)) => {
                    let media = post
                        .media
                        .iter()
                        .map(|m| MediaItem {
                            url: m.url.clone(),
                            is_video: m.is_video,
                            caption: m.caption.clone(),
                        })
                        .collect();
                    ("post", post.title, post.body, media)
                }
                // Target vanished (e.g. already removed) — show a stub row.
                _ => ("post", String::new(), "(content unavailable)".into(), Vec::new()),
            },
            ReportTarget::Comment(c) => match state.services.comments.get(c).await {
                Ok(Some(comment)) => ("comment", String::new(), comment.body, Vec::new()),
                _ => ("comment", String::new(), "(content unavailable)".into(), Vec::new()),
            },
            ReportTarget::User(_) => continue,
        };
        items.push(ReviewItem {
            case_id: case.id.0,
            kind,
            title,
            body,
            media,
            votes: case.reviewer_count(),
            quorum: REVIEW_QUORUM,
            already_voted: case.has_voted(user.id),
            note: case.note,
        });
    }

    render(ReviewView {
        t: lang.strings(),
        lang: lang.code(),
        current_user: Some(user.handle),
        items,
    })
}
