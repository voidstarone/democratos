# Plan — Sensitive-Content Flagging & Extra-Demos Review

> **Status:** core **implemented** 2026-07-11 (single-node; memory + file +
> postgres stores). Written 2026-07-11.
> **Why this exists:** automated CSAM/illegal-content detection is disabled (no
> lawful hash source available — see `docs/media-safety.md` §3). So the platform
> must rely on **human flagging + reporting**, reviewed by opted-in accounts on a
> **platform-wide** path that sits *outside* per-demos governance.

## 0. Implemented design (supersedes the plan below where they differ)

The shipped model is a **quorum-based reviewer jury**:

- **Reviewer opt-in.** `User.is_sensitive_reviewer` (default off), toggled on the
  `/preferences` page. A plain platform account attribute — not a demos tier.
- **Flagging.** Any signed-in user flags a post via `POST /post/:id/flag` (button
  on the post page). The content is set `pending_review` and **hidden from all
  feeds, search, and the post page** for everyone except reviewers; a
  `SensitiveCase` opens (or the flag merges into the open one). Comments are
  flaggable through the service/store layer; the per-comment UI button is deferred.
- **Review console** `/review` (reviewer-gated): each open case reveals its content
  behind a click-through and offers a **classification** — CSAM, death/gore,
  self-harm, regular porn, spam, other, or not-sensitive.
- **Quorum + majority.** A case needs **≥ 5 distinct reviewers** (`REVIEW_QUORUM`)
  before it resolves; the **plurality tag wins**, ties broken toward the more
  severe/removing tag (`domain::tally_tags`). One vote per reviewer (a re-vote
  corrects it).
- **Category-driven disposition** (`domain::outcome_for`): CSAM → remove + operator
  escalation (logged at ERROR, per 18 U.S.C. §2258A) ; gore/self-harm/spam/other →
  remove; **regular porn → keep up but NSFW age-gate** (reuses the existing NSFW
  path); not-sensitive → restore.
- **Nav badge.** `GET /review/summary` returns `{reviewer, open}`; a small script in
  `base.html` shows a "Review N" badge on every page for reviewers with a non-empty
  queue.

Code map: `domain/src/sensitive/` (tags, case, quorum tally, outcome) +
`Post/Comment.pending_review` + `User.is_sensitive_reviewer`;
`app` `SensitiveCaseStore` port + `Services::{flag_sensitive, cast_review_vote,
list_review_queue, open_case_count, set_sensitive_reviewer}`; store impls in all
three adapters (migration `0011_sensitive_cases.sql`); web handlers
`flag_post`/`review_page`/`cast_review`/`review_summary` + `review.html`. Tested by
`adapter-store-memory/tests/review_flow.rs`.

**Deferred:** per-comment flag button; byte-level preservation of CSAM matches into
the media quarantine (currently remove + ERROR-log escalation only); federation
propagation of removals; operator dashboard; the "reveal only to the operator, never
the reviewer pool" refinement for suspected CSAM (§8); abuse-weighting of flags.

---

## 1. Core idea (one paragraph)

Any user can **flag** a post/comment/media as *sensitive/illegal*. A flag
**immediately hides** the content pending review (illegal material must come down
fast — it is *not* left up for a community jury window). The flag does **not** go to
the demos jury; it goes to a **platform review queue** handled by accounts that have
turned on an account-level **"review sensitive content"** toggle. That toggle is
**default OFF** and is **not** a demos tier, not elected, not per-community — it is a
plain account attribute. Reviewers triage: clear (restore), confirm-remove
(take down platform-wide + preserve to quarantine), or escalate to the operator (who
handles the NCMEC CyberTipline report).

## 2. Principles & non-goals

- **Outside self-governance, by design.** Illegal content is not a matter of
  community opinion, so this path bypasses `select_jury` / demos voting entirely.
  A demos cannot vote to keep CSAM up.
- **Default-off, opt-in visibility.** No account sees flagged-sensitive content
  unless it has explicitly opted in. Normal feeds never show it.
- **Fast hide, slow restore.** Flagging hides immediately; only a reviewer (or the
  operator) restores. Bias toward removal on suspected-illegal.
- **Preserve, never silently delete.** A confirmed-illegal takedown routes the bytes
  to the existing `MediaQuarantine` (18 U.S.C. §2258A / NCMEC) and alerts the
  operator. Reuses the quarantine built for the CSAM-scan path.
- **Minimize exposure.** As few humans as possible should see suspected CSAM. The
  self-serve reviewer toggle is a harm-reduction *triage* tool; confirmed CSAM
  escalates to the operator, not spread further. (See §8 — this is the main tension.)
