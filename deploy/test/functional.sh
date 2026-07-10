#!/usr/bin/env bash
# Functional correctness check for a running federation. Asserts the guarantees
# the design promises: replication, write-forwarding to the owner, no
# double-voting across nodes, synchronous standby durability, and convergence.
# Assumes the cluster is up (./up.sh). Usage: ./functional.sh
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

pass=0; fail=0
ok()   { echo "  ✓ $1"; pass=$((pass+1)); }
bad()  { echo "  ✗ $1"; fail=$((fail+1)); }
jqget(){ python3 -c "import json,sys;print(json.load(open('$1'))$2)"; }

M="$RUN/functional-manifest.json"
echo "── seeding a small community on node 1 (owner) ──"
"$LOADGEN" seed --owner-db "$(owner_db 1)" --node-id 1 --voters 20 --slug functional --out "$M" >/dev/null
DEMOS=$(jqget "$M" "['demos_id']"); PID=$(jqget "$M" "['proposal_id']")
V0=$(jqget "$M" "['voter_ids'][0]"); V1=$(jqget "$M" "['voter_ids'][1]")

echo "── waiting for replication + ownership ──"
for i in $(seq 1 25); do
  m2=$(psql_node 2 -c "SELECT count(*) FROM memberships WHERE demos_id=$DEMOS;")
  st=$(psql_node 2 -c "SELECT count(*) FROM demoi WHERE id=$DEMOS;")
  { [ "${m2:-0}" -ge 20 ] && [ "${st:-0}" = "1" ]; } && break
  sleep 2
done

echo "── assertions ──"
# 1. Replication: node2 has node1's community + members.
d2=$(psql_node 2 -c "SELECT count(*) FROM demoi WHERE id=$DEMOS;")
m2=$(psql_node 2 -c "SELECT count(*) FROM memberships WHERE demos_id=$DEMOS;")
{ [ "$d2" = "1" ] && [ "$m2" -ge 20 ]; } && ok "replication: node2 mirrors node1's community ($m2 members)" \
  || bad "replication: node2 has demoi=$d2 members=$m2 (expected 1 / 20)"

vote() { # node_url voter choice  -> http code
  curl -s -o /dev/null -w "%{http_code}" -X POST "$1/p/$PID/vote" \
    -H "Cookie: uid=$2" -H "x-requested-with: t" --data "choice=$3"
}
tally_owner() { local n; n=$(psql_node 1 -c "SELECT count(*) FROM votes WHERE proposal_id=$PID;"); echo "${n:-0}"; }

# 2. Forwarding: a vote cast on node2 (NOT the owner) is recorded on node1.
before=$(tally_owner)
code=$(vote "$(web_url 2)" "$V0" aye)
after=$(tally_owner)
{ [ "$code" = "200" ] && [ "$after" -eq $((before+1)) ]; } \
  && ok "forwarding: a vote cast on the non-owner node landed on the owner" \
  || bad "forwarding: http=$code owner-tally $before→$after"

# 3. No double-voting across nodes: the same voter voting again (on the OTHER
#    node) is refused and does not change the tally.
before=$(tally_owner)
code=$(vote "$(web_url 1)" "$V0" nay)
after=$(tally_owner)
{ [ "$code" != "200" ] && [ "$after" -eq "$before" ]; } \
  && ok "no double-vote: the same voter is refused on a second node (http $code)" \
  || bad "double-vote NOT prevented: http=$code tally $before→$after"

# 4. Synchronous durability: a vote accepted on the owner is already on the
#    standby (quorum of 2) essentially immediately.
code=$(vote "$(web_url 1)" "$V1" aye)
sleep 0.3
onstandby=$(psql_node 2 -c "SELECT count(*) FROM votes WHERE proposal_id=$PID AND voter_id=$V1;")
{ [ "$code" = "200" ] && [ "$onstandby" = "1" ]; } \
  && ok "sync durability: an owner-accepted vote is on the standby at once" \
  || bad "sync durability: http=$code standby-has-vote=$onstandby"

# 5. Convergence: the full owner tally reaches the replica.
ot=$(tally_owner)
for i in $(seq 1 20); do rt=$(psql_node 2 -c "SELECT count(*) FROM votes WHERE proposal_id=$PID;"); [ "$rt" = "$ot" ] && break; sleep 1; done
[ "$rt" = "$ot" ] && ok "convergence: replica tally ($rt) matches the owner ($ot)" \
  || bad "convergence: replica $rt vs owner $ot"

echo
echo "── functional: $pass passed, $fail failed ──"
[ "$fail" -eq 0 ]
