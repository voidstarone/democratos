#!/usr/bin/env bash
# Stop the host-process cluster and (optionally) drop its databases.
# Usage: ./down.sh [--keep-dbs]
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

keep_dbs=0; [ "${1:-}" = "--keep-dbs" ] && keep_dbs=1

if [ -d "$RUN" ]; then
  for pf in "$RUN"/node*.pid; do
    [ -e "$pf" ] || continue
    pid="$(cat "$pf")"
    if kill -0 "$pid" 2>/dev/null; then kill "$pid" 2>/dev/null && echo "stopped pid $pid"; fi
    rm -f "$pf"
  done
fi

if [ "$keep_dbs" -eq 0 ]; then
  for i in $(seq 1 "$NODES"); do
    db="$(db_name "$i")"
    psql_admin -c "DROP DATABASE IF EXISTS $db WITH (FORCE);" >/dev/null 2>&1 && echo "dropped $db" || true
  done
  # Wipe control-plane state too. Ownership/standby/node keys are lease-bound and
  # expire on their own, but the epoch counters persist by design — and since a
  # fresh run recreates the databases (so IDs restart), a stale epoch would fence
  # the reborn community. Clearing it keeps each run reproducible.
  docker exec democratos-etcd-test etcdctl --endpoints=http://127.0.0.1:2379 \
    del --prefix democratos/ >/dev/null 2>&1 && echo "wiped etcd control-plane state" || true
fi
echo "cluster down."
