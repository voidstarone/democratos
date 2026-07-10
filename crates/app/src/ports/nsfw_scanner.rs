//! Classifies image/video content for NSFW material.

use async_trait::async_trait;

use crate::ports::media_verdict::MediaVerdict;
use crate::Result;

/// Classifies image/video content for NSFW material — the part of detection that
/// can't be a pure function. A port so the lightweight default (a caption
/// heuristic, fine for a Raspberry Pi) can be swapped for a real model or an
/// external service with no change to the domain or the use-cases.
#[async_trait]
pub trait NsfwScanner: Send + Sync {
    /// Classify media by its URL, caption, and kind label (`"image"`/`"video"`).
    async fn scan_media(&self, url: &str, caption: &str, kind: &str) -> Result<MediaVerdict>;
}
