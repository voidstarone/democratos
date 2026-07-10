#!/usr/bin/env bash
# Security / abuse scenarios against a RUNNING federated dev cluster (./up.sh).
# Where functional.sh asserts the happy-path guarantees, this asserts the
# *adversarial* ones: a forged cookie authenticates nobody, a non-member can't
# vote, the change feed and command/ingest endpoints demand a node token, the
# security headers are served, and auth endpoints are rate-limited. It further
# pokes at privileged-action IDOR (closing someone else's proposal), session-
# cookie hardening, feed input clamping, HTTP method tampering, and the
# open-redirect guard — and regression-guards the closed HIGH-1 comment-scope fix.
#
# Two kinds of check:
#   ok/bad   — GUARDRAILS. A defence that MUST hold; a failure fails the suite.
#   xfail    — KNOWN-OPEN findings from the audit. The attack currently succeeds;
#              reported as ⚠ but does NOT fail the suite. When the fix lands, the
#              same probe flips to ok and you delete the xfail.
#
# Auth note: the signed-cookie hardening means a bare `uid=N` cookie no longer
# authenticates (that is exactly SCENARIO 1). To act AS a user we mint a real
# signed cookie the way a browser would — via the dev account switcher
# (`/dev/unlock` → `/dev/switch`), which only the `--dev` cluster exposes.
#
# Usage: ./up.sh && ./security.sh          (needs NODES>=2 for the forwarding check)
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
# lib.sh enables `set -euo pipefail`. Turn OFF `-e` (probes deliberately hit
# endpoints that return non-2xx — a rejected forgery is a *pass*) and `-u` (this
# script prints status strings containing non-ASCII, which bash 3.2 on macOS
# mis-parses as variable names when adjacent to `$`). Keep pipefail.
set +eu
set -o pipefail

pass=0; fail=0; xf=0
ok()    { echo "  ✓ $1"; pass=$((pass+1)); }
bad()   { echo "  ✗ $1"; fail=$((fail+1)); }
xfail() { echo "  ⚠ KNOWN-OPEN: $1"; xf=$((xf+1)); }
jqget() { python3 -c "import json,sys;print(json.load(open('$1'))$2)"; }

curl -fsS -o /dev/null "$(web_url 1)/" 2>/dev/null || { echo "cluster not up — run ./up.sh first" >&2; exit 1; }

# Mint a valid signed session cookie for an existing user id, via the dev
# switcher, into a curl cookie jar. Echoes the jar path.
sign_in() { # web_url uid
  local base="$1" id="$2"; local jar="$RUN/sec.$id.jar"
  rm -f "$jar"
  curl -s -c "$jar" -o /dev/null "$base/dev/unlock"                                  # dev_login cookie
  curl -s -b "$jar" -c "$jar" -o /dev/null -X POST "$base/dev/switch" --data "id=$id" # signed uid cookie
  echo "$jar"
}
# Create a brand-new account (joins nothing) and sign in as it. Echoes the jar.
sign_in_new() { # web_url handle
  local base="$1" handle="$2"; local jar="$RUN/sec.new.$handle.jar"
  rm -f "$jar"
  curl -s -c "$jar" -o /dev/null "$base/dev/unlock"
  curl -s -b "$jar" -c "$jar" -o /dev/null -X POST "$base/dev/create" --data "handle=$handle"
  echo "$jar"
}
cookie_val() { awk -v n="$2" '$6==n {print $7}' "$1"; }  # value of cookie $2 in netscape jar $1

vote_jar() { curl -s -o /dev/null -w '%{http_code}' -X POST "$1/p/$PID/vote" -b "$2"           -H "x-requested-with: t" --data "choice=$3"; }
vote_raw() { curl -s -o /dev/null -w '%{http_code}' -X POST "$1/p/$PID/vote" -H "Cookie: $2"    -H "x-requested-with: t" --data "choice=$3"; }
tally()    { psql_node 1 -c "SELECT count(*) FROM votes WHERE proposal_id=$PID;"; }

