# Democratos — Security Testing Scenarios

Adversarial test scenarios that back the audit in
`docs/security-audit-2026-07-10-verification.md`. Two executable harnesses plus a
set of manual/design scenarios for the findings that need crafted federation
events (beyond what `curl` can drive).

## How to run

```bash
# Container-free: fail-closed startup + default HTTP posture. Just needs the binary.
cargo build -p democratos
deploy/test/posture.sh

# Against a running federated dev cluster (reuses the shared test containers):
cargo build -p democratos -p loadgen
deploy/test/up.sh          # NODES>=2 for the forwarding scenario
deploy/test/security.sh
deploy/test/down.sh
```

## Validation status (2026-07-10)

Both harnesses were executed against the real stack. `posture.sh` → **18/18 pass**
(fail-closed startup guards, security + defence-in-depth headers, dev-off, no
raw-`uid` bypass, method tampering, open-redirect guard, reflected-XSS escaping,
headers on error responses, path-param hardening, a sensitive-action auth gate,
and the private dev switcher's secret gate + puppet-only allowlist + fail-closed
exposure guard). `security.sh` runs against a live 2-node cluster (shared test
pg/etcd/minio): all guardrails below (S1–S17) are defences that must hold; there
are **no known-open** probes
in the executable suite. HIGH-1 (comment scope) is now **CLOSED** — the fix lives
in `event_scope` (routes `comments` via the parent post), so S10 is a source-level
regression guard rather than the old `democratos_outbox`-trigger probe, and the
crafted cross-community forgery is exercised by `deploy/byzantine` + the
federation unit tests. Re-run both after any change touching auth, headers,
redirects, or federation scoping.

## Result classes

- **GUARDRAIL** (`ok`/`bad`) — a defence that must hold. A failure fails the suite;
  treat it as a security regression.
- **KNOWN-OPEN** (`xfail`) — an audit finding that is *not yet fixed*. The attack
  currently succeeds; the probe reports `⚠` and does **not** fail the suite. When
  the fix lands, the probe flips to `ok` — then delete the `xfail`.

## Authenticating in tests

The signed-cookie hardening means a bare `uid=N` cookie no longer authenticates —
that rejection is Scenario 1 itself. To act *as* a user, `security.sh` mints a real
signed cookie the way a browser does, via the dev switcher
(`GET /dev/unlock` → `POST /dev/switch id=N`), which only the `--dev` cluster
exposes. (The old `loadgen`/`functional.sh` raw-`uid=N` login predates this
hardening and would now 401 — `security.sh`'s `sign_in`/`sign_in_new` helpers are
the correct pattern to reuse.)

---

## Executable scenario matrix

