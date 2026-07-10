#!/usr/bin/env bash
# Production-posture security checks. These need NO cluster and NO containers —
# just the built `democratos` binary and an in-memory store. They assert the
# fail-closed *startup* guards and the default (non-dev, security-header) HTTP
# posture that a real deployment relies on. Usage: ./posture.sh
#
# Complements security.sh (which exercises a running federated dev cluster). The
# two together map to docs/security-audit-2026-07-10-verification.md.
#
# NOTE: `set -e` is deliberately OFF — several checks run commands that are
# *expected* to fail (a fail-closed startup exits non-zero on purpose).
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
# lib.sh enables `set -e`; turn it back off — several checks below run commands
# that are *expected* to fail (a fail-closed startup exits non-zero on purpose).
set +e
set -uo pipefail

pass=0; fail=0
ok()  { echo "  ✓ $1"; pass=$((pass+1)); }
bad() { echo "  ✗ $1"; fail=$((fail+1)); }

[ -x "$BIN" ] || { echo "build first: cargo build -p democratos" >&2; exit 1; }
mkdir -p "$RUN"
PORT="${POSTURE_PORT:-3990}"
LONG_SECRET="$(printf 'a%.0s' $(seq 1 48))"   # 48 chars, passes the length gate

# Run `serve` with a given session secret + bind address, capture output and exit
# code. Kills the process after `wait_s` if it is still running (a clean start).
# Sets RC (exit code) and OUT (combined stdout/stderr).
run_serve() { # secret addr wait_s
  local secret="$1" addr="$2" wait_s="${3:-3}" out="$RUN/posture.out"
  DEMOCRATOS_SESSION_SECRET="$secret" \
    timeout "$wait_s" "$BIN" --store memory serve --addr "$addr" >"$out" 2>&1
  RC=$?; OUT="$(cat "$out")"
}

echo "── WEB-2 / session-secret fail-closed guards (main.rs:604-640) ──"

# P1: a placeholder secret on a NON-loopback bind must fail closed (bail!, RC≠0,
# and NOT a timeout — i.e. it exited on its own before ever binding).
run_serve "CHANGE_ME_openssl_rand_hex_32" "0.0.0.0:$PORT" 4
{ [ "$RC" -ne 0 ] && [ "$RC" -ne 124 ] && echo "$OUT" | grep -qi "placeholder or too short"; } \
  && ok "placeholder secret on 0.0.0.0 fails closed (rc=$RC)" \
  || bad "placeholder secret on 0.0.0.0 did NOT fail closed (rc=$RC): ${OUT:0:120}"

# P2: a too-short secret on a non-loopback bind must also fail closed.
run_serve "tooshort" "0.0.0.0:$PORT" 4
{ [ "$RC" -ne 0 ] && [ "$RC" -ne 124 ]; } \
  && ok "short (<16 char) secret on 0.0.0.0 fails closed (rc=$RC)" \
  || bad "short secret on 0.0.0.0 did NOT fail closed (rc=$RC)"

# P3: the SAME placeholder on a LOOPBACK bind must only warn and keep serving
# (a bare local dev run stays frictionless). timeout kills it → rc 124 = it ran.
run_serve "CHANGE_ME_openssl_rand_hex_32" "127.0.0.1:$PORT" 3
{ [ "$RC" -eq 124 ] && echo "$OUT" | grep -qi "placeholder"; } \
  && ok "placeholder secret on loopback warns but serves (rc=$RC)" \
  || bad "loopback placeholder behaviour unexpected (rc=$RC): ${OUT:0:120}"

echo "── default HTTP posture: non-dev node on loopback ──"
DEMOCRATOS_SESSION_SECRET="$LONG_SECRET" \
  "$BIN" --store memory serve --addr "127.0.0.1:$PORT" >"$RUN/posture-node.log" 2>&1 &
NODE_PID=$!
trap 'kill "$NODE_PID" 2>/dev/null' EXIT
if ! wait_http "http://127.0.0.1:$PORT/" 15; then
  bad "posture node failed to start; see $RUN/posture-node.log"