echo "── seeding a small community on node 1 (owner) ──"
M="$RUN/security-manifest.json"
"$LOADGEN" seed --owner-db "$(owner_db 1)" --node-id 1 --voters 20 --slug security --out "$M" >/dev/null
DEMOS=$(jqget "$M" "['demos_id']"); PID=$(jqget "$M" "['proposal_id']")
V0=$(jqget "$M" "['voter_ids'][0]"); V1=$(jqget "$M" "['voter_ids'][1]")

echo "── waiting for replication to node 2 ──"
for i in $(seq 1 25); do
  m2=$(psql_node 2 -c "SELECT count(*) FROM memberships WHERE demos_id=$DEMOS;" 2>/dev/null || echo 0)
  [ "${m2:-0}" -ge 20 ] && break; sleep 2
done

echo "── assertions ──"

# SCENARIO 1 (guardrail) — forged/unsigned session cookie authenticates nobody.
# The classic attack: hand-write `uid=<victim>` with no HMAC tag.
b=$(tally); code=$(vote_raw "$(web_url 1)" "uid=$V0" aye); a=$(tally)
{ [ "$code" = "401" ] && [ "$a" -eq "$b" ]; } \
  && ok "forged unsigned cookie rejected (401), no vote recorded" \
  || bad "forged cookie: http=$code tally $b→$a (expected 401 / unchanged)"

# SCENARIO 2 (guardrail) — a tampered *signed* cookie is rejected (HMAC binds it).
jar=$(sign_in "$(web_url 1)" "$V0")
uid_c=$(cookie_val "$jar" uid)
if [ -n "$uid_c" ]; then
  tampered="${uid_c%?}$([ "${uid_c: -1}" = 0 ] && echo 1 || echo 0)"   # flip last hex digit
  b=$(tally); code=$(vote_raw "$(web_url 1)" "uid=$tampered" aye); a=$(tally)
  { [ "$code" = "401" ] && [ "$a" -eq "$b" ]; } \
    && ok "tampered signed cookie rejected (401)" \
    || bad "tampered cookie: http=$code tally $b→$a (expected 401 / unchanged)"
else
  bad "could not mint a signed cookie via /dev/switch (is the cluster --dev?)"
fi

# SCENARIO 3 (guardrail) — login CSRF: POST /session with no csrf cookie/field is
# rejected BEFORE any credential/Argon2 work (handlers.rs:976).
code=$(curl -s -o /dev/null -w '%{http_code}' -X POST "$(web_url 1)/session" --data "handle=anyone&password=whatever")
[ "$code" != "200" ] && [ "$code" != "302" ] \
  && ok "login without a CSRF token is refused (http $code)" \
  || bad "login CSRF not enforced (http $code)"

# SCENARIO 4 (guardrail) — a non-member cannot vote in a community (no IDOR by id).
jar_out=$(sign_in_new "$(web_url 1)" "outsider$$")
b=$(tally); code=$(vote_jar "$(web_url 1)" "$jar_out" aye); a=$(tally)
{ [ "$code" != "200" ] && [ "$a" -eq "$b" ]; } \
  && ok "non-member vote refused (http $code), tally unchanged" \
  || bad "non-member was able to vote: http=$code tally ${b}->${a}"

# SCENARIO 5 (guardrail) — federation enforces require_signatures: a KEYLESS user
# cannot cast a vote (the H5 fix that closed keyless-vote forgery on forwarded
# writes). Seeded voters have no enrolled key, so an authenticated-but-keyless
# vote must be refused with the "requires a signing key" message, tally unchanged.
# (A full forward + double-vote flow needs a real Ed25519-signed ballot — see the
# manual scenarios in SECURITY_SCENARIOS.md and the Rust test gateway.rs.)
jark=$(sign_in "$(web_url 1)" "$V1")
b=$(tally); body=$(curl -s -X POST "$(web_url 1)/p/$PID/vote" -b "$jark" -H "x-requested-with: t" --data "choice=aye"); a=$(tally)
{ echo "$body" | grep -qi "signing key" && [ "$a" -eq "$b" ]; } \
  && ok "federation require_signatures: keyless vote refused, tally unchanged" \
  || bad "keyless vote NOT refused as expected: '$body' tally ${b}->${a}"

