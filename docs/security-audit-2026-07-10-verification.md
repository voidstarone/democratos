# Democratos Security Audit — Verification Pass (2026-07-10)

This is a **verification + fresh-hunt** audit run after the "fix everything" remediation
rounds of 2026-07-10. It (a) re-checks that every previously-claimed fix still holds in
the current tree, and (b) hunts for new/residual issues. Four independent auditors covered
the four trust domains; every headline finding below was re-verified against source.

**Scope reviewed:** `crates/adapter-web`, `crates/app`, `crates/domain`,
`crates/adapter-store-postgres` (+ migrations 0001–0010), `crates/federation`,
`crates/adapter-federation`, `crates/adapter-control-etcd`, `crates/adapter-media-*`,
`crates/democratos`, `docker-compose*.yml`, `deploy/`.

**Bottom line.** The cryptographic core is sound and *every* prior fix is intact. Residual
risk concentrates in two places: (1) one governance-replication scope gap that is the exact
class already fixed twice elsewhere (**HIGH-1, comments**), and (2) the control-plane write
trust model — the anti-takeover binding is only as strong as the *first-write-wins*
publication of community keys (**FED-1**). Nothing here is a remotely-exploitable
single-node RCE/SQLi/XSS; the open items are federation-trust and governance-integrity
issues plus a handful of hardening LOWs.

---

## Severity summary

