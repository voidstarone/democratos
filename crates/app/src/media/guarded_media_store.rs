//! The single ingest choke point: sanitize → scan → store, or block and preserve.

use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    MediaError, MediaQuarantine, MediaSafetyScanner, MediaSanitizer, MediaStore, Result,
    SafetyVerdict, ScanFailurePolicy, MAX_UPLOAD_BYTES,
};

/// What an uploader is told when their file is refused on safety grounds.
///
/// Deliberately uniform and uninformative: a message that distinguished "matched
/// the known-bad corpus" from "the scanner was down" would turn the upload
/// endpoint into an oracle someone could probe to learn whether a given file is
/// in the corpus. The operator gets the detail, in the logs and the incident
/// record; the client gets a refusal.
const REFUSAL: &str = "that file was refused";

/// A [`MediaStore`] decorator that runs every upload through the safety pipeline
/// before it reaches the real backend.
///
/// This is the *only* path bytes take into storage, which is the point: the
/// safety posture is assembled once, in the composition root, and a new caller
/// cannot bypass it by reaching for the inner store — it never sees one. The
/// stages, in order:
///
/// 1. **Sanitize** ([`MediaSanitizer`]) — prove the bytes are really the media
///    they claim to be, bound their decoded cost, and re-encode images so what is
///    persisted carries none of the original's metadata or trailing payload.
/// 2. **Scan** ([`MediaSafetyScanner`]) — match the *sanitized* bytes against the
///    operator's known-bad corpus.
/// 3. **Store**, or block and preserve in [`MediaQuarantine`].
///
/// The scan runs on the sanitized bytes rather than the originals so that what is
/// checked is byte-for-byte what would be served — re-encoding after scanning
/// would leave a gap where the stored object was never the thing that was
/// cleared.
///
/// A positive match is blocked and preserved unconditionally. Only scanner
/// *unavailability* consults [`ScanFailurePolicy`].
pub struct GuardedMediaStore {
    inner: Arc<dyn MediaStore>,
    sanitizer: Arc<dyn MediaSanitizer>,
    scanner: Arc<dyn MediaSafetyScanner>,
    quarantine: Arc<dyn MediaQuarantine>,
    policy: ScanFailurePolicy,
}

impl GuardedMediaStore {
    /// Wrap `inner` so every upload is sanitized and scanned before it is stored.
    pub fn new(
        inner: Arc<dyn MediaStore>,
        sanitizer: Arc<dyn MediaSanitizer>,
        scanner: Arc<dyn MediaSafetyScanner>,
        quarantine: Arc<dyn MediaQuarantine>,
        policy: ScanFailurePolicy,
    ) -> Self {
        Self {
            inner,
            sanitizer,
            scanner,
            quarantine,
            policy,
        }
    }

    /// Preserve refused bytes, returning the incident id. A failure to preserve
    /// is propagated, never swallowed: the pipeline must not fall through and
    /// serve media it has decided to refuse just because the hold failed.
    async fn preserve(
        &self,
        content_type: &str,
        bytes: &[u8],
        reason: &str,
    ) -> Result<String, MediaError> {
        self.quarantine.preserve(content_type, bytes, reason).await
    }
}

#[async_trait]
impl MediaStore for GuardedMediaStore {
    async fn put(&self, content_type: &str, bytes: Vec<u8>) -> Result<String, MediaError> {
        // Defence in depth: the delivery layer already caps each part while
        // streaming it, but this is the choke point every caller shares — the
        // CLI and federation ingest reach it without passing through that cap.
        if bytes.len() > MAX_UPLOAD_BYTES {
            return Err(MediaError::Rejected(format!(
                "that file exceeds the {} MB limit",
                MAX_UPLOAD_BYTES / (1024 * 1024)
            )));
        }

        // 1. Sanitize. Rejections here are the client's problem to fix (wrong
        //    type, undecodable, too many pixels) and carry their own message.
        let clean = self.sanitizer.sanitize(content_type, bytes).await?;

        // 2. Scan the bytes that would actually be served.
        match self.scanner.scan(&clean.content_type, &clean.bytes).await {
            Ok(SafetyVerdict::Clear) => {}

            // A positive match is blocked and preserved, always — no policy
            // value permits storing it. Preservation is a legal duty, not a
            // bin: see `MediaQuarantine` and docs/media-safety.md §4.
            Ok(SafetyVerdict::Match { source, reason }) => {
                let incident = self
                    .preserve(
                        &clean.content_type,
                        &clean.bytes,
                        &format!("known-bad match ({source}: {reason})"),
                    )
                    .await?;
                tracing::error!(
                    %incident,
                    %source,
                    %reason,
                    "MEDIA BLOCKED: upload matched the known-bad corpus and was preserved in \
                     quarantine — an operator must review it and file a NCMEC CyberTipline \
                     report (18 U.S.C. §2258A)"
                );
                return Err(MediaError::Rejected(REFUSAL.to_string()));
            }

            // The scanner could not decide. Per its contract this is never
            // downgraded to `Clear`; the node's policy decides what happens.
            Err(e) => match self.policy {
                ScanFailurePolicy::FailClosed => {
                    tracing::warn!(
                        error = %e,
                        "media scan unavailable — upload refused (policy: fail-closed)"
                    );
                    return Err(MediaError::Rejected(REFUSAL.to_string()));
                }
                ScanFailurePolicy::Quarantine => {
                    let incident = self
                        .preserve(&clean.content_type, &clean.bytes, "scanner unavailable")
                        .await?;
                    tracing::warn!(
                        %incident,
                        error = %e,
                        "media scan unavailable — upload refused and preserved for review \
                         (policy: quarantine)"
                    );
                    return Err(MediaError::Rejected(REFUSAL.to_string()));
                }
                ScanFailurePolicy::Allow => {
                    tracing::warn!(
                        error = %e,
                        "media scan unavailable — storing UNSCANNED media (policy: allow)"
                    );
                }
            },
        }

        // 3. Cleared (or explicitly allowed through): persist the sanitized bytes
        //    under the type the sanitizer settled on, never the client's.
        self.inner.put(&clean.content_type, clean.bytes).await
    }

