#!/usr/bin/env bash
# Start an N-node federated cluster as host processes. Idempotent-ish: run
# down.sh first for a clean slate. Usage: NODES=3 ./up.sh
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

require_containers
[ -x "$BIN" ] || { echo "build first: cargo build -p democratos -p loadgen" >&2; exit 1; }
mkdir -p "$RUN"

echo "starting a $NODES-node federation (web $((WEB_BASE_PORT+1))…, feed $((FED_BASE_PORT+1))…)"

# Fresh per-node databases.
for i in $(seq 1 "$NODES"); do
  db="$(db_name "$i")"
  psql_admin -c "DROP DATABASE IF EXISTS $db WITH (FORCE);" >/dev/null
  psql_admin -c "CREATE DATABASE $db;" >/dev/null
  echo "  created database $db"
done

# Launch each node with every other node as a peer.
for i in $(seq 1 "$NODES"); do
  peers=()
  for j in $(seq 1 "$NODES"); do
    [ "$j" -ne "$i" ] && peers+=(--peer "$j=$(fed_url "$j")")
  done
  log="$RUN/node$i.log"
  DEMOCRATOS_NODE_SEED="$(node_seed "$i")" \
  AWS_ACCESS_KEY_ID="$S3_KEY" AWS_SECRET_ACCESS_KEY="$S3_SECRET" \
  "$BIN" \
    --store postgres --node-id "$i" --database-url "$(owner_db "$i")" \
    --db-pool-size "$DB_POOL" \
    --media s3 --s3-endpoint "$S3_ENDPOINT" --s3-bucket democratos-media --s3-path-style \
    --recommend-index "$RUN/rec$i.idx" \
    serve \
      --addr "127.0.0.1:$((WEB_BASE_PORT + i))" \
      --federation-addr "127.0.0.1:$((FED_BASE_PORT + i))" \
      --etcd-endpoints "$ETCD" \
      --cluster-token "$CLUSTER_TOKEN" \
      --dev \
      "${peers[@]}" \
    >"$log" 2>&1 &
  echo $! > "$RUN/node$i.pid"
  echo "  node $i pid $(cat "$RUN/node$i.pid") → $(web_url "$i")  (log: $log)"
done

# Wait until every node serves its web port.
for i in $(seq 1 "$NODES"); do
  wait_http "$(web_url "$i")/" 30 && echo "  node $i healthy" || { echo "node $i failed to come up; tail $RUN/node$i.log" >&2; exit 1; }
done

echo "cluster up. Try: $(web_url 1)/  and  $(web_url 2)/"