| # | Scenario | Class | Attacker action → expected defence | Harness | Audit ref |
|---|----------|-------|-----------------------------------|---------|-----------|
| P1 | Placeholder session secret, exposed bind | GUARDRAIL | `serve --addr 0.0.0.0` + `CHANGE_ME…` secret → process **fails closed** before binding | posture.sh | WEB-2 / main.rs:604 |
| P2 | Short session secret, exposed bind | GUARDRAIL | `<16`-char secret on `0.0.0.0` → fails closed | posture.sh | main.rs:613 |
| P3 | Placeholder secret, loopback | GUARDRAIL | same secret on `127.0.0.1` → warns, keeps serving (dev-frictionless) | posture.sh | main.rs:615 |
| P4 | Security headers | GUARDRAIL | `GET /` carries CSP(`script-src 'self'`)/HSTS/X-Frame-Options/nosniff | posture.sh + S8 | lib.rs:48 |
| P5 | Dev tooling off by default | GUARDRAIL | no `--dev` → `/dev/unlock`, `/dev/accounts` are `404` | posture.sh | dev.rs |
| P6/S1 | Forged unsigned cookie | GUARDRAIL | `Cookie: uid=1` (no HMAC) → `401`, nothing recorded | posture.sh + security.sh | handlers.rs:420 |
| P7 | Defence-in-depth headers | GUARDRAIL | `referrer-policy` + CSP `object-src/frame-ancestors/base-uri/form-action` | posture.sh | lib.rs:48 |
| P8 | Method tampering (posture) | GUARDRAIL | `GET /session`, `GET /logout` → `405` (POST-only) | posture.sh | lib.rs router |
| P9/S15 | Open-redirect guard | GUARDRAIL | `POST /lang` off-site `Referer` reduced to same-origin path; `//host` → `/` | posture.sh + security.sh | handlers.rs:394 |
| S2 | Tampered signed cookie | GUARDRAIL | flip a byte of a valid tag → `401` | security.sh | session.rs |
| S3 | Login CSRF | GUARDRAIL | `POST /session` with no `csrf` cookie/field → refused pre-Argon2 | security.sh | handlers.rs:976 |
| S4 | Non-member vote (IDOR) | GUARDRAIL | sign in as an outsider, vote by proposal id → refused, tally unchanged | security.sh | services.rs:513 |
| S5 | require_signatures enforced | GUARDRAIL | keyless user's vote refused ("requires a signing key") in a federated node — closes keyless-vote forgery | security.sh | H5 / services.rs |
| S6 | Change-feed node auth | GUARDRAIL | `GET /federation/changes` → no-token=`401`, wrong=`401`, correct=`200` | security.sh | http.rs:57 |
| S7 | Command/ingest reject unauth writes | GUARDRAIL | `POST /federation/command`,`/ingest` without a token never succeed (4xx) | security.sh | http.rs:197 |
| S9 | Auth rate limiting | GUARDRAIL | burst `POST /session` → starts returning `429` (Argon2-DoS/brute) | security.sh | rate_limit.rs |
| S10 | Comment scope (HIGH-1 regression) | GUARDRAIL | `event_scope` routes `comments` via parent post (`ViaParent{Post}`); revert trips the suite | security.sh | ownership.rs:302 |
| S11 | Privileged-action IDOR | GUARDRAIL | outsider/anon `POST /p/:id/close` → refused ("only a voter"/"sign in") | security.sh | handlers.rs close_proposal |
| S12 | Session-cookie hardening | GUARDRAIL | server-minted `uid` cookie is `HttpOnly` + `SameSite=Lax` + bounded `Max-Age` | security.sh | handlers.rs:263 |
| S13 | Feed input clamping | GUARDRAIL | hostile `since=-1`/`limit=999999` clamped, endpoint stays `200` | security.sh | http.rs:78 |
| S14 | Method tampering (cluster) | GUARDRAIL | `GET /p/:id/vote`, `GET /session` → `405` (POST-only) | security.sh | lib.rs router |
| P10/S17 | Reflected XSS | GUARDRAIL | `/search?q=<script>…` echoed into text + `value="…"` is HTML-escaped | posture.sh + security.sh | search.html / askama |
| P11 | Headers on error responses | GUARDRAIL | a `404` still carries framing + CSP (middleware wraps every response) | posture.sh | lib.rs:224 |
| P12 | Path-param hardening | GUARDRAIL | non-numeric/negative/overflow `/post/:id` → clean 4xx, never a 5xx | posture.sh | handlers.rs `Path<u64>` |
| P13 | Sensitive-action auth gate | GUARDRAIL | unauth `POST /account/key` → refused, no signing key bound | posture.sh | handlers.rs enroll_key |
| PD1 | Exposed dev switcher fails closed | GUARDRAIL | `--dev` on a non-loopback bind without `--dev-unlock-secret` → refuses to boot | posture.sh | main.rs serve |
| PD2 | Dev unlock demands the secret | GUARDRAIL | `/dev/unlock` no key=`404`, wrong key=`404`, correct key issues the cookie | posture.sh | dev.rs unlock |
| PD3 | Switcher is puppet-only | GUARDRAIL | `/dev/accounts` lists only barred puppets; switch to a puppet=`204`, to any other id=`404` | posture.sh | dev.rs switch/accounts |
| S16 | CSRF value-checked | GUARDRAIL | mismatched `csrf_token` refused (constant-time), correct token passes to auth | security.sh | handlers.rs csrf_valid |

---

## Manual / design scenarios (crafted federation events or DB state)

These exercise findings that require a malicious *node* (a valid published key + a
hand-crafted `ChangeEvent`/command) or specific DB state — beyond `curl`. Reproduce
them with a small Rust integration test in `crates/adapter-federation/tests/` or a
throwaway node, using the existing `federation`/`adapter-federation` test scaffolding.

### HIGH-1 — cross-community comment forgery (CLOSED; exploit for regression coverage)
Fixed: `event_scope("comments")` resolves a comment's community from its parent
post (`ViaParent{Post}`), so a comment carries the owner's scope at authorisation
time even though its outbox row's `demos_id` is NULL. S10 guards the fix at that
site; the end-to-end forgery below is exercised by `deploy/byzantine` and the
`crates/federation` ownership tests. Kept here as the reproduction:
1. Bring up nodes A (owns demos X) and B (owns demos Y), peered.
2. On A, create a post in X and a comment on it. The outbox row still has
   `demos_id` NULL (`SELECT entity, demos_id FROM outbox WHERE entity='comments';`)
   — now benign, because scope is derived from the parent post, not the row.