| ID | Sev | Area | One-line |
|------|--------|------------|----------|
| HIGH-1 | ~~HIGH~~ **✅ FIXED** | governance/replication | `comments` had no resolvable community → `ScopeMismatch` → **comments never replicated** (liveness; reclassified from "forgery" by the harness). Fixed: `event_scope` resolves a comment via its parent post |
| FED-1 | ~~HIGH~~ **✅ FIXED** | federation control-plane | Community-key first-publish was unauthenticated → durable takeover. Fixed: `publish_community_key` is **origin-node-authenticated**. Residual by design: a genuinely origin-less community's bare lease stays permissively claimable (closing it would break failover/import) |
| FED-2 | MEDIUM | federation | `POST /federation/command` never checks *this* node owns the target community → insider/replay forces writes onto a replica |
| FED-3 | ~~MEDIUM~~ **✅ FIXED** | federation | `set_home_binding` stored a binding without verifying its signature → poison/DoS. Fixed: verified against the community key at write, and an unverifiable stored binding is treated as absent in `authorize`/`claim` |
| WEB-1 | MEDIUM | web | Raw `sqlx` error text is rendered to clients (`AppError::Store` → `render_error`) → schema/recon leak |
| GOV-2 | MEDIUM | governance | Weighted quorum *denominator* is summed from the `MAX_ROWS=10 000`-capped members list → quorum bypass in >10k-voter weighted communities |
| GOV-3 | MEDIUM | governance | `vote_comment` is unsigned yet feeds `contribution` → forged comment-votes clear the franchise / inflate `ByContribution` weight |
| GOV-4 | MEDIUM | governance | Enfranchisement slot check is TOCTOU → concurrent admissions all pass, bypassing the flood cap |
| WEB-2 | LOW→MED | web | `secure_cookies` is warn-only off-loopback (not fail-closed) → sniffable session cookie on plain-HTTP exposed bind |
| GOV-5 | LOW/MED | governance | Proposal tally mixes cast-time ballot weights against a close-time electorate → quorum-ratio manipulation |
| DEP-1 | LOW | deploy | `.env.example` ships `change-me…` values that satisfy the compose presence-only `:?` guards → boots with world-known MinIO creds / guessable cluster token |
| DEP-2 | LOW | deploy | Unpinned base images; `minio/minio` has **no tag** (floating `latest`) |
| WEB-3 | LOW | web | Open-redirect: `/\`-guard missing on the absolute-URL branch of `local_path_of` |
| WEB-4 | LOW | web | No server-side session revocation; 30-day signed cookie survives logout/password-change |
| WEB-5 | LOW | web | CSRF token derives entropy from std `RandomState`/SipHash, not `OsRng` |
| GOV-6 | LOW | governance | `FoundingStore::list` has no `LIMIT` (only no-arg read that escaped the `MAX_ROWS` cap) |
| GOV-7 | LOW | governance | `close_proposal` commits `applied=true` then applies effects in a *separate* txn → a crash drops a passed rule/ban |
| FED-4 | LOW/INFO | federation | Home binding hardcodes `epoch=1` and is only re-signed by the origin node → second failover to a later standby is refused (availability) |
| DEP-3 | LOW | deploy | `reddit.json` (synthetic PII + Argon2 hashes for `password123`) is not in `.gitignore` (it is in `.dockerignore`) |
| Various | INFO | all | dev-mode blast radius; rate-limiter collapses to one bucket behind the bundled proxy; cross-community feed exposure within the fleet; sanctioned voters left in quorum denominator (fail-safe direction) |

---

## Resolution — HIGH-1, FED-1, FED-3 closed (2026-07-10)

The three findings the Byzantine harness surfaced were fixed and verified (full
workspace suite green; the harness now reports **8/8 guardrails, 0 known-open**):

- **HIGH-1** — `crates/federation/src/ownership.rs`: `event_scope` now routes
  `comments` through their parent post (`EventScope::ViaParent { Post, post_id }`),
  exactly like ballots. Comments replicate again *and* stay owner-scoped; a forged
  comment for a foreign community is rejected (`NotOwner`). No migration needed. New
  unit tests: `a_comment_scopes_to_its_parent_post…`, `a_comment_authorizes_against_its_parent_posts_owner`.
- **FED-3** — `set_home_binding` (InMemory + etcd) now verifies the binding against
  the community key **before** storing it, and `authorize`/`claim` treat an
  unverifiable stored binding as *absent* (via the new `binding_is_authoritative`),
  so a poisoned binding can neither be installed through the API nor DoS the honest
  owner. New unit tests: `binding_is_authoritative_only_for_a_matching_community_key`,
  `set_home_binding_rejects_a_binding_that_does_not_verify`.
- **FED-1** — `publish_community_key` gains an `origin_proof_hex` argument: an
  Ed25519 signature by the community's **origin node** (`origin_node(demos)`) over
  `community_key_publish_challenge`. The etcd registry verifies it against the origin
  node's published key; `fed.rs::ensure_home_binding` signs it (it only ever runs on
  the origin node). A hostile peer can't mint a verifying community key/binding, so
  the durable, fencing-surviving takeover is closed. `InMemoryRegistry` (single-node)
  ignores the proof. **Residual, by design:** a community whose origin node never
  published a key stays permissively claimable (a bare lease, no verifying binding) —
  the deliberate "unbound/imported = unconstrained" contract; constraining it would
  break failover and cross-node import.

**Still genuinely open** from the broader audit (not the harness set): FED-2
(command endpoint local-ownership check), WEB-1/WEB-2, GOV-2..5, and the LOWs below.

## Verified — still fixed (no action)

Every fix from the prior remediation rounds was re-checked and **holds**:

- **Session auth.** HMAC-SHA256 cookie binds `uid`+`expiry`, constant-time `verify_slice`,
  server-side expiry re-check; a hand-written `uid=1` fails verification in *every* path
  (there is **no** raw-`uid` bypass, including in dev mode — dev endpoints only ever issue
  properly *signed* cookies). `app/src/session.rs`, `handlers.rs:420-431`, `dev.rs`.
- **Passwords.** Argon2id, per-password salt, enumeration-timing equalizer. `app/src/auth.rs`.
- **Rate limiting.** Per-**connection-peer-IP** fixed-window (XFF deliberately *not* trusted),
  strict on `/session`+`/register`, CSRF checked before Argon2. `rate_limit.rs`.
- **CSRF.** Double-submit token, HttpOnly+Lax cookie, constant-time compare, fail-closed.
- **Headers.** CSP `script-src 'self'; object-src 'none'`, HSTS, X-Frame-Options, nosniff;
  no inline `<script>`/`on*` handlers; Askama auto-escapes (no `| safe` anywhere) → **no XSS**.
- **Session secret.** Fails closed (`bail!`) on placeholder/`<16`-char secret on a non-loopback
  bind; compose ships no working secret. `main.rs:604-640`.
- **Media.** SHA-256 content-addressed keys, path-traversal-safe key guards, magic-number MIME
  sniff enforced, canonical Content-Type + nosniff on serve, no SVG, no server-side URL fetch
  (no SSRF), streaming upload (~one 25 MB file peak RAM), `MAX_ATTACHMENTS=8`, ~201 MB body cap.
- **Federation envelope.** Every `ChangeEvent` Ed25519-`verify_strict` over the **exact received
  bytes**; `authorize` derives the community from the *payload* (not the envelope), requires
  signer==current-owner at a non-stale epoch; epoch+seq+demos all inside the signed part.
- **Founder-signed home binding** is verified on **every** ownership honoring (not just at claim);
  `claim` eligibility gated in **both** InMemory and etcd registries; `set_standby` gated on
  `holder==self`; epoch is monotonic (never rolled back); claim is a bounded optimistic CAS.
- **Durable replay guard** (Postgres `command_nonces`, ±120 s freshness, 240 s retention — no gap).
- **Node auth / TLS.** Bearer token on feed/command/ingest, constant-time; etcd mTLS supported;
  plaintext off-box peer/etcd refused unless an explicit escape env is set.
- **`require_signatures` forced true** when federated; the `ALLOW_UNSIGNED_FEDERATION` hatch is gone.
- **SQL** fully parameterized; `apply_change` allowlists table names at compile time; `users`
  ingest strips `password_hash`/`email`; double-vote/jury race-free via `ON CONFLICT DO NOTHING`;
  `close_proposal` claims via rev-CAS before effects; `cast_vote` rejects post-`closes_at`;
  jury weights frozen per-juror; import is ID-preserving, FK-ordered, idempotent.

---

## New / residual findings (detail)

### HIGH-1 — `comments` replicate with `demos_id = NULL` → cross-community forgery
**Files:** `domain/src/content.rs:229-239` (`Comment` has no `demos_id`); every
`democratos_outbox` migration incl. `migrations/0008_rescope_comment_votes.sql:35-49`.

The capture trigger's `IF d IS NULL` block derives a community for `demoi`, `votes`,
`post_votes`, `comment_votes`, and `jury_ballots` — but has **no `comments` branch**, and the
`comments` row carries no `demos_id`. So every comment create/edit/remove event is emitted
with `demos_id = NULL`, which — per the project's own threat model (see the `0004` header) — is
authorized as a **global** event (authenticity only, no owner gate). Note the irony: a
comment's *votes* are correctly scoped via `comment→post→demos`, but the comment itself is not.

**Exploit:** any node holding a valid published key crafts a `comments` upsert (edit body / set
`removed`) for a comment in a community it does **not** own; peers authorize it globally and
apply it → cross-community content forgery and censorship. Secondary: per-community feed
subscribers filtering on `demos_id` never receive their own comments.

**Fix:** new migration `CREATE OR REPLACE` the `0008` body plus:
```sql
ELSIF TG_TABLE_NAME = 'comments' THEN
    SELECT demos_id INTO d FROM posts WHERE id = (r->>'post_id')::BIGINT;