- **Non-goals:** not a reputation system, not a demos moderator role, not tied to
  voter status or franchise.

## 3. Data-model changes

### 3.1 Account toggle (the one the user asked for)
`domain::User` gains a field, same backward-compatible pattern as the others:
```rust
/// Whether this account has opted in to reviewing sensitive/flagged content on
/// the platform review queue (and therefore to seeing it, behind a click-through).
/// Platform-wide, NOT a demos tier; deliberately outside community governance.
/// Default false — no account reviews or sees sensitive content unless it opts in.
#[serde(default)]
pub is_sensitive_reviewer: bool,
```
- Self-serve: a user sets it in account settings. Default off.
- Consider gating the *enable* action behind age-verification acknowledgment
  (see §8 open questions).

### 3.2 Content state
Posts and comments gain a sensitivity state (an enum, not a bare bool, so the
lifecycle is explicit). Suggested `domain::SensitiveState`:
```
None                 -> normal
FlaggedPendingReview -> hidden from everyone except reviewers; in the queue
ConfirmedRemoved     -> taken down platform-wide; bytes preserved
Cleared              -> a reviewer dismissed the flag; restored, immune to re-flag churn
```
Store as `#[serde(default)]` (older rows read back as `None`). Applies to
`Post` and `Comment` (media rides on its post).

### 3.3 The review case (the extra-demos lane)
The existing `Report` is **demos-scoped** (`Report.demos_id: DemosId`, juries from
demos voters) — it cannot represent a platform-wide case. So add a **separate**
type rather than overloading it:
```
domain::SensitiveCase {
    id, target: ReportTarget, reason: SensitiveReason,
    reporter: Option<UserId>, note, created_at,
    status: Open | Removed | Cleared,
    reviewed_by: Option<UserId>, reviewed_at: Option<Timestamp>,
    rev: u64,   // optimistic concurrency, like Report/Proposal
}
```
`SensitiveReason`: `Illegal` (suspected CSAM/illegal), `SelfHarm`, `Other` — start
minimal (`Illegal` + `Other`). Keep it deliberately distinct from
`ReportReason::Nsfw`, which stays the per-demos NSFW/jury path.
- New port `SensitiveCaseStore` (list open cases, get, create/merge by target,
  set status with rev-check) — mirrors `ReportStore`'s shape.

## 4. Ports / services / adapters to add

- **Domain:** `SensitiveState`, `SensitiveReason`, `SensitiveCase` (+ ids). Pure.
- **App port:** `SensitiveCaseStore` (in `app/src/ports/`). Implemented by the
  memory, textfile, and postgres store adapters (same as `ReportStore`).
- **App services (`Services`) methods:**
  - `flag_sensitive(reporter, target, reason, note)` — anyone; rate-limited;
    sets content `FlaggedPendingReview`, creates/merges a `SensitiveCase`.
  - `list_sensitive_queue(reviewer)` — reviewer-only; returns open cases.
  - `review_sensitive(reviewer, case_id, decision)` — reviewer-only; `decision ∈
    {Remove, Clear}`; `Remove` → content `ConfirmedRemoved`, media bytes →
    `MediaQuarantine::preserve(...)`, operator alert; `Clear` → restore.
  - `set_sensitive_reviewer(user, on)` — toggles the account flag.
  - All reviewer-gated methods check `user.is_sensitive_reviewer` (a new
    `require_sensitive_reviewer` guard + `SensitiveReviewError`).
- **Reuse:** `MediaQuarantine` (already built) for preservation on confirmed-remove.

## 5. Visibility rules

- **Feeds / post pages / search:** exclude anything not in `SensitiveState::None`
  (or `Cleared`) for normal users — filter at the query/service layer, not the
  template, so it can't leak.
- **Reviewers (toggle on):** see `FlaggedPendingReview` items **blurred behind a
  click-through** ("This content was flagged as sensitive. Reveal for review?"),
  plus the review queue page.
- **Author of flagged content:** sees a "hidden pending review" placeholder, not the
  content silently vanishing (transparency), but cannot un-flag it themselves.
- **`ConfirmedRemoved`:** gone for everyone; only the operator/quarantine retains it.

## 6. End-to-end flows

**Flag → hide:** user hits "Flag as sensitive" on a post/comment → `flag_sensitive`
→ content → `FlaggedPendingReview`, disappears from all normal surfaces, a
`SensitiveCase` opens (or the flag merges into an open one).

