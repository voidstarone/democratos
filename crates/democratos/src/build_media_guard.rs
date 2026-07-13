//! Wrap a media store in the sanitize + CSAM-scan + quarantine pipeline.

use std::sync::Arc;

use anyhow::Result;

use adapter_media_safety::{
    AllowAllSafetyScanner, DirQuarantine, HashListSafetyScanner, ImageReencodeSanitizer,
    KnownHashSet, PassthroughSanitizer,
};
use app::{
    GuardedMediaStore, MediaSafetyScanner, MediaSanitizer, MediaStore,
};

use crate::media_guard_config::MediaGuardConfig;
use crate::sanitizer_kind::SanitizerKind;

/// Wrap `inner` (the chosen media backend) in a [`GuardedMediaStore`] so every
/// upload is sanitized and safety-scanned before it is stored — the bytes a CDN
/// later serves are always a product of our own encoder and cleared by our
/// scanner. Called for every store arm; the guard's behaviour is entirely
/// config-driven, so this is the single place the safety posture is assembled.
pub(crate) fn build_media_guard(
    inner: Arc<dyn MediaStore>,
    cfg: &MediaGuardConfig,
) -> Result<Arc<dyn MediaStore>> {
    let sanitizer: Arc<dyn MediaSanitizer> = match cfg.sanitizer {
        SanitizerKind::Reencode => Arc::new(ImageReencodeSanitizer),
        SanitizerKind::Passthrough => {
            eprintln!(
                "⚠ media sanitizer is 'passthrough' — uploads are type-checked but NOT re-encoded, \
                 so stored bytes keep any embedded metadata or trailing payload. Prefer 'reencode' \
                 wherever the hardware allows."
            );
            Arc::new(PassthroughSanitizer)
        }
    };

    let scanner: Arc<dyn MediaSafetyScanner> = if cfg.csam_scan {
        let corpus = match &cfg.hash_file {
            Some(path) => KnownHashSet::load(path)
                .map_err(|e| anyhow::anyhow!("reading --csam-hash-file {path}: {e}"))?,
            None => KnownHashSet::empty(),
        };
        let source = cfg
            .hash_file
            .clone()
            .unwrap_or_else(|| "local-hash-list".to_string());
        let scanner = HashListSafetyScanner::new(corpus, source);
        if scanner.is_noop() {
            // The one misconfiguration that silently defeats the whole feature:
            // scanning "on" but with nothing to match against. Shout like the
            // age-verification stub does, so it can't pass unnoticed.
            eprintln!(
                "⚠ CSAM scanning is ENABLED but no --csam-hash-file (DEMOCRATOS_CSAM_HASH_FILE) is \
                 configured, so the known-bad corpus is EMPTY and every upload clears the scan. \
                 Supply a curated hash list (see docs/media-safety.md) before relying on this, or \
                 pass --csam-scan false to opt out explicitly."
            );
        }
        Arc::new(scanner)
    } else {
        // Off by default: without a lawful known-bad hash source (or an external
        // classifier) the scan can't actually detect anything, so we don't pretend
        // to. Malicious-media sanitization above still runs. State it plainly so the
        // posture is never in doubt.
        eprintln!(
            "ℹ CSAM scanning is OFF — uploaded media is sanitized (re-encoded, bounded) but NOT \
             scanned for known illegal content. Enable with --csam-scan and --csam-hash-file once \
             a lawful hash source or external classifier is available (see docs/media-safety.md)."
        );
        Arc::new(AllowAllSafetyScanner)
    };

    let quarantine = Arc::new(
        DirQuarantine::new(&cfg.quarantine_dir)
            .map_err(|e| anyhow::anyhow!("opening quarantine dir {}: {e}", cfg.quarantine_dir))?,
    );

    Ok(Arc::new(GuardedMediaStore::new(
        inner,
        sanitizer,
        scanner,
        quarantine,
        cfg.policy,
    )))
}