```

**⚠ Correction from the Byzantine harness (empirical, 2026-07-10).** The
majority-compromised harness pushed a real comment event (payload as the outbox emits
it — no `demos_id`, since the `comments` table has no such column) to an honest node.
`authorize()` **rejected** it (`event_scope("comments")` → payload has no `demos_id`
→ `Indeterminate` → `ScopeMismatch`). So HIGH-1's real impact is a **liveness/
correctness bug — comments never replicate across nodes** — *not* cross-community
forgery (an attacker cannot get a forged comment applied; honest comments simply
don't propagate). Reclassify **HIGH-1 → MEDIUM (replication liveness)**. And the
one-line trigger fix above is **insufficient on its own**: it sets the *outbox*
`demos_id`, but `event_scope` reads the *payload*, which still has no `demos_id`, so
comments stay `ScopeMismatch`-rejected. The real fix must also make `event_scope`
resolve a comment's community **via its parent post** (like ballots — add a
`ParentKind::Post`-style derivation for `comments` in `ownership.rs`), or carry
`demos_id` in the comment payload. This is the harness earning its keep — it
overturned the code-reading conclusion.

### FED-1 — unauthenticated community-key first-publish → durable takeover
**Files:** `adapter-control-etcd/src/lib.rs:324-357` (`publish_community_key`), `:402-443`
(`publish_key`); consumed by `ownership.rs:422-438`.

Key publication is first-write-wins with no authentication of the writer. The **community-key**
case is worse than a DoS: whoever writes the community key first can then install a `HomeBinding`
**validly signed by that key** naming an attacker-controlled `home_node`. `authorize` fetches
that key, the binding verifies, `binding.authorizes(attacker)` is true → ownership that
*survives epoch fencing*. The entire anti-takeover guarantee rests on the community key's
first-write being trustworthy, which is not enforced (esp. for imported communities whose key
was never published, or a community pre-empted before its real home boots).

**Fix:** authenticated (per-node/founder-signed) key-publish; derive community-key authenticity
from the founder, not "first etcd writer"; at minimum bind the community keyspace to the home
node's mTLS identity via etcd RBAC and document it as load-bearing.

### FED-2 — command endpoint does not verify local ownership
**Files:** `adapter-federation/src/http.rs:203-236`, `command.rs:322-358`.

`command_handler` authenticates the forwarding node and checks the replay nonce, then calls
`execute(&services, &cmd)` with **no check that this node currently owns** the command's target
community. Because nonce logs are per-owner, a signed command accepted by owner A can be replayed
to non-owner B (whose log lacks the nonce), diverging B's replica. Bounded by `require_signatures`
and eventual convergence on pull, but it violates the single-authoritative-owner invariant.

**Fix:** resolve `registry.owner_of(demos_of(cmd))` and reject (409/421) unless it equals this
node before `execute`.

### FED-3 — `set_home_binding` stores without verifying the signature
**Files:** `adapter-control-etcd/src/lib.rs:371-390`, `ownership.rs:588-599`.

Neither impl verifies the binding against the community key before storing it. A party with etcd
write access can install a garbage/wrong-key binding; `authorize` then fails `binding.verify` →
`AuthError::Fed` → permanent skip → **every event for that community is dropped fleet-wide**
(censorship/DoS). Also: if the binding is present but the key is deleted, `authorize` returns the
*transient* `AuthError::Registry` → the puller stalls forever.

**Fix:** verify the binding signature in `set_home_binding`; in `authorize`, treat an
unverifiable stored binding as `NotBoundHome` (permanent skip that node) rather than
`Fed`/`Registry`, so a poisoned binding can't stall the whole community.

### WEB-1 — backend error text leaked to clients
**Files:** `app/src/error.rs:11-12` (`Store` → `"storage failure: {0}"`),
`adapter-store-postgres/src/lib.rs:59-61` (fills it with raw `sqlx` `Display`), handlers pass
`e.to_string()` into `render_error` (e.g. `handlers.rs:1710`, plus `vote`/`post_vote`/`add_comment`).

Any request that induces a DB error returns SQL fragments, column/constraint names, and schema
structure to the browser — a reconnaissance aid. The template-render error path already avoids
this; the domain-error path does not.

**Fix:** collapse `Store` (and other internal variants) to a generic client message while logging
detail server-side.

### GOV-2 — weighted quorum denominator uses the capped members list
**Files:** `services.rs:1410-1424` (`total_voter_weight` sums over `members()`),
`lib.rs:670-681` (`members` `LIMIT 10 000`).

For a community with >10 000 voters under non-`Equal` proposal weighting, the electorate
denominator only sums the first 10 000 members while `voter_count` is a true `COUNT(*)`. A
proposal (incl. `Constitutional`/`Ban`/`Recall`) can pass with roughly half the mandated
turnout — a quorum bypass scaling with how far the community exceeds the cap.

**Fix:** compute electorate weight with a DB-side aggregate (or a dedicated uncapped path); the
`MAX_ROWS` DoS backstop must never feed a governance denominator.

### GOV-3 — `vote_comment` unsigned but gates the franchise
**Files:** `services.rs:944-967` (no `sig`, no `verify_user_action`); absent from `GovernanceWrites`.

Comment votes feed `recompute_popularity → Membership.contribution`, which gates franchise
eligibility, `PostingPolicy::MinContribution`, and `ByContribution` weight. A hosting/relaying
node can forge comment votes for an account (no owner-verified signature blocks it) to push it
over the franchise bar or inflate its weight — the exact vector signatures close for post votes.

**Fix:** add a `sig` param, verify via `verify_user_action` over a canonical
`comment_vote_message`, and route through `GovernanceWrites`.

### GOV-4 — enfranchisement slot check is TOCTOU
**File:** `services.rs:429-458`.

Reads `voter_count`/`admitted_since`, computes `enfranchisement_slots`, then `upsert(Voter)`
with no lock across check and write. Concurrent admissions all observe the same count and all
pass, bypassing the Layer-2 flood cap (10 %/floor-of-5) — enough to jump phase boundaries or
swing quorum with a prepared cohort.

**Fix:** serialize admission per demos (`FOR UPDATE`/advisory lock) and re-check the slot count
inside the writing transaction, or enforce it with a conditional insert.

### WEB-2 — `secure_cookies` is warn-only off-loopback
**File:** `main.rs:697` warns but does not `bail!` or force `secure_cookies=true`; `Secure` is
emitted only when the flag is set (`handlers.rs:272-277`).

An operator serving plain HTTP on `0.0.0.0` ships a sniffable, rideable session cookie and the
process still starts. Contrast the session-secret path, which fails closed. **Fix:** default
`secure_cookies=true` off-loopback (or fail closed), matching the secret path.

### GOV-5 — proposal tally: cast-time weights vs close-time electorate
**Files:** `services.rs:564-569` (ballot weight frozen at cast) vs `:608-616` (electorate live at
close). Juries freeze *both* numerator and denominator; proposals freeze only the numerator. A
voter can vote at weight 16, then shed contribution to weight 1; at close the denominator counts
them as 1 while the aye numerator keeps 16 → inflated quorum ratio (non-`Equal` weighting only).
**Fix:** snapshot per-voter weight at proposal open, or measure both sides at the same instant.

### Lower severity (fix opportunistically)
- **DEP-1** — entrypoint/app should reject `change-me*`/low-entropy `DEMOCRATOS_CLUSTER_TOKEN`,
  `DB_PASSWORD`, `MINIO_ROOT_*` the way `main.rs` rejects `CHANGE_ME` session secrets; or ship
  `.env.example` with empty RHS so an unedited copy fails the `:?` guard.
- **DEP-2** — pin base images by digest; at minimum give `minio/minio` a concrete tag.
- **WEB-3** — apply the `/\` guard to the absolute-URL branch of `local_path_of` (`handlers.rs:399`).
- **WEB-4** — add a per-user session epoch to the signed payload; bump on logout-all / credential change.
- **WEB-5** — use `OsRng` (already a dep) for the CSRF token.
- **GOV-6** — add `LIMIT MAX_ROWS` to `FoundingStore::list` (`lib.rs:611`).
- **GOV-7** — apply the passed effect and the `applied` flag in one transaction (idempotent).
- **FED-4** — re-sign the binding on failover with a higher epoch (return-home path is unexercised).
- **DEP-3** — add `reddit.json` to `.gitignore` for symmetry with `democratos.json`.
- **INFO** — rate limiting collapses to one bucket behind the bundled Caddy proxy (XFF ignored):
  set a trusted-proxy real-IP source at the edge, or teach the limiter the proxy address.
  Sanctioned voters remain in the quorum denominator (fail-safe direction). Cross-community feed
  is readable by any token-bearing peer (accepted "replicas hold everything" property). If `--dev`
  is ever set on an exposed node, `/dev/unlock`→`/dev/switch` is full passwordless account takeover.

---

## Prioritized remediation order
1. **HIGH-1** — one-line migration; highest value, lowest effort; mirrors an already-solved pattern.
2. **FED-1 / FED-2 / FED-3** — control-plane write-authority hardening (the acknowledged trust anchor).
3. **GOV-2 / GOV-3 / GOV-4 / GOV-5** — governance-integrity (quorum + franchise) correctness.
4. **WEB-1 / WEB-2** — stop leaking DB errors; fail-closed on secure cookies.
5. Remaining LOWs as hardening.

See `deploy/test/security.sh`, `deploy/test/posture.sh`, and
`deploy/test/SECURITY_SCENARIOS.md` for executable regression/attack scenarios
covering each of these.

## Executable-scenario validation (2026-07-10)
- `posture.sh` (container-free) → **8/8 pass**: session-secret fail-closed on
  exposed binds, security headers, dev tooling 404 without `--dev`, and the
  unsigned-`uid` bypass rejected.
- `security.sh` against a live 2-node cluster → **9 guardrails pass, 0 fail, 1
  known-open**. The known-open is **HIGH-1**, confirmed live against the running
  cluster's `democratos_outbox` trigger (the `comments` branch is genuinely
  absent). This means HIGH-1 is not merely a code-reading inference — it is
  reproduced against real Postgres.
### Byzantine (majority-compromised) harness — `deploy/byzantine/`
A fully dockerized (plain `docker` CLI, no compose plugin; colima/Linux portable)
federation is stood up and attacked from a **majority-compromised** position by a
`redteam` tool that links the real `federation` crate (genuine Ed25519 + control-plane
writes). Ran at **2 honest / 3 compromised (5 nodes)** and **2 honest / 5 compromised
(7 nodes)** — identical results both times:
- **5/5 guardrails hold**: a compromised majority + cluster token + etcd write access
  **cannot** seize an honest founder-bound community (control-plane claim refused),
  cannot get a forged event applied (external adversary *and* a captured real cluster
  member both → `NotOwner`, 0 applied), and the honest node rejects a rogue peer's
  forged feed. Epoch fencing holds against repeated claims. This confirms the thesis:
  **authority is the founder key, not node-count** — the majority buys the attacker
  nothing against a bound community.
- **FED-1** reproduced (xfail): an *unbound* community is seized via unauthenticated
  first-write key + self-signed binding + claim.
- **FED-3** reproduced (xfail): the control plane stores an *unverified* attacker home
  binding → poisons that community's authorization (DoS).
- **HIGH-1** empirically **reclassified** (see the correction under HIGH-1 above):
  liveness bug, not forgery.

- Notable byproduct: the pre-existing `loadgen`/`functional.sh`/`stress.sh` load
  harness authenticates with a bare `uid=N` cookie, which the signed-cookie
  hardening now rejects (401), and votes by keyless users are refused by the
  `require_signatures` federation enforcement. Those load scripts predate the
  hardening and would need the `sign_in`-style dev-switch flow (and key enrolment)
  to drive authenticated writes again.