**Review → remove:** reviewer opens queue → reveals item behind click-through →
`review_sensitive(Remove)` → content `ConfirmedRemoved`; if it carried uploaded
media, the stored bytes are moved to quarantine and an operator alert is logged (for
the NCMEC report). Case → `Removed`.

**Review → clear (false flag):** `review_sensitive(Clear)` → content restored,
state `Cleared`; case → `Cleared`. Cleared content resists immediate re-flagging
(anti-harassment; require a fresh distinct reporter or an operator to re-open).

## 7. UI surfaces

- **Account settings:** a single toggle "Review sensitive content" (default off) with
  a plain-language explanation of what opting in means (you will see flagged content;
  it may be disturbing; confirmed-illegal is escalated to the operator).
- **Content actions:** a "Flag as sensitive/illegal" item alongside the existing
  report action (kept separate from the demos `report` → jury flow).
- **Review queue page:** visible only when the toggle is on; lists open cases with
  click-through reveal and Remove / Clear buttons.
- **Placeholders:** blurred "flagged — reveal for review" (reviewers) and "hidden
  pending review" (authors/others).

## 8. Safeguards & the central tension

**The hard problem:** letting *self-selected* users view flagged content that may be
CSAM is itself a legal/ethical hazard — viewing/distributing CSAM is exactly what the
law forbids, and a bad actor could enable the toggle specifically to seek it out.
Mitigations to build in:

- **Escalation, not exposure, for confirmed-illegal.** The reviewer's job is *triage
  and rapid hiding*, not adjudication of CSAM. A "this looks like CSAM" action should
  **immediately remove + escalate to the operator** and stop showing it to other
  reviewers — it must not fan out across the reviewer pool.
- **Age-gate + explicit acknowledgment** to enable the toggle.
- **Audit every reveal and every reviewer action** (who, when, which case) — a
  deterrent and an evidence trail.
- **Rate-limit flagging** and weight it, so "sensitive" can't be weaponized to censor
  (a single flag hides pending review, but repeated false flags from one account get
  throttled / can auto-clear).
- **Cap concurrent reviewers per item** to minimize how many people see any one item.
- **Operator kill-switch / dashboard** outside the user path entirely.

## 9. Open questions (decide before building)

1. **Who may enable the toggle?** Truly anyone (as stated), or gated by
   age-verification / account age / an operator grant? Stated design = anyone,
   default off — confirm given §8.
2. **Split "see" from "review"?** One toggle (opt-in = see + review) vs. two
   (a viewer preference and a separate reviewer role). Stated design = one toggle.
3. **Reveal policy for suspected-CSAM specifically** — should reviewers ever see it,
   or should an `Illegal`-reason flag auto-remove + operator-only, never shown to the
   reviewer pool? (Recommended: the latter, per §8.)
4. **Does flagging need a reason at flag time,** or is everything "sensitive" until a
   reviewer classifies it?
5. **Federation:** is a takedown platform-local or does it propagate to peer nodes?
   (`ConfirmedRemoved` likely must federate; the queue itself is per-node.)

## 10. Phasing

- **MVP:** `User.is_sensitive_reviewer` toggle + settings UI; flag action; content
  `FlaggedPendingReview` hide + feed/search exclusion; a memory/textfile
  `SensitiveCaseStore`; queue page; Remove/Clear with quarantine-on-remove;
  audit log. Single-node.
- **Later:** postgres store + federation propagation of removals; abuse-weighting on
  flags; operator dashboard; the `Illegal`-auto-escalate refinement from §8;
  age-gate on the toggle.

## 11. Touch points in the current codebase

- `crates/domain/src/user/user.rs` — add `is_sensitive_reviewer` (mirror
  `is_franchise_barred`'s `#[serde(default)]` pattern).
- `crates/domain/src/**` — new `SensitiveState`, `SensitiveReason`, `SensitiveCase`
  (one-def-per-file).
- `crates/app/src/ports/` — `SensitiveCaseStore`; `crates/app/src/error/` —
  `SensitiveReviewError` (per-use-case error, no catch-all).
- `crates/app/src/services/services.rs` — the methods in §4; reuse
  `MediaQuarantine`; a `require_sensitive_reviewer` guard.
- `crates/adapter-store-*` — implement `SensitiveCaseStore` + content-state columns.
- `crates/adapter-web/` — settings toggle handler, flag handler, queue page +
  reveal, and **feed/search filters** excluding non-`None`/`Cleared` content.
- Keep the existing `ReportReason::Nsfw` → demos-jury path untouched; this is a
  parallel lane.
