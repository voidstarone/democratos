//! Caption/URL heuristic NSFW scanner.

use async_trait::async_trait;

use app::{MediaVerdict, NsfwScanner, Result};
use domain::is_nsfw_text;

/// Caption/URL heuristic NSFW scanner. See crate docs.
#[derive(Default)]
pub struct HeuristicNsfwScanner;

#[async_trait]
impl NsfwScanner for HeuristicNsfwScanner {
    async fn scan_media(&self, url: &str, caption: &str, _kind: &str) -> Result<MediaVerdict> {
        // URL path segments often carry signal (e.g. ".../porn/..."); treat path
        // separators as token boundaries by scoring the caption and URL together.
        let text = format!(
            "{caption} {}",
            url.replace(['/', '.', '-', '_', '?', '&', '='], " ")
        );
        if is_nsfw_text(&text) {
            Ok(MediaVerdict::Nsfw)
        } else {
            Ok(MediaVerdict::Unknown)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn flags_explicit_caption_or_url() {
        let s = HeuristicNsfwScanner;
        assert_eq!(
            s.scan_media("https://x/img/cat.jpg", "explicit nude photo", "image")
                .await
                .unwrap(),
            MediaVerdict::Nsfw
        );
        assert_eq!(
            s.scan_media("https://x/porn/clip.mp4", "", "video")
                .await
                .unwrap(),
            MediaVerdict::Nsfw
        );
        // No signal → Unknown (the scanner can't see pixels), not a false Sfw.
        assert_eq!(
            s.scan_media("https://x/img/sunset.jpg", "a lovely sunset", "image")
                .await
                .unwrap(),
            MediaVerdict::Unknown
        );
    }
}
