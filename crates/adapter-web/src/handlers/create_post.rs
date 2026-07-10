//! Create a post from the global composer (plus its media helpers).

use axum::{
    extract::{Multipart, State},
    http::HeaderMap,
    response::{IntoResponse, Redirect, Response},
};
use domain::{normalize_tags, Media};

use crate::handlers::current_user::current_user;
use crate::handlers::max_attachments::MAX_ATTACHMENTS;
use crate::handlers::render_error::render_error;
use crate::handlers::resolve_lang::resolve_lang;
use crate::AppState;

/// A media attachment as received before it is resolved to a stored [`Media`].
/// An upload is persisted to the media store *as it streams* (see
/// [`create_post`]) so we only ever hold its resulting URL here, never its bytes.
struct PendingMedia {
    kind: PendingKind,
    caption: String,
}

enum PendingKind {
    /// An uploaded file already streamed to, validated by, and stored in the media
    /// port. Only the resulting URL is retained — the bytes are dropped the moment
    /// the part finishes, so peak memory stays near one file, not the whole post.
    Stored { url: String, is_video: bool },
    /// A dragged-in link that is already a media URL (resolved at the end).
    Url { url: String },
}

impl PendingMedia {
    fn stored(url: String, is_video: bool) -> Self {
        PendingMedia {
            kind: PendingKind::Stored { url, is_video },
            caption: String::new(),
        }
    }
    fn url(url: String) -> Self {
        PendingMedia {
            kind: PendingKind::Url { url },
            caption: String::new(),
        }
    }
}

/// Accept a dragged-in media URL only if it is an `http(s)` link whose extension
/// maps to a supported type (the same allowlist uploads use). Returns whether it
/// is a video, or `None` if the link isn't acceptable. The server never fetches
/// it — the browser embeds it directly — so this is a shape check, not a probe.
fn classify_media_url(url: &str) -> Option<bool> {
    let lower = url.trim().to_ascii_lowercase();
    if !(lower.starts_with("http://") || lower.starts_with("https://")) {
        return None;
    }
    // Extension of the path, ignoring any query string or fragment.
    let path = lower.split(['?', '#']).next().unwrap_or(&lower);
    let ext = path.rsplit('.').next().filter(|e| !e.contains('/'))?;
    let content_type = app::content_type_for(ext)?;
    Some(content_type.starts_with("video/"))
}