3. From B, craft a signed `comments` upsert (edit `body` / set `removed=true`) for
   A's comment id and serve it on B's feed / push it. `event_scope` resolves X via
   the parent post; `owner_of(X) != B` → rejected as **`NotOwner`** (pre-fix this
   authorised globally). Regression test mirrors `adapter-federation/tests/ingest.rs`.

### FED-1 — community-key first-publish takeover
1. For a demos whose community key was never published (e.g. an *imported*
   community, or one whose home node hasn't booted), have the attacker call
   `publish_community_key` with a key it controls, then `set_home_binding` with a
   `HomeBinding` it signs naming an attacker node as `home_node`.
2. Attacker `claim`s the lease. `authorize` fetches the attacker key, the binding
   verifies, `binding.authorizes(attacker)` is true → **ownership that survives
   epoch fencing.** **Expected after fix:** key-publish is authenticated (per-node/
   founder-signed) and an unauthorised community key is refused.

### FED-2 — forced write onto a non-owner replica
1. Capture (or, as an insider, mint) a valid signed `Command` for a vote in demos
   X owned by node A.
2. Deliver it to node B (a replica, not the owner) with a valid cluster token. B's
   per-owner nonce log doesn't contain the nonce, so replay protection doesn't fire;
   B runs `execute` against its own replica → divergence. **Expected after fix:** B
   rejects with 409/421 because `owner_of(X) != self`.

### FED-3 — poisoned binding → community DoS
1. With etcd write access, write a garbage `community/<X>/binding` value.
2. `authorize` fails `binding.verify` → `AuthError::Fed` → permanent skip → **every
   event for X is dropped fleet-wide.** **Expected after fix:** `set_home_binding`
   verifies the signature before storing; a bad stored binding degrades to
   `NotBoundHome` (skip that node) rather than stalling the community.

### GOV-2 — weighted-quorum bypass past the row cap
1. Seed a community with >10 000 voters and a non-`Equal` proposal weighting.
2. Open a proposal; the electorate denominator (`total_voter_weight` over the
   `LIMIT 10 000` `members()` list) undercounts while `voter_count` is a true
   `COUNT(*)`. A proposal passes with ~half the mandated turnout. **Expected after
   fix:** denominator computed with a DB-side `SUM`, independent of `MAX_ROWS`.

### GOV-3 — forged comment votes clear the franchise
1. Account U has an enrolled key. A relaying node forges `comment_votes` for U
   (no `sig`, no `verify_user_action`).
2. `recompute_popularity` lifts U's `contribution` over `min_contribution` /
   inflates `ByContribution` weight. **Expected after fix:** `vote_comment` carries
   and verifies a signature and routes through `GovernanceWrites`.

### GOV-4 — enfranchisement flood via TOCTOU
1. Prepare a cohort of already-eligible accounts (aged, member, contribution ≥ bar).
2. Fire N concurrent `POST /d/<slug>/enfranchise`. All read the same `admitted`
   count, all pass the slot check, all become voters in one window — bypassing the
   10 %/floor-of-5 cap. **Expected after fix:** admission serialised per demos
   (`FOR UPDATE`/advisory lock) with an in-txn re-check.

### GOV-5 — cast-time-weight vs close-time-electorate skew
1. In a non-`Equal` community, a voter pumps contribution to a high weight, votes
   aye (ballot freezes the high weight), then sheds contribution.
2. At close, the electorate denominator recomputes live (counts them low) while the
   aye numerator keeps the high weight → inflated quorum ratio. **Expected after
   fix:** freeze the electorate basis consistently with ballot weights (the jury
   pattern).

### WEB-1 — DB error disclosure
- Induce a DB error (e.g. a constraint violation) on a write handler; the response
  body currently contains raw `sqlx` text (schema/column/constraint names).
  **Expected after fix:** a generic client message; detail only in server logs.

### DEP-1 — weak `.env` boots anyway
- Copy `deploy/.env.example` → `.env` unedited and bring up the federation compose.
  The `${VAR:?}` guards check presence only, so it boots with world-known MinIO
  creds and a guessable cluster token. **Expected after fix:** the app/entrypoint
  rejects `change-me…`/low-entropy `DEMOCRATOS_CLUSTER_TOKEN`, `DB_PASSWORD`,
  `MINIO_ROOT_*`, mirroring the session-secret gate.

---

## Coverage note

`security.sh`/`posture.sh` cover every *single-node and node-auth* guardrail with
`curl` + `psql`. The federation-trust findings (FED-1/2/3, HIGH-1 exploit) and the
governance-integrity findings (GOV-2/3/4/5) are driven by the manual scenarios above
because they require a second cooperating (or malicious) node emitting crafted signed
events — the right home for those is a Rust integration test alongside
`crates/adapter-federation/tests/{ingest,gateway,forwarding}.rs`.
