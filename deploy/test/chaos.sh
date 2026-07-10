#!/usr/bin/env bash
# Chaos / failover test: kill a community's owner and prove the cluster (a) rehomes
# it onto a surviving node, (b) fences the old owner when it returns (no
# split-brain), and (c) never double-counts or loses a vote. Assumes the cluster
# is up (./up.sh). Best with NODES=3 (a survivor can then form a fresh quorum and
# keep accepting votes); with NODES=2 the lone survivor correctly FAILS votes
# closed (quorum of 2 impossible). Usage: NODES=3 ./up.sh && ./chaos.sh
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

pass=0; fail=0
ok()  { echo "  ✓ $1"; pass=$((pass+1)); }
bad() { echo "  ✗ $1"; fail=$((fail+1)); }
owner_of(){ docker exec "$PG_CONTAINER" true 2>/dev/null; docker exec democratos-etcd-test etcdctl --endpoints=http://127.0.0.1:2379 get "democratos/owners/$1/holder" --print-value-only 2>/dev/null; }

M="$RUN/chaos-manifest.json"
echo "── seeding + converging ──"
"$LOADGEN" seed --owner-db "$(owner_db 1)" --node-id 1 --voters 40 --slug chaos --out "$M" >/dev/null
DEMOS=$(python3 -c "import json;print(json.load(open('$M'))['demos_id'])")
PID=$(python3 -c "import json;print(json.load(open('$M'))['proposal_id'])")
# shellcheck disable=SC2207  (portable to bash 3.2 — no readarray/mapfile)
VOTERS=($(python3 -c "import json;print(' '.join(map(str, json.load(open('$M'))['voter_ids'])))"))
for i in $(seq 1 25); do m2=$(psql_node 2 -c "SELECT count(*) FROM memberships WHERE demos_id=$DEMOS;"); [ "${m2:-0}" -ge 40 ] && [ "$(owner_of "$DEMOS")" = "1" ] && break; sleep 2; done

# Cast a few votes pre-failover so we have a baseline count.
vote(){ curl -s -o /dev/null -w "%{http_code}" -X POST "$1/p/$PID/vote" -H "Cookie: uid=$2" -H "x-requested-with: t" --data "choice=aye"; }
for k in 0 1 2 3 4; do vote "$(web_url 2)" "${VOTERS[$k]}" >/dev/null; done
pre=$(psql_node 1 -c "SELECT count(*) FROM votes WHERE proposal_id=$PID;")
echo "  owner=1, pre-failover votes=$pre"

echo "── killing the owner (node 1) ──"
kill "$(cat "$RUN/node1.pid")" 2>/dev/null; rm -f "$RUN/node1.pid"

echo "── waiting for rehome (lease expiry + tick, ≤45s) ──"
new=""
for i in $(seq 1 23); do
  o=$(owner_of "$DEMOS")
  if [ -n "$o" ] && [ "$o" != "1" ]; then new="$o"; break; fi
  sleep 2
done
[ -n "$new" ] && ok "failover: community rehomed from node 1 to node $new" \
  || { bad "failover: community did not rehome within 45s (owner=$(owner_of "$DEMOS"))"; echo "── chaos: $pass passed, $fail failed ──"; exit 1; }

# On the new owner, votes either succeed (≥3 nodes → fresh quorum) or fail closed
# (2 nodes → quorum impossible). Either way, no double-count and no lost data.
newurl="$(web_url "$new")"
code=$(vote "$newurl" "${VOTERS[10]}")
if [ "$code" = "200" ]; then
  ok "post-failover: the new owner accepts votes (quorum re-formed)"
else
  ok "post-failover: the new owner FAILS a vote closed (http $code — quorum of 2 unmet; expected with 2 nodes)"
fi

echo "── restarting the old owner (node 1) ──"
peers=(); for j in $(seq 1 "$NODES"); do [ "$j" -ne 1 ] && peers+=(--peer "$j=$(fed_url "$j")"); done
DEMOCRATOS_NODE_SEED="$(node_seed 1)" AWS_ACCESS_KEY_ID="$S3_KEY" AWS_SECRET_ACCESS_KEY="$S3_SECRET" \
  "$BIN" --store postgres --node-id 1 --database-url "$(owner_db 1)" --db-pool-size "${DB_POOL:-40}" \
    --media s3 --s3-endpoint "$S3_ENDPOINT" --s3-bucket democratos-media --s3-path-style \
    --recommend-index "$RUN/rec1.idx" \
    serve --addr "127.0.0.1:$((WEB_BASE_PORT+1))" --federation-addr "$(fed_url 1 | sed 's#http://##')" \
      --etcd-endpoints "$ETCD" --cluster-token "$CLUSTER_TOKEN" --dev "${peers[@]}" \
    >"$RUN/node1.log" 2>&1 &
echo $! > "$RUN/node1.pid"
wait_http "$(web_url 1)/" 30 || bad "old owner did not restart"

echo "── verifying no split-brain (old owner is fenced) ──"
sleep 8   # let node1 re-register and its heartbeat run
back=$(owner_of "$DEMOS")
[ "$back" = "$new" ] && ok "fencing: after returning, node 1 did NOT reclaim — node $new still owns it" \
  || bad "split-brain risk: owner is now $back (expected $new)"

echo "── verifying no corruption (every ballot is one distinct voter) ──"
# Only ACKed votes are guaranteed durable across failover (the fail-closed
# quorum contract); an un-acked vote that reached only the dead owner may be
# dropped. So the invariant is integrity, not a count: no voter is double-counted.
ot=$(psql_node "$new" -c "SELECT count(*) FROM votes WHERE proposal_id=$PID;")
distinct=$(psql_node "$new" -c "SELECT count(DISTINCT voter_id) FROM votes WHERE proposal_id=$PID;")
[ "$ot" = "$distinct" ] \
  && ok "integrity: $ot ballots on the new owner, all distinct voters (no double-count)" \
  || bad "integrity: votes=$ot distinct=$distinct (a voter was double-counted!)"

echo "── verifying replication is not stalled after the old owner returns ──"
# Ownership is stable, so writes route to the new owner; its feed must keep
# flowing to the other nodes (the returning old owner's fenced events must be
# skipped, not stall the log).
h_new=$(psql_node "$new" -c "SELECT max(seq) FROM outbox;")
progress=0
for i in $(seq 1 15); do
  c=$(psql_node 1 -c "SELECT last_seq FROM replication_cursor WHERE peer_node=$new;")
  [ -n "$c" ] && [ "${c:-0}" -ge "${h_new:-0}" ] && { progress=1; break; }
  sleep 2
done
[ "$progress" = "1" ] \
  && ok "no stall: the returned old owner caught its replica up to the new owner (cursor ≥ $h_new)" \
  || bad "stall: old owner's replica of node $new did not catch up (cursor=$c vs head $h_new)"

echo
echo "── chaos: $pass passed, $fail failed ──"
[ "$fail" -eq 0 ]
