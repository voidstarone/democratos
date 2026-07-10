#!/usr/bin/env bash
# One-command stress run against a live cluster: seed a large electorate, wait
# for it to replicate, drive a concurrent vote storm across every node, verify
# the authoritative tally + replica convergence, then a read storm. Assumes the
# cluster is up (./up.sh). Tunables via env:
#   VOTERS (default 1000)  CONCURRENCY (default 64)  READS (default 5000)
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

VOTERS="${VOTERS:-1000}"; CONCURRENCY="${CONCURRENCY:-64}"; READS="${READS:-5000}"
M="$RUN/stress-manifest.json"

# Comma-joined web URLs for every node.
urls=""; for i in $(seq 1 "$NODES"); do urls="$urls,$(web_url "$i")"; done; urls="${urls#,}"

echo "── seeding $VOTERS voters on node 1 (owner) ──"
"$LOADGEN" seed --owner-db "$(owner_db 1)" --node-id 1 --voters "$VOTERS" --slug stress --out "$M"
DEMOS=$(python3 -c "import json;print(json.load(open('$M'))['demos_id'])")

echo "── waiting for the electorate to replicate + a standby to be designated ──"
for i in $(seq 1 40); do
  m2=$(psql_node 2 -c "SELECT count(*) FROM memberships WHERE demos_id=$DEMOS;")
  st=$(docker exec democratos-etcd-test etcdctl --endpoints=http://127.0.0.1:2379 get "democratos/owners/$DEMOS/standbys" --print-value-only 2>/dev/null)
  echo "  node2 members=${m2:-0}/$VOTERS standby=${st:-<none>}"
  { [ "${m2:-0}" -ge "$VOTERS" ] && [ -n "$st" ]; } && break
  sleep 2
done

echo
echo "── VOTE STORM: $VOTERS voters across $NODES node(s), concurrency $CONCURRENCY ──"
"$LOADGEN" vote --manifest "$M" --nodes "$urls" --concurrency "$CONCURRENCY"

echo
echo "── VERIFY: authoritative tally + replica convergence ──"
"$LOADGEN" verify --manifest "$M" --owner-db "$(owner_db 1)" --replica-db "$(owner_db 2)"

echo
echo "── READ STORM: $READS GETs across $NODES node(s), concurrency $CONCURRENCY ──"
"$LOADGEN" read --nodes "$urls" --path / --requests "$READS" --concurrency "$CONCURRENCY"