else
  BASE="http://127.0.0.1:$PORT"

  # P4: security headers present on a normal page (lib.rs:48-73).
  H="$(curl -sSI "$BASE/")"
  hdr() { echo "$H" | grep -qi "^$1:"; }
  { hdr content-security-policy && hdr strict-transport-security \
      && hdr x-frame-options && hdr x-content-type-options; } \
    && ok "security headers present (CSP, HSTS, X-Frame-Options, nosniff)" \
    || bad "a required security header is missing"
  echo "$H" | grep -qi "content-security-policy:.*script-src 'self'" \
    && ok "CSP pins script-src to 'self'" \
    || bad "CSP does not pin script-src 'self'"

  # P5: dev tooling is inert without --dev (dev.rs — every dev handler 404s).
  code=$(curl -s -o /dev/null -w '%{http_code}' "$BASE/dev/accounts")
  [ "$code" = "404" ] && ok "/dev/accounts is 404 without --dev" \
                       || bad "/dev/accounts returned $code without --dev (expected 404)"
  code=$(curl -s -o /dev/null -w '%{http_code}' "$BASE/dev/unlock")
  [ "$code" = "404" ] && ok "/dev/unlock is 404 without --dev" \
                       || bad "/dev/unlock returned $code without --dev (expected 404)"

  # P6: a forged / unsigned session cookie authenticates NOBODY (handlers.rs:420).
  # `uid=1` with no HMAC tag must be treated as no session even on a mutating route.
  code=$(curl -s -o /dev/null -w '%{http_code}' -X POST "$BASE/p/1/vote" \
           -H "Cookie: uid=1" -H "x-requested-with: t" --data "choice=aye")
  [ "$code" = "401" ] && ok "unsigned uid=1 cookie is rejected (401), no raw-uid bypass" \
                      || bad "unsigned uid=1 cookie got $code (expected 401)"

  # P7: defence-in-depth headers beyond the four in P4 (lib.rs security_headers).
  # A referrer-policy (no cross-origin URL leak) plus a CSP that pins object-src
  # 'none' (no plugin exec), frame-ancestors 'none' (clickjacking), base-uri
  # 'self' (no <base> hijack) and form-action 'self' (no off-site form post).
  hdr() { echo "$H" | grep -qi "^$1:"; }
  csp="$(echo "$H" | grep -i '^content-security-policy:')"
  has() { echo "$csp" | grep -qi -- "$1"; }
  { hdr referrer-policy && has "object-src 'none'" && has "frame-ancestors 'none'" \
      && has "base-uri 'self'" && has "form-action 'self'"; } \
    && ok "hardening headers present (referrer-policy; CSP object-src/frame-ancestors/base-uri/form-action)" \
    || bad "a defence-in-depth header/CSP directive is missing"

  # P8: HTTP method tampering — mutating routes are POST-only; a GET must not
  # reach the handler (axum answers 405). Stops a state change smuggled through a
  # method a CSRF/SameSite defence or a cache/prefetch treats as safe.
  m1=$(curl -s -o /dev/null -w '%{http_code}' "$BASE/session")
  m2=$(curl -s -o /dev/null -w '%{http_code}' "$BASE/logout")
  { [ "$m1" = "405" ] && [ "$m2" = "405" ]; } \
    && ok "GET on POST-only routes is 405 (/session, /logout)" \
    || bad "a POST-only route answered GET: session=$m1 logout=$m2 (expected 405)"

  # P9: open-redirect defence — `POST /lang` bounces back to the `Referer`, but
  # `local_path_of` reduces an absolute URL to its same-origin path and rejects a
  # protocol-relative `//host` (falls back to `/`). A crafted Referer must never
  # send the browser off-site (handlers.rs local_path_of).
  L="$RUN/posture.lang"
  curl -s -D "$L" -o /dev/null -X POST "$BASE/lang" -H "Referer: http://evil.example/pwned" --data "lang=en"
  loc1=$(grep -i '^location:' "$L" | head -1 | tr -d '\r' | awk '{print $2}')
  curl -s -D "$L" -o /dev/null -X POST "$BASE/lang" -H "Referer: //evil.example/x" --data "lang=en"
  loc2=$(grep -i '^location:' "$L" | head -1 | tr -d '\r' | awk '{print $2}')
  { [ "$loc1" = "/pwned" ] && [ "$loc2" = "/" ]; } \
    && ok "open-redirect guard: off-site Referer reduced to same-origin path (abs→$loc1, proto-rel→$loc2)" \
    || bad "open-redirect guard leaked off-site: abs='$loc1' proto-rel='$loc2' (expected /pwned and /)"

  # P10: reflected-XSS escaping. `/search` echoes the raw query into visible text
  # AND an input `value="…"` (search.html), so a crafted `q` is the classic
  # reflected-XSS vector. Askama auto-escapes both contexts — the injected markup
  # must never appear un-escaped in the response (no live <script>, no attribute
  # breakout). We probe both a tag-injection and an attribute-breakout payload.
  body=$(curl -s "$BASE/search?q=%3Cscript%3Ealert(1)%3C%2Fscript%3E")             # <script>alert(1)</script>
  body2=$(curl -s "$BASE/search?q=%22%3E%3Cimg+src%3Dx+onerror%3Dalert(1)%3E")      # "><img src=x onerror=alert(1)>
  { ! printf '%s' "$body" | grep -qiF "<script>alert(1)" \
      && ! printf '%s' "$body2" | grep -qiF "<img src=x onerror=alert(1)>"; } \
    && ok "reflected search query is HTML-escaped (no live <script>, no attribute breakout)" \
    || bad "XSS: an injected payload survived un-escaped in /search output"

  # P11: security headers cover the WHOLE router, not just happy-path pages — the
  # middleware is `.layer()`ed over every response, so an error/404 must still
  # carry the framing + CSP defences (a common gap: headers only on 200s).
  E="$(curl -sSI "$BASE/no-such-path-$$-xyz")"
  ec=$(printf '%s' "$E" | head -1 | awk '{print $2}')
  ehdr() { printf '%s' "$E" | grep -qi "^$1:"; }
  { [ "$ec" = "404" ] && ehdr x-frame-options && ehdr content-security-policy && ehdr x-content-type-options; } \
    && ok "security headers present on a 404 (framing/CSP/nosniff cover error responses)" \
    || bad "a 404 ($ec) is missing security headers (middleware not covering error paths)"

  # P12: path-parameter hardening. `/post/:id` is typed `Path<u64>`, so a
  # non-numeric, negative, or overflowing id must be rejected by the extractor as a
  # clean 4xx — never a 500/panic that would signal a parsing weakness to probe.
  bad_ids=0
  for pid in "abc" "-1" "99999999999999999999999" "1e9" "0x10"; do
    c=$(curl -s -o /dev/null -w '%{http_code}' "$BASE/post/$pid")
    case "$c" in 5*) bad_ids=$((bad_ids+1)); echo "      /post/$pid → $c";; esac
  done
  [ "$bad_ids" -eq 0 ] \
    && ok "hostile /post/:id values (non-numeric/negative/overflow) rejected without a 5xx" \
    || bad "$bad_ids hostile id(s) triggered a 5xx on /post/:id"

  # P13: a sensitive account action is auth-gated. Enrolling an Ed25519 public key
  # binds a signing identity to an account; an unauthenticated POST must be refused
  # ("sign in first") and enroll nothing — never bind a key to a session-less caller.
  ek=$(curl -s -X POST "$BASE/account/key" --data "public_key=deadbeef")
  printf '%s' "$ek" | grep -qi "sign in" \
    && ok "enroll_key refuses an unauthenticated caller (no key bound without a session)" \
    || bad "enroll_key did not refuse an unauthenticated POST: '${ek:0:80}'"