# SCENARIO 6 (guardrail) — the change feed demands a valid node bearer token.
FED="$(fed_url 1)"
feed() { curl -s -o /dev/null -w '%{http_code}' "$FED/federation/changes?since=0&limit=1" ${1:+-H "Authorization: Bearer $1"}; }
n=$(feed ""); w=$(feed "wrong-token"); g=$(feed "$CLUSTER_TOKEN")
{ [ "$n" = "401" ] && [ "$w" = "401" ] && [ "$g" = "200" ]; } \
  && ok "change feed: no-token=401, wrong=401, correct=200" \
  || bad "change feed auth wrong: none=$n wrong=$w correct=$g"

# SCENARIO 7 (guardrail) — an unauthenticated caller cannot get a write applied by
# the command / ingest endpoints (they reject with 4xx; S6 already proves the
# bearer mechanism cleanly on the feed). The token is checked inside the handler,
# after body extraction, so a no-token request surfaces as 4xx (401/415/422) — the
# security property is simply that it never succeeds (2xx).
rejected() { case "$1" in 2*) return 1;; *) return 0;; esac; }
cc=$(curl -s -o /dev/null -w '%{http_code}' -X POST "$FED/federation/command" -H 'content-type: application/json' --data '{}')
ic=$(curl -s -o /dev/null -w '%{http_code}' -X POST "$FED/federation/ingest"  -H 'content-type: application/json' --data '{}')
{ rejected "$cc" && rejected "$ic"; } \
  && ok "command & ingest reject an unauthenticated write (command=$cc ingest=$ic)" \
  || bad "command/ingest accepted an unauthenticated write: command=$cc ingest=$ic"

# SCENARIO 8 (guardrail) — security headers on a normal page.
H="$(curl -sSI "$(web_url 1)/")"
hdr() { echo "$H" | grep -qi "^$1:"; }
{ hdr content-security-policy && hdr strict-transport-security && hdr x-frame-options && hdr x-content-type-options; } \
  && ok "security headers present (CSP/HSTS/X-Frame-Options/nosniff)" \
  || bad "a required security header is missing"

# SCENARIO 9 (guardrail) — auth endpoints are rate-limited (Argon2-DoS / brute
# force). AUTH_MAX_REQUESTS=10/60s per peer IP; a burst must start returning 429.
hits=0
for i in $(seq 1 15); do
  c=$(curl -s -o /dev/null -w '%{http_code}' -X POST "$(web_url 1)/session" --data "handle=b$i&password=x")
  [ "$c" = "429" ] && hits=$((hits+1))
done
[ "$hits" -ge 1 ] && ok "POST /session is rate-limited under a burst (${hits}x 429)" \
                   || bad "no 429 seen in a 15-request burst — rate limiting not effective"

# SCENARIO 10 (guardrail) — HIGH-1 regression guard. A comment carries no
# `demos_id` of its own, so its community must be resolved at authorisation time
# from its parent post (`event_scope` → ViaParent{Post}); a `demos_id=NULL`
# comment must NOT authorise globally. The earlier probe grepped the outbox
# trigger for a 'comments' branch — but the real fix lives in `event_scope`
# (the trigger deliberately still emits NULL for comments, which is now safe
# because scope is derived from the parent at auth time). We assert the fix at
# its true site so a revert trips the suite. The crafted cross-community forgery
# (a foreign node mutating our comment → NotOwner) is exercised end-to-end by the
# byzantine harness (deploy/byzantine) and the federation unit tests.
if grep -Eq '"comments" *=> *match id\("post_id"\)' "$ROOT/crates/federation/src/ownership.rs"; then
  ok "HIGH-1 fixed: comment events resolve their community via the parent post (event_scope ViaParent{Post})"
