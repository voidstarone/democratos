#!/usr/bin/env bash
# Shared config + helpers for the Byzantine (majority-compromised) docker harness.
#
# Everything runs in Docker, orchestrated by the PLAIN `docker` CLI — no compose
# plugin, no host-side psql/curl/etcdctl. The host needs only a working Docker
# daemon (colima on macOS, native on Linux); all client tooling runs INSIDE
# containers via `docker exec` / ephemeral `docker run`, so the harness is
# platform-independent.
#
# Topology is parameterized: NODES total, of which the first HONEST are honest and
# the rest are attacker-controlled ("compromised"). Node 1 homes the honest
# community; the compromised majority + the `redteam` tool try to subvert it.
set -uo pipefail

# --- topology (override via env) --------------------------------------------
NODES="${NODES:-5}"          # total app nodes
HONEST="${HONEST:-2}"        # nodes 1..HONEST are honest; the rest are compromised
CLUSTER_TOKEN="${CLUSTER_TOKEN:-byzantine-cluster-token}"
# A shared, strong session secret so signed cookies verify across nodes (and so no
# node fails closed on an exposed 0.0.0.0 bind). 64 hex chars.
SESSION_SECRET="${SESSION_SECRET:-$(printf '9f%.0s' $(seq 1 32))}"

# --- image / network / container names --------------------------------------
IMAGE="${IMAGE:-democratos-byz}"
NET="${NET:-democratos-byz}"
PG="${PG:-byz-pg}"
ETCD="${ETCD:-byz-etcd}"
PG_USER=app; PG_PASS=pg
ETCD_IMAGE="${ETCD_IMAGE:-gcr.io/etcd-development/etcd:v3.5.16}"
PG_IMAGE="${PG_IMAGE:-postgres:16-alpine}"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

node_name() { echo "byz-node$1"; }
db_name()   { echo "democratos_n$1"; }
fed_url()   { echo "http://$(node_name "$1"):7400"; }   # in-network URL
web_url()   { echo "http://$(node_name "$1"):3000"; }
# Stable per-node signing seed (hex 32 bytes), derived from the node number.
node_seed() { printf '00000000000000000000000000000000000000000000000000000000%08x' "$((0xB0 + $1))"; }
is_honest() { [ "$1" -le "$HONEST" ]; }

# psql into the shared postgres container (no host psql needed).
psql_db() { local n="$1"; shift; docker exec -i "$PG" psql -U "$PG_USER" -d "$(db_name "$n")" -tA "$@"; }
psql_pg() { docker exec -i "$PG" psql -U "$PG_USER" -d postgres -v ON_ERROR_STOP=1 "$@"; }
# Run curl from inside a node container (nodes ship curl for their healthcheck), so
# in-network hostnames resolve and the host needs no curl.
dcurl()   { local from="$1"; shift; docker exec "$(node_name "$from")" curl "$@"; }
# Run the redteam tool as an ephemeral container on the cluster network.
redteam() { docker run --rm --network "$NET" -e REDTEAM_TOKEN="$CLUSTER_TOKEN" "$IMAGE" redteam --etcd "http://$ETCD:2379" "$@"; }

# Wait until a node's web port answers, using the node's own curl.
wait_node() { # node_num timeout_s
  local n="$1" timeout="${2:-60}" i=0
  until docker exec "$(node_name "$n")" curl -fsS -o /dev/null "http://127.0.0.1:3000/" 2>/dev/null; do
    i=$((i+1)); [ "$i" -ge $((timeout*2)) ] && { echo "timeout waiting for node $n" >&2; return 1; }
    sleep 0.5
  done
}

# Poll a shell condition (command) until it succeeds or times out. Portable
# replacement for `timeout` (which is not present on stock macOS).
poll() { # timeout_s  cmd...
  local timeout="$1"; shift; local i=0
  until "$@"; do i=$((i+1)); [ "$i" -ge $((timeout*2)) ] && return 1; sleep 0.5; done
}