fi

echo "── dev account switcher: secret-gated, puppet-only (dev-federation setup) ──"
# The dev switcher lets a browser act as any *puppet* account with no password —
# catastrophic if anyone could reach it. These checks assert the two guarantees
# the private dev node relies on: (1) the fail-closed startup guard, and (2) at
# runtime, unlock demands the secret and the switcher only ever reaches the fixed
# franchise-barred puppet accounts. A dedicated dev node is started for this.
DPORT="${POSTURE_DEV_PORT:-3994}"
DSECRET="posture-dev-secret"

# PD1: --dev on a NON-loopback bind without an unlock secret must fail closed —
# otherwise anyone who can reach the node could unlock the switcher.
run_serve_dev() { # addr wait_s  (dev on, NO secret)
  local addr="$1" wait_s="${2:-4}" out="$RUN/posture.devguard.out"
  DEMOCRATOS_SESSION_SECRET="$LONG_SECRET" \
    timeout "$wait_s" "$BIN" --store memory serve --addr "$addr" --dev >"$out" 2>&1
  RC=$?; OUT="$(cat "$out")"
}
run_serve_dev "0.0.0.0:$DPORT" 4
{ [ "$RC" -ne 0 ] && [ "$RC" -ne 124 ] && echo "$OUT" | grep -qi "unlock-secret"; } \
  && ok "dev on a non-loopback bind without an unlock secret fails closed (rc=$RC)" \
  || bad "exposed --dev without a secret did NOT fail closed (rc=$RC): ${OUT:0:120}"