/// Create a post from the global composer. Accepts `multipart/form-data`: the
/// target community (`demos` slug), a `title`, an optional text `body`, `tags`,
/// and any number of media attachments — each a `file` upload or a `media_url`
/// link, each optionally followed, in order, by its `caption`. Media type (image
/// vs video) is inferred from the upload's content type or the URL's extension.
pub async fn create_post(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Response {
    let lang = resolve_lang(&headers);
    let Some(user) = current_user(&state, &headers).await else {
        return render_error(lang, None, "sign in to post".to_string());
    };

    let (mut demos_slug, mut title, mut body, mut tags_raw) =
        (String::new(), String::new(), String::new(), String::new());
    // Media in submission order. Each is either an uploaded file (bytes to store)
    // or a dragged-in URL (already a media link); a trailing `caption` field
    // attaches to whichever media part preceded it.
    let mut media_parts: Vec<PendingMedia> = Vec::new();
    loop {
        let mut field = match multipart.next_field().await {
            Ok(Some(f)) => f,
            Ok(None) => break,
            Err(e) => return render_error(lang, Some(user.handle), e.to_string()),
        };
        match field.name().unwrap_or("") {
            "file" => {
                // Cap the number of attachments up front so a request can neither
                // buffer nor persist an unbounded set of files.
                if media_parts.len() >= MAX_ATTACHMENTS {
                    return render_error(
                        lang,
                        Some(user.handle),
                        format!("a post may carry at most {MAX_ATTACHMENTS} attachments"),
                    );
                }
                let ct = field.content_type().unwrap_or("").to_string();
                // Enforce the per-file cap while *streaming*, not after: reading
                // the whole field with `.bytes()` would buffer up to the aggregate
                // body limit into memory before we could reject it. Stop as soon as
                // we cross the cap.
                let mut buf: Vec<u8> = Vec::new();
                let mut over_cap = false;
                loop {
                    match field.chunk().await {
                        Ok(Some(chunk)) => {
                            if buf.len() + chunk.len() > app::MAX_UPLOAD_BYTES {
                                over_cap = true;
                                break;
                            }
                            buf.extend_from_slice(&chunk);
                        }
                        Ok(None) => break,
                        Err(e) => return render_error(lang, Some(user.handle), e.to_string()),
                    }
                }
                if over_cap {
                    return render_error(
                        lang,
                        Some(user.handle),
                        format!(
                            "one file exceeds the {} MB limit",
                            app::MAX_UPLOAD_BYTES / (1024 * 1024)
                        ),
                    );
                }
                if buf.is_empty() {
                    continue;
                }
                // Validate and persist this part *now*, then drop its bytes, so the
                // request never holds more than one file's worth of data at a time
                // (bounding peak memory regardless of how many files are attached).
                if !app::is_allowed(&ct) {
                    return render_error(
                        lang,
                        Some(user.handle),
                        format!("unsupported upload type: {ct}"),
                    );
                }
                // Don't trust the client's declared content type: the bytes must
                // actually be the media they claim to be, so a document uploaded as
                // `image/png` can't be stored and later served under an image type
                // the browser might sniff and execute.
                if !app::upload_matches_bytes(&ct, &buf) {
                    return render_error(
                        lang,
                        Some(user.handle),
                        "that file's contents don't match its type".to_string(),
                    );
                }
                let is_video = ct.starts_with("video/");
                let url = match state.services.media.put(&ct, buf).await {
                    Ok(url) => url,
                    Err(e) => return render_error(lang, Some(user.handle), e.to_string()),
                };
                media_parts.push(PendingMedia::stored(url, is_video));
            }
            "media_url" => {
                if media_parts.len() >= MAX_ATTACHMENTS {
                    return render_error(
                        lang,
                        Some(user.handle),
                        format!("a post may carry at most {MAX_ATTACHMENTS} attachments"),
                    );
                }
                let url = field.text().await.unwrap_or_default();
                if !url.trim().is_empty() {
                    media_parts.push(PendingMedia::url(url.trim().to_string()));
                }
            }
            // A caption belongs to the media part just before it.
            "caption" => {
                let caption = field.text().await.unwrap_or_default();
                if let Some(last) = media_parts.last_mut() {
                    last.caption = caption.trim().to_string();
                }
            }
            "demos" => demos_slug = field.text().await.unwrap_or_default(),
            "title" => title = field.text().await.unwrap_or_default(),
            "body" => body = field.text().await.unwrap_or_default(),
            "tags" => tags_raw = field.text().await.unwrap_or_default(),
            _ => continue,
        }
    }

    let title = title.trim();
    if title.is_empty() {
        return render_error(lang, Some(user.handle), "a title is required".to_string());
    }
    let demos = match state.services.demoi.by_slug(demos_slug.trim()).await {
        Ok(Some(d)) => d,
        Ok(None) => return render_error(lang, Some(user.handle), "choose a community".to_string()),
        Err(e) => return render_error(lang, Some(user.handle), e.to_string()),
    };

    // Resolve each part to a media item, preserving order. Uploads were already
    // validated and stored while streaming; here we only pair them with captions
    // and vet any dragged-in links.
    let mut media = Vec::with_capacity(media_parts.len());
    for part in media_parts {
        let item = match part.kind {
            PendingKind::Stored { url, is_video } => Media {
                url,
                caption: part.caption,
                is_video,
            },
            PendingKind::Url { url } => match classify_media_url(&url) {
                Some(is_video) => Media {
                    url,
                    caption: part.caption,
                    is_video,
                },
                None => {
                    return render_error(
                        lang,
                        Some(user.handle),
                        "that link isn't a supported image or video".to_string(),
                    )
                }
            },
        };
        media.push(item);
    }

    if body.trim().is_empty() && media.is_empty() {
        return render_error(
            lang,
            Some(user.handle),
            "add some text or attach media".to_string(),
        );
    }

    let tags = normalize_tags(&tags_raw);
    match state
        .services
        .create_post(user.id, demos.id, title, body.trim(), media, tags)
        .await
    {
        Ok(p) => Redirect::to(&format!("/post/{}", p.id.0)).into_response(),
        Err(e) => render_error(lang, Some(user.handle), e.to_string()),
    }
}
