#!/usr/bin/env bash
# Shared config + helpers for the local federation test harness.
#
# This harness runs a REAL federated cluster on your machine as host processes —
# N `democratos` nodes, each with its own Postgres database, all sharing one etcd
# (control plane) and one MinIO (media). It reuses the containers the Rust test
# suite already uses, so there is nothing new to stand up. The same driver
# (`loadgen`) and checks work unchanged against the docker-compose cluster in
# ../docker-compose.federation.yml — just point --nodes / --owner-db at it.

set -euo pipefail

# --- topology / endpoints (override via env) --------------------------------
NODES="${NODES:-2}"                                   # number of app nodes
PG_CONTAINER="${PG_CONTAINER:-democratos-pg-test}"    # postgres container name
PG_HOST="${PG_HOST:-127.0.0.1}"; PG_PORT="${PG_PORT:-55432}"
PG_USER="${PG_USER:-app}"; PG_PASS="${PG_PASS:-pg}"
ETCD="${ETCD:-http://127.0.0.1:52379}"
S3_ENDPOINT="${S3_ENDPOINT:-http://127.0.0.1:59000}"
S3_KEY="${S3_KEY:-minioadmin}"; S3_SECRET="${S3_SECRET:-minioadmin}"
CLUSTER_TOKEN="${CLUSTER_TOKEN:-loadtest-token}"

WEB_BASE_PORT="${WEB_BASE_PORT:-3000}"   # node i web:  WEB_BASE_PORT + i
FED_BASE_PORT="${FED_BASE_PORT:-7400}"   # node i feed: FED_BASE_PORT + i
# All nodes share ONE test Postgres (max_connections≈100), so keep the SUM of
# per-node pools well under that, leaving headroom for loadgen. Real deployments
# give each node its own database and can size pools far higher.
DB_POOL="${DB_POOL:-$(( 80 / NODES ))}"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BIN="${BIN:-$ROOT/target/debug/democratos}"
LOADGEN="${LOADGEN:-$ROOT/target/debug/loadgen}"
RUN="${RUN:-$ROOT/deploy/test/.run}"     # pids, logs, manifest live here

db_name()   { echo "democratos_ln$1"; }
web_url()   { echo "http://127.0.0.1:$((WEB_BASE_PORT + $1))"; }
fed_url()   { echo "http://127.0.0.1:$((FED_BASE_PORT + $1))"; }
owner_db()  { echo "postgres://$PG_USER:$PG_PASS@$PG_HOST:$PG_PORT/$(db_name "$1")"; }
# A fixed per-node signing seed (stable identity across restarts within a run).
node_seed() { printf '%064x' "$((0xA5A5A5A5 + $1))"; }

psql_admin() { docker exec -i "$PG_CONTAINER" psql -U "$PG_USER" -d postgres -v ON_ERROR_STOP=1 "$@"; }
psql_node()  { local n="$1"; shift; docker exec -i "$PG_CONTAINER" psql -U "$PG_USER" -d "$(db_name "$n")" -tA "$@"; }

wait_http() { # url, timeout_s
  local url="$1" timeout="${2:-30}" i=0
  until curl -fsS -o /dev/null "$url" 2>/dev/null; do
    i=$((i+1)); [ "$i" -ge $((timeout*2)) ] && { echo "timeout waiting for $url" >&2; return 1; }
    sleep 0.5
  done
}

require_containers() {
  for c in "$PG_CONTAINER"; do
    docker inspect "$c" >/dev/null 2>&1 || { echo "missing container '$c' — start the test containers first (see README)" >&2; exit 1; }
  done
  curl -fsS -o /dev/null "$ETCD/version" 2>/dev/null || { echo "etcd not reachable at $ETCD" >&2; exit 1; }
  curl -fsS -o /dev/null "$S3_ENDPOINT/minio/health/live" 2>/dev/null || { echo "minio not reachable at $S3_ENDPOINT" >&2; exit 1; }
}
