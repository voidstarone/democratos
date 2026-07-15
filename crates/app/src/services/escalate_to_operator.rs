use domain::ReportTarget;

/// Alert the operator that a review upheld a CSAM classification. There is a legal
/// duty to preserve the material and report it to the NCMEC CyberTipline (18 U.S.C.
/// §2258A); the content is already taken down, but a human must act on the report.
/// Logged at ERROR so it cannot pass unnoticed. (Byte-level preservation to the
/// media quarantine is a follow-up — see docs/sensitive-content-review-plan.md.)
pub(super) fn escalate_to_operator(target: ReportTarget) {
    tracing::error!(
        ?target,
        "SENSITIVE REVIEW: content classified as CSAM and removed — PRESERVE the material \
         and file a NCMEC CyberTipline report (18 U.S.C. §2258A)"
    );
}
