# Services decomposition — state & remaining work

Breaking up the `app::Services` god object (was 25 ports + ~40 use-case methods,
one struct with every reason-to-change) into cohesive, separately-injectable
per-area services. Full SOLID is the target. This doc is the resume point.

## Done (committed on `master`, all green: `cargo build` + `cargo test -p app -p adapter-store-memory`)

1. **File split** (`962524c`) — the 2329-line `services.rs` became one struct file +
   per-area `impl Services` files (one-def-per-file).
2. **14 cohesive services extracted** — all use-case logic now lives in its own
   service struct owning ONLY the ports it needs; `Services` retains a thin
   delegator per method. Commits: `73aa479` Notification, `cfa3ce0`
   Blocking/Profile/Search/Metrics, `9d8a797` Account/Invite/SensitiveReview,
   `bf5e520` Membership/Founding/Moderation, `8cacb97` Governance/Content,
   `5fb9d80` Feed.
   - Files: `crates/app/src/services/<name>_service.rs`.
   - Cross-area helpers became public methods on their owning service; dependent
     services hold `Arc<PeerService>` (acyclic DAG). Cross-cuts: `verify_user_action`,
     `ensure_not_barred` → AccountService; `load_triplet` → MembershipService;
     `require_can_post`/`require_unsanctioned_member`/`file_or_merge_flag`/
     `total_voter_weight` → ModerationService; `recompute_popularity` → MetricsService.
3. **`ServiceSet` DI container** (`1214c61`) — `crates/app/src/services/service_set.rs`.
   Bundles one built instance of each service (`from_services(&Services)`). Holds no
   logic/ports — a wiring bundle, not a god object.
4. **Web layer on separate injection** (`27d0ca5`, `5fb9d80`) — `AppState` holds each
   of the 14 services as its own `Arc<XService>` field (built in `router.rs` via
   `ServiceSet::from_services`). ~40 handler call sites migrated from
   `state.services.M(...)` to the specific `state.<svc>.M(...)`.

### Key fact
`Services` is now **logic-free**: every method just delegates to a cohesive
service via a `pub(super) fn <area>_service(&self)` builder (Arc clones only, so no
`Services` struct field was added and no `Services { .. }` literal changed). The
god *object* is gone; what remains named `Services` is a delegating facade / DI
builder still threaded through the composition root, CLI, federation, and tests.

## Remaining (not started)

Strangler pattern kept everything green at each step; continue the same way
(build + test green, commit per slice).

- **Phase 4 tail — migrate remaining `Services` callers off it:**
  - `adapter-cli/src/dispatch.rs` (~16 `services.X()` sites) → take a `ServiceSet`.
  - `app` federation-write ports that wrap `Services`: `writes.rs` (LocalWrites →
    Governance+Content+Moderation services), `local_minter.rs` (→ AccountService),
    `local_authenticator.rs` (→ AccountService).
  - `democratos/src/`: `spawn_recommendation_refresher.rs`, `seed`, and the
    `main.rs` serve arm (`ensure_barred_account`, `is_invite_only`, etc.).
  - 16 integration-test files that build `Services { ..ports.. }` and call
    `svc.method()` (`adapter-store-memory/tests/*`, `adapter-federation/tests/*`).
- **Phase 5 — delete `Services`:** once no external caller remains, give `ServiceSet`
  a port-based constructor (move the 14 `<area>_service()` builder bodies into it),
  delete the `Services` struct + the 14 delegator area files (`accounts.rs`,
  `governance.rs`, …) + the `foo_service()` builders.
- **Phase 6 — DIP leaks (`adapter-web`):** `AppState` still exposes raw
  `Arc<dyn …Store>` via the kept `services` field; handlers reach stores directly
  (`demos_page.rs`, `post_page.rs`, `can_vote.rs`, `current_user.rs`,
  `search_page.rs`). Route these through a service; drop the raw stores / the
  `services` field from `AppState`.
- **Phase 7 — OCP (`DemosStore`):** replace the 8 single-field setters
  (`set_allows_nsfw`, `set_jury_sizing`, …) with `update(&Demos)` optimistic write,
  matching Trial/Proposal. Touches domain + 3 store adapters + callers.
- **Phase 8 — LSP:** `MemoryStore` panics on a poisoned mutex while `TextFileStore`
  recovers (`into_inner`); make them substitutable (both recover).

## Recommended next order
DIP/OCP/LSP (6→8) are the remaining *real* SOLID violations and are higher value
than deleting an already-logic-free `Services`. Do those first, then the Phase 4
tail + Phase 5 deletion as mechanical cleanup.

Design detail & port/peer sets: see the session scratchpad `services-split-design.md`.