else
  bad "HIGH-1 REGRESSED: event_scope no longer routes comments via their parent post — comment events would authorise globally"
fi

# SCENARIO 11 (guardrail) — privileged-action IDOR. Closing a proposal freezes
# its tally and can apply a rule change; it is gated to voters of that proposal's
# community (handlers.rs close_proposal). An outsider (member of nothing) and an
# anonymous caller must both be refused, so nobody can close an arbitrary
# proposal by id to freeze a tally at a chosen moment.
b_out=$(curl -s -X POST "$(web_url 1)/p/$PID/close" -b "$jar_out")
b_anon=$(curl -s -X POST "$(web_url 1)/p/$PID/close")
{ echo "$b_out" | grep -qi "only a voter" && echo "$b_anon" | grep -qi "sign in"; } \
  && ok "close_proposal refused for outsider (‘only a voter’) and anon (‘sign in’)" \
  || bad "close_proposal not gated: outsider='${b_out:0:60}' anon='${b_anon:0:60}'"

# SCENARIO 12 (guardrail) — a server-minted session cookie is hardened. The uid
# cookie the server sets (via the dev switcher, same `uid_cookie` builder the
# real login uses) must carry HttpOnly (no JS theft) + SameSite=Lax (login-CSRF /
# cross-site defence) + a bounded Max-Age (a stolen cookie has a finite life).
hj="$RUN/sec.hdr.$$"; uj="$RUN/sec.unlock.$$.jar"
curl -s -c "$uj" -o /dev/null "$(web_url 1)/dev/unlock"
curl -s -b "$uj" -D "$hj" -o /dev/null -X POST "$(web_url 1)/dev/switch" --data "id=$V0"
setck=$(grep -i '^set-cookie:.*uid=' "$hj" | head -1)
{ echo "$setck" | grep -qi "HttpOnly" && echo "$setck" | grep -qi "SameSite=Lax" && echo "$setck" | grep -qi "Max-Age="; } \
  && ok "session cookie is HttpOnly + SameSite=Lax + bounded Max-Age" \
  || bad "session cookie missing a hardening attribute: '${setck#*:}'"

# SCENARIO 13 (guardrail) — feed input hardening. `limit` is clamped server-side
# to [1, 5000] (http.rs), so an attacker-supplied negative or oversized limit
# must NOT error or exfiltrate the whole log in one shot — the endpoint stays 200.
neg=$(curl -s -o /dev/null -w '%{http_code}' "$FED/federation/changes?since=0&limit=-5"     -H "Authorization: Bearer $CLUSTER_TOKEN")
big=$(curl -s -o /dev/null -w '%{http_code}' "$FED/federation/changes?since=-1&limit=999999" -H "Authorization: Bearer $CLUSTER_TOKEN")
{ [ "$neg" = "200" ] && [ "$big" = "200" ]; } \
  && ok "feed clamps hostile since/limit values (neg=$neg huge=$big, both 200)" \
  || bad "feed mishandled hostile since/limit: neg=$neg huge=$big (expected 200/200)"

# SCENARIO 14 (guardrail) — HTTP method tampering. Mutating routes are POST-only;
# a GET must not reach the handler (axum answers 405 with an Allow header). Guards
# against a state change smuggled through a method a CSRF/SameSite defence or a
# cache/prefetch might treat as safe.
gv=$(curl -s -o /dev/null -w '%{http_code}' "$(web_url 1)/p/$PID/vote")
gs=$(curl -s -o /dev/null -w '%{http_code}' "$(web_url 1)/session")
{ [ "$gv" = "405" ] && [ "$gs" = "405" ]; } \
  && ok "GET on POST-only routes is 405 (/p/:id/vote, /session)" \
  || bad "a POST-only route answered GET: vote=$gv session=$gs (expected 405)"

