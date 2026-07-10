#!/usr/bin/env bash
# Byzantine (majority-compromised) scenarios against a running cluster (./up.sh).
#
# Thesis: in this open federation, a community's authority is its FOUNDER KEY, not
# node-count or etcd-write. So an attacker who controls the MAJORITY of nodes + the
# cluster token + the etcd control plane still cannot seize, forge into, or rewrite
# an honest, founder-bound community. The known-open holes are exercised as xfails.
#
#   ok/bad  GUARDRAIL — the crypto trust model must hold; a failure fails the suite.
#   xfail   KNOWN-OPEN — an audit finding; the attack succeeds today (reported ⚠).
#   probe   an empirical question the harness answers (reported, never fails).
#
# Usage:  ./up.sh && ./byzantine.sh
set -uo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
[ -f "$ROOT/deploy/byzantine/.run/env" ] && source "$ROOT/deploy/byzantine/.run/env"
DEMOS="${DEMOS:-}"; SLUG="${SLUG:-honest-1}"
[ -n "$DEMOS" ] || { echo "no cluster state — run ./up.sh first" >&2; exit 1; }

pass=0; fail=0; xf=0
ok()    { echo "  ✓ $1"; pass=$((pass+1)); }
bad()   { echo "  ✗ $1"; fail=$((fail+1)); }
xfail() { echo "  ⚠ KNOWN-OPEN: $1"; xf=$((xf+1)); }
probe() { echo "  • PROBE: $1"; }

# Pick a compromised node id (first one past the honest set) to model a *captured*
# real cluster member acting maliciously.
CN=$((HONEST + 1))
slug_of() { psql_db 1 -c "SELECT slug FROM demoi WHERE id=$1;"; }   # on node 1's replica
outcome() { echo "$1" | grep -o 'OUTCOME=[^ ]*' | tail -1; }

echo "── Byzantine cluster: $NODES nodes, honest 1..$HONEST, compromised $((HONEST+1))..$NODES + rogue $ROGUE_ID ──"
echo "── honest community d/$DEMOS (owned + founder-bound by node 1) ──"
echo

# G1 (guardrail) — a compromised node cannot seize the honest community via the
# control plane: publishing a rival community key is refused (first-write-wins) and
# claiming the live-owned, founder-bound lease yields no ownership.
o=$(redteam --node "$CN" --seed "$(node_seed "$CN")" seize-bound --demos "$DEMOS" 2>/dev/null)
[ "$(outcome "$o")" = "OUTCOME=BLOCKED" ] \
  && ok "G1 control-plane seizure of the bound community is refused (compromised node $CN)" \
  || bad "G1 seizure NOT blocked: $(outcome "$o")"

# G2 (guardrail) — an EXTERNAL adversary (holds the cluster token + etcd access) that
# signs a demoi rewrite and pushes it to an honest node's ingest is rejected
# (NotOwner). Both the applied-count and the on-disk slug must show no change.
o=$(redteam --node 250 forge-event --demos "$DEMOS" --feed "$(fed_url 1)" 2>/dev/null)
s=$(slug_of "$DEMOS")
{ [ "$(outcome "$o")" = "OUTCOME=APPLIED:0" ] && [ "$s" = "$SLUG" ]; } \
  && ok "G2 forged event from an external adversary rejected (0 applied, slug still '$s')" \
  || bad "G2 forged event: $(outcome "$o"), slug=$s (expected APPLIED:0 / honest)"

# G3 (guardrail) — a CAPTURED real cluster member (compromised node, its own seed +
# published key) still cannot forge into a community it does not own.
o=$(redteam --node "$CN" --seed "$(node_seed "$CN")" forge-event --demos "$DEMOS" --feed "$(fed_url 1)" 2>/dev/null)
s=$(slug_of "$DEMOS")
{ [ "$(outcome "$o")" = "OUTCOME=APPLIED:0" ] && [ "$s" = "$SLUG" ]; } \
  && ok "G3 forged event from a captured cluster node rejected (0 applied, slug still '$s')" \
  || bad "G3 captured-node forgery: $(outcome "$o"), slug=$s"

# G4 (guardrail) — the ROGUE malicious peer (a node running attacker code that serves
# a forged feed) is pulled by the honest node and every event rejected. If it had
# been trusted, node 1's demoi slug would read 'pwned'.
sleep 3   # give node 1's puller a cycle against the rogue peer
s=$(slug_of "$DEMOS")
[ "$s" = "$SLUG" ] \
  && ok "G4 rogue peer's forged feed rejected by the honest node (slug still '$s')" \
  || bad "G4 rogue feed was accepted: slug=$s (expected honest)"

# G5 (guardrail) — epoch monotonicity: even holding the control plane, a second claim
# of the live-owned community does not roll back or steal the epoch.
own=$(redteam --node 250 seize-bound --demos "$DEMOS" 2>/dev/null | grep -c 'OUTCOME=BLOCKED')
[ "$own" = "1" ] \
  && ok "G5 repeated claims of the live-owned community never take ownership (epoch fenced)" \
  || bad "G5 epoch/ownership fence failed"

echo
echo "── previously-open findings, now closed (were xfail) ──"

# G6 (was xfail FED-1) — the durable takeover of a community (publish a community key
# + a fencing-surviving binding) is now blocked by origin-authentication: the attacker
# can only sign the community-key publish with its own node key, not the community's
# origin node key, so the registry refuses it. (A genuinely origin-less community's
# bare lease may still be claimable — the deliberate "unbound = unconstrained"
# contract — but without a verifying key/binding there is no fencing-surviving seizure.)
o=$(redteam --node 250 takeover-unbound --demos 999999 --feed "$(fed_url 1)" 2>/dev/null)
[ "$(outcome "$o")" = "OUTCOME=SEIZED:false" ] \
  && ok "G6 FED-1 closed: durable takeover blocked (origin-authenticated community-key publish)" \
  || bad "G6 FED-1 takeover not blocked: $(outcome "$o")"

# G7 (was probe HIGH-1) — a forged comment for a community the attacker doesn't own is
# rejected; and comments now scope via their parent post (they replicate again — the
# liveness fix, covered by the federation unit tests).
o=$(redteam --node 250 forge-comment --post 1 --feed "$(fed_url 1)" 2>/dev/null)
[ "$(outcome "$o")" = "OUTCOME=APPLIED:0" ] \
  && ok "G7 HIGH-1 closed: forged comment rejected; comments now scope via parent post" \
  || bad "G7 forged comment: $(outcome "$o") (expected APPLIED:0)"

# G8 (was xfail FED-3) — the control plane now verifies a home binding against the
# community key before storing it, so an attacker-signed binding is refused (no poison,
# no owner DoS). DESTRUCTIVE intent, but now a no-op — run last regardless.
o=$(redteam --node 250 poison-binding --demos "$DEMOS" 2>/dev/null)
[ "$(outcome "$o")" = "OUTCOME=POISONED:false" ] \
  && ok "G8 FED-3 closed: unverified home binding refused at set_home_binding (no poison/DoS)" \
  || bad "G8 FED-3 poisoning not blocked: $(outcome "$o")"

echo
echo "── byzantine: $pass guardrails passed, $fail failed, $xf known-open ──"
[ "$fail" -eq 0 ]