# Now a properly-configured private dev node: loopback, secret, two barred puppets.
DEMOCRATOS_SESSION_SECRET="$LONG_SECRET" \
  "$BIN" --store memory serve --addr "127.0.0.1:$DPORT" \
    --dev --dev-unlock-secret "$DSECRET" --dev-accounts pptt-alice,pptt-bob \
    >"$RUN/posture-dev.log" 2>&1 &
DEV_PID=$!
trap 'kill "$NODE_PID" "$DEV_PID" 2>/dev/null' EXIT
if ! wait_http "http://127.0.0.1:$DPORT/" 15; then
  bad "dev posture node failed to start; see $RUN/posture-dev.log"
else
  DB="http://127.0.0.1:$DPORT"

  # PD2: unlock demands the secret. No key and a wrong key are BOTH 404 (identical
  # to dev-off — the endpoint reveals nothing); the correct key hands out the
  # dev_login cookie.
  nk=$(curl -s -o /dev/null -w '%{http_code}' "$DB/dev/unlock")
  wk=$(curl -s -o /dev/null -w '%{http_code}' "$DB/dev/unlock?key=wrong")
  gk=$(curl -s -D - -o /dev/null "$DB/dev/unlock?key=$DSECRET")
  gkcode=$(printf '%s' "$gk" | head -1 | awk '{print $2}')
  { [ "$nk" = "404" ] && [ "$wk" = "404" ] && printf '%s' "$gk" | grep -qi '^set-cookie:.*dev_login='; } \
    && ok "dev/unlock: no key=404, wrong key=404, correct key issues the unlock cookie (http $gkcode)" \
    || bad "dev/unlock secret gate wrong: none=$nk wrong=$wk correct=$gkcode"

  # PD3: switcher is puppet-only. /dev/accounts lists exactly the barred puppets;
  # switching to one works (204); switching to a non-puppet id is refused (404).
  curl -s -c "$RUN/posture-dev.jar" -o /dev/null "$DB/dev/unlock?key=$DSECRET"
  acc=$(curl -s -b "$RUN/posture-dev.jar" "$DB/dev/accounts")
  swc=$(curl -s -b "$RUN/posture-dev.jar" -o /dev/null -w '%{http_code}' -X POST "$DB/dev/switch" --data "id=1")
  badswc=$(curl -s -b "$RUN/posture-dev.jar" -o /dev/null -w '%{http_code}' -X POST "$DB/dev/switch" --data "id=999999")
  { echo "$acc" | grep -q "pptt-alice" && echo "$acc" | grep -q "pptt-bob" \
      && [ "$swc" = "204" ] && [ "$badswc" = "404" ]; } \
    && ok "switcher lists only the barred puppets; switch to a puppet=204, to a non-puppet id=404" \
    || bad "switcher not puppet-scoped: accounts='${acc:0:80}' switch=$swc non-puppet=$badswc"
fi

echo
echo "── posture: $pass passed, $fail failed ──"
[ "$fail" -eq 0 ]