# SCENARIO 15 (guardrail) — open-redirect defence. `POST /lang` bounces back to
# the `Referer`, but `safe_referer_back`/`local_path_of` reduce it to a same-origin
# path (an absolute URL keeps only its path; a protocol-relative `//host` falls
# back to `/`). A crafted Referer must never send the browser off-site.
lh="$RUN/sec.lang.$$"
curl -s -D "$lh" -o /dev/null -X POST "$(web_url 1)/lang" -H "Referer: http://evil.example/pwned" --data "lang=en"
loc1=$(grep -i '^location:' "$lh" | head -1 | tr -d '\r' | awk '{print $2}')
curl -s -D "$lh" -o /dev/null -X POST "$(web_url 1)/lang" -H "Referer: //evil.example/x" --data "lang=en"
loc2=$(grep -i '^location:' "$lh" | head -1 | tr -d '\r' | awk '{print $2}')
{ [ "$loc1" = "/pwned" ] && [ "$loc2" = "/" ]; } \
  && ok "open-redirect guard: off-site Referer reduced to same-origin path (abs→$loc1, proto-rel→$loc2)" \
  || bad "open-redirect guard leaked off-site: abs='$loc1' proto-rel='$loc2' (expected /pwned and /)"

# SCENARIO 16 (guardrail) — CSRF is actually *validated*, not merely required.
# S3 proves a token-less POST is refused; this proves a token that DOESN'T match
# the cookie is refused too (the double-submit compare, constant-time), while the
# CORRECT token gets past CSRF into (failing) auth — so the check keys on the
# value, not mere presence of any field.
sj="$RUN/sec.signin.$$.jar"
tok=$(curl -s -c "$sj" "$(web_url 1)/signin" | grep -oiE 'name="csrf_token"[^>]*value="[^"]*"' | grep -oE 'value="[^"]*"' | head -1 | sed 's/value="//;s/"//')
mism=$(curl -s -b "$sj" -X POST "$(web_url 1)/session" --data "email=nobody@x&password=x&csrf_token=WRONG$tok")
good=$(curl -s -b "$sj" -X POST "$(web_url 1)/session" --data "email=nobody@x&password=x&csrf_token=$tok")
{ [ -n "$tok" ] && echo "$mism" | grep -qi "session expired" && ! echo "$good" | grep -qi "session expired"; } \
  && ok "CSRF is value-checked: mismatched token refused, correct token passes to auth" \
  || bad "CSRF not value-checked (tok='${tok:0:8}…' mism-has-expired=$(echo "$mism"|grep -qi 'session expired' && echo y||echo n) good-has-expired=$(echo "$good"|grep -qi 'session expired' && echo y||echo n))"

# SCENARIO 17 (guardrail) — reflected XSS. `/search` echoes the raw query into
# visible text and an input `value="…"`; the templates auto-escape both, so an
# injected payload must never survive un-escaped in the real (postgres-backed)
# render path. Probe a tag injection and an attribute breakout.
xb=$(curl -s "$(web_url 1)/search?q=%3Cscript%3Ealert(1)%3C%2Fscript%3E")
xb2=$(curl -s "$(web_url 1)/search?q=%22%3E%3Cimg+src%3Dx+onerror%3Dalert(1)%3E")
{ ! echo "$xb" | grep -qiF "<script>alert(1)" && ! echo "$xb2" | grep -qiF "<img src=x onerror=alert(1)>"; } \
  && ok "reflected search query is HTML-escaped (no live <script>, no attribute breakout)" \
  || bad "XSS: an injected payload survived un-escaped in /search output"

echo
echo "── security: $pass passed, $fail failed, $xf known-open ──"
[ "$fail" -eq 0 ]
