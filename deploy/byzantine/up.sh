#!/usr/bin/env bash
# Bring up a Byzantine cluster with plain `docker` (no compose plugin):
#   * shared postgres (one DB per node) + single-node etcd control plane
#   * NODES real democratos nodes; nodes 1..HONEST honest, the rest compromised
#     (attacker-controlled — same real binary, subverted via etcd + the redteam tool)
#   * one extra rogue node (id NODES+1) running `redteam serve-rogue`: a malicious
#     peer that serves a forged feed for the honest community
#
# The honest community is seeded into node 1 BEFORE node 1 boots, so node 1 claims
# it and mints a founder-signed home binding at startup — giving us a genuinely
# founder-bound community for the majority-compromised assertions.
#
# Usage:  NODES=5 HONEST=2 ./up.sh        (run ./build.sh first)
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

docker image inspect "$IMAGE" >/dev/null 2>&1 || { echo "image $IMAGE missing — run ./build.sh first" >&2; exit 1; }
ROGUE_ID=$((NODES + 1))
VOTERS="${VOTERS:-20}"
DB_POOL=$(( 72 / (NODES + 1) )); [ "$DB_POOL" -lt 4 ] && DB_POOL=4

echo "── network ──"
docker network inspect "$NET" >/dev/null 2>&1 || docker network create "$NET" >/dev/null
echo "  net $NET"

echo "── control plane + storage ──"
docker rm -f "$ETCD" "$PG" >/dev/null 2>&1 || true
docker run -d --name "$ETCD" --network "$NET" "$ETCD_IMAGE" \
  /usr/local/bin/etcd --name byz --data-dir /tmp/etcd \
    --listen-client-urls http://0.0.0.0:2379 \
    --advertise-client-urls "http://$ETCD:2379" \
    --listen-peer-urls http://0.0.0.0:2380 >/dev/null
docker run -d --name "$PG" --network "$NET" \
  -e POSTGRES_USER="$PG_USER" -e POSTGRES_PASSWORD="$PG_PASS" -e POSTGRES_DB=postgres \
  "$PG_IMAGE" >/dev/null
echo "  waiting for postgres + etcd…"
poll 60 docker exec "$PG"   pg_isready -U "$PG_USER" -q            || { echo "postgres not ready" >&2; exit 1; }
poll 60 docker exec "$ETCD" etcdctl --endpoints=http://127.0.0.1:2379 endpoint health >/dev/null 2>&1 \
                                                                     || { echo "etcd not healthy" >&2; exit 1; }

echo "── per-node databases ──"
for i in $(seq 1 "$NODES"); do
  docker exec -i "$PG" psql -U "$PG_USER" -d postgres -v ON_ERROR_STOP=1 \
    -c "DROP DATABASE IF EXISTS $(db_name "$i") WITH (FORCE);" -c "CREATE DATABASE $(db_name "$i");" >/dev/null
  echo "  $(db_name "$i")"
done

echo "── seed the honest community into node 1 (before it boots, so it binds it) ──"
docker run --rm --network "$NET" -e DEMOCRATOS_ALLOW_INSECURE_DB=1 "$IMAGE" loadgen seed \
  --owner-db "postgres://$PG_USER:$PG_PASS@$PG:5432/$(db_name 1)" \
  --node-id 1 --voters "$VOTERS" --slug honest --out /tmp/seed.json >/dev/null
# loadgen suffixes the slug (e.g. `honest-1`); read back whatever it created.
DEMOS=$(psql_db 1 -c "SELECT id FROM demoi ORDER BY id LIMIT 1;")
SLUG=$(psql_db 1 -c "SELECT slug FROM demoi WHERE id=$DEMOS;")
[ -n "${DEMOS:-}" ] || { echo "seed failed: no honest community" >&2; exit 1; }
echo "  honest community d/$DEMOS (slug '$SLUG') with $VOTERS voters"

# Boot one democratos node. $1=node id, honest and compromised nodes are identical
# real binaries (the "compromise" is attacker control, exercised by redteam).
boot_node() { # node_id
  local i="$1" peers=() j
  for j in $(seq 1 "$NODES"); do [ "$j" -ne "$i" ] && peers+=(--peer "$j=$(fed_url "$j")"); done
  peers+=(--peer "$ROGUE_ID=http://byz-rogue:7400")   # every node also peers the rogue
  docker run -d --name "$(node_name "$i")" --network "$NET" \
    -e DEMOCRATOS_NODE_SEED="$(node_seed "$i")" \
    -e DEMOCRATOS_SESSION_SECRET="$SESSION_SECRET" \
    -e DATABASE_URL="postgres://$PG_USER:$PG_PASS@$PG:5432/$(db_name "$i")" \
    -e DEMOCRATOS_ALLOW_INSECURE_DB=1 -e DEMOCRATOS_ALLOW_PLAINTEXT_FEDERATION=1 \
    -e DEMOCRATOS_MEDIA=local -e DEMOCRATOS_MEDIA_DIR=/data/media \
    "$IMAGE" democratos \
      --store postgres --node-id "$i" --db-pool-size "$DB_POOL" --media local \
      serve --addr 0.0.0.0:3000 --federation-addr 0.0.0.0:7400 \
        --etcd-endpoints "http://$ETCD:2379" --cluster-token "$CLUSTER_TOKEN" \
        "${peers[@]}" >/dev/null
}

echo "── rogue malicious peer (node $ROGUE_ID) ──"
docker rm -f byz-rogue >/dev/null 2>&1 || true
docker run -d --name byz-rogue --network "$NET" -e REDTEAM_TOKEN="$CLUSTER_TOKEN" \
  "$IMAGE" redteam --etcd "http://$ETCD:2379" --node "$ROGUE_ID" \
    serve-rogue --demos "$DEMOS" --bind 0.0.0.0:7400 >/dev/null
echo "  byz-rogue serving a forged feed for d/$DEMOS"

echo "── $NODES nodes (1..$HONEST honest, $((HONEST+1))..$NODES compromised) ──"
for i in $(seq 1 "$NODES"); do
  boot_node "$i"
  role="honest"; is_honest "$i" || role="COMPROMISED"
  echo "  node $i ($role) → $(node_name "$i")"
done

echo "── waiting for nodes to serve ──"
for i in $(seq 1 "$NODES"); do
  wait_node "$i" 90 && echo "  node $i healthy" || { echo "node $i failed; docker logs $(node_name "$i")" >&2; exit 1; }
done

echo "── waiting for node 1 to own + bind d/$DEMOS ──"
# Probe from a COMPROMISED node id: once node 1 has published the community key and
# claimed the lease, an attacker's seize attempt is BLOCKED — our readiness signal.
owns_demos() { redteam --node $((HONEST + 1)) --seed "$(node_seed $((HONEST + 1)))" seize-bound --demos "$DEMOS" 2>/dev/null | grep -q 'OUTCOME=BLOCKED'; }
if poll 40 owns_demos; then echo "  d/$DEMOS is honestly owned + founder-bound"; else
  echo "  ⚠ d/$DEMOS ownership/binding not confirmed yet (scenarios will re-check)"; fi

# Record what the scenario runner needs.
mkdir -p "$ROOT/deploy/byzantine/.run"
{ echo "DEMOS=$DEMOS"; echo "SLUG=$SLUG"; echo "NODES=$NODES"; echo "HONEST=$HONEST"; echo "ROGUE_ID=$ROGUE_ID"; } > "$ROOT/deploy/byzantine/.run/env"
echo
echo "cluster up (NODES=$NODES HONEST=$HONEST). Honest community d/$DEMOS. Run: ./byzantine.sh"