    /// Reads are not guarded: everything readable was written through `put`, so
    /// it has already been sanitized and scanned. Re-checking on every read would
    /// pay the cost again for bytes we produced ourselves.
    async fn get(&self, key: &str) -> Result<Option<(String, Vec<u8>)>, MediaError> {
        self.inner.get(key).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SanitizedMedia;
    use std::sync::Mutex;

    /// Records what reached the backend.
    #[derive(Default)]
    struct SpyStore {
        stored: Mutex<Vec<(String, Vec<u8>)>>,
    }

    #[async_trait]
    impl MediaStore for SpyStore {
        async fn put(&self, content_type: &str, bytes: Vec<u8>) -> Result<String, MediaError> {
            self.stored
                .lock()
                .unwrap()
                .push((content_type.to_string(), bytes));
            Ok("/media/stored".to_string())
        }
        async fn get(&self, _key: &str) -> Result<Option<(String, Vec<u8>)>, MediaError> {
            Ok(None)
        }
    }

    /// Rewrites the bytes, standing in for a re-encode.
    struct FakeSanitizer;

    #[async_trait]
    impl MediaSanitizer for FakeSanitizer {
        async fn sanitize(
            &self,
            declared_content_type: &str,
            _bytes: Vec<u8>,
        ) -> Result<SanitizedMedia, MediaError> {
            if declared_content_type != "image/png" {
                return Err(MediaError::Rejected("unsupported upload type".to_string()));
            }
            Ok(SanitizedMedia::new("image/png", b"sanitized".to_vec()))
        }
    }

    enum ScannerBehaviour {
        Clear,
        Match,
        Unavailable,
    }

    struct FakeScanner(ScannerBehaviour);

    #[async_trait]
    impl MediaSafetyScanner for FakeScanner {
        async fn scan(&self, _ct: &str, _bytes: &[u8]) -> Result<SafetyVerdict, MediaError> {
            match self.0 {
                ScannerBehaviour::Clear => Ok(SafetyVerdict::Clear),
                ScannerBehaviour::Match => Ok(SafetyVerdict::matched("test-corpus", "sha256")),
                ScannerBehaviour::Unavailable => {
                    Err(MediaError::Store("scanner offline".to_string()))
                }
            }
        }
    }

    #[derive(Default)]
    struct SpyQuarantine {
        held: Mutex<Vec<(String, Vec<u8>, String)>>,
        fail: bool,
    }

    #[async_trait]
    impl MediaQuarantine for SpyQuarantine {
        async fn preserve(
            &self,
            content_type: &str,
            bytes: &[u8],
            reason: &str,
        ) -> Result<String, MediaError> {
            if self.fail {
                return Err(MediaError::Store("quarantine unwritable".to_string()));
            }
            self.held.lock().unwrap().push((
                content_type.to_string(),
                bytes.to_vec(),
                reason.to_string(),
            ));
            Ok("incident-1".to_string())
        }
    }

    fn make_guard(
        behaviour: ScannerBehaviour,
        policy: ScanFailurePolicy,
    ) -> (GuardedMediaStore, Arc<SpyStore>, Arc<SpyQuarantine>) {
        let store = Arc::new(SpyStore::default());
        let quarantine = Arc::new(SpyQuarantine::default());
        let guard = GuardedMediaStore::new(
            store.clone(),
            Arc::new(FakeSanitizer),
            Arc::new(FakeScanner(behaviour)),
            quarantine.clone(),
            policy,
        );
        (guard, store, quarantine)
    }

    #[tokio::test]
    async fn clear_media_is_stored_as_the_sanitizer_left_it() {
        let (guard, store, quarantine) =
            make_guard(ScannerBehaviour::Clear, ScanFailurePolicy::FailClosed);
        let url = guard.put("image/png", b"original".to_vec()).await.unwrap();

        assert_eq!(url, "/media/stored");
        // The *sanitized* bytes reach the backend, never the client's originals.
        let stored = store.stored.lock().unwrap();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].0, "image/png");
        assert_eq!(stored[0].1, b"sanitized");
        assert!(quarantine.held.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn sanitizer_rejection_never_reaches_the_backend() {
        let (guard, store, _q) = make_guard(ScannerBehaviour::Clear, ScanFailurePolicy::Allow);
        let err = guard.put("application/zip", b"junk".to_vec()).await.unwrap_err();

        assert!(matches!(err, MediaError::Rejected(_)));
        assert!(store.stored.lock().unwrap().is_empty());
    }

    /// A positive match is blocked and preserved under *every* policy — this is
    /// the invariant the whole pipeline exists to hold.
    #[tokio::test]
    async fn a_match_is_always_blocked_and_preserved_whatever_the_policy() {
        for policy in [
            ScanFailurePolicy::FailClosed,
            ScanFailurePolicy::Quarantine,
            ScanFailurePolicy::Allow,
        ] {
            let (guard, store, quarantine) = make_guard(ScannerBehaviour::Match, policy);
            let err = guard.put("image/png", b"bad".to_vec()).await.unwrap_err();

            assert!(matches!(err, MediaError::Rejected(_)), "policy {policy:?}");
            assert!(
                store.stored.lock().unwrap().is_empty(),
                "matched media must never be stored (policy {policy:?})"
            );
            let held = quarantine.held.lock().unwrap();
            assert_eq!(held.len(), 1, "policy {policy:?}");
            // Quarantine holds the sanitized bytes — what would have been served.
            assert_eq!(held[0].1, b"sanitized");
        }
    }

    #[tokio::test]
    async fn refusal_message_does_not_reveal_why() {
        let (guard, _s, _q) = make_guard(ScannerBehaviour::Match, ScanFailurePolicy::FailClosed);
        let matched = guard.put("image/png", b"bad".to_vec()).await.unwrap_err();
        let (guard, _s, _q) = make_guard(ScannerBehaviour::Unavailable, ScanFailurePolicy::FailClosed);
        let unavailable = guard.put("image/png", b"x".to_vec()).await.unwrap_err();

        // Identical text, so the endpoint can't be probed as a corpus oracle.
        assert_eq!(matched.to_string(), unavailable.to_string());
    }

    #[tokio::test]
    async fn fail_closed_refuses_when_the_scanner_is_unavailable() {
        let (guard, store, quarantine) =
            make_guard(ScannerBehaviour::Unavailable, ScanFailurePolicy::FailClosed);
        let err = guard.put("image/png", b"x".to_vec()).await.unwrap_err();

        assert!(matches!(err, MediaError::Rejected(_)));
        assert!(store.stored.lock().unwrap().is_empty());
        // fail-closed refuses but does not hold a copy.
        assert!(quarantine.held.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn quarantine_policy_refuses_and_holds_a_copy() {
        let (guard, store, quarantine) =
            make_guard(ScannerBehaviour::Unavailable, ScanFailurePolicy::Quarantine);
        let err = guard.put("image/png", b"x".to_vec()).await.unwrap_err();

        assert!(matches!(err, MediaError::Rejected(_)));
        assert!(store.stored.lock().unwrap().is_empty());
        assert_eq!(quarantine.held.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn allow_policy_stores_unscanned_media() {
        let (guard, store, _q) = make_guard(ScannerBehaviour::Unavailable, ScanFailurePolicy::Allow);
        let url = guard.put("image/png", b"x".to_vec()).await.unwrap();

        assert_eq!(url, "/media/stored");
        assert_eq!(store.stored.lock().unwrap().len(), 1);
    }

    /// If the bytes cannot be preserved, the upload fails — the pipeline must not
    /// fall through and serve media it has already decided to refuse.
    #[tokio::test]
    async fn failing_to_preserve_fails_the_upload_rather_than_storing_it() {
        let store = Arc::new(SpyStore::default());
        let quarantine = Arc::new(SpyQuarantine {
            held: Mutex::new(Vec::new()),
            fail: true,
        });
        let guard = GuardedMediaStore::new(
            store.clone(),
            Arc::new(FakeSanitizer),
            Arc::new(FakeScanner(ScannerBehaviour::Match)),
            quarantine,
            ScanFailurePolicy::Allow,
        );

        let err = guard.put("image/png", b"bad".to_vec()).await.unwrap_err();
        assert!(matches!(err, MediaError::Store(_)));
        assert!(store.stored.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn oversized_uploads_are_refused_before_sanitizing() {
        let (guard, store, _q) = make_guard(ScannerBehaviour::Clear, ScanFailurePolicy::Allow);
        let too_big = vec![0u8; MAX_UPLOAD_BYTES + 1];
        let err = guard.put("image/png", too_big).await.unwrap_err();

        assert!(matches!(err, MediaError::Rejected(_)));
        assert!(store.stored.lock().unwrap().is_empty());
    }
}
