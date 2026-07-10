#!/usr/bin/env bash
# Tear the Byzantine cluster down: stop/remove every container and the network.
# Usage:  NODES=5 ./down.sh   (NODES only needs to cover the largest run you started)
set -uo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

names=("$PG" "$ETCD" byz-rogue)
for i in $(seq 1 "$((NODES + 2))"); do names+=("$(node_name "$i")"); done   # +slack for rogue ids
echo "removing containers…"
docker rm -f "${names[@]}" >/dev/null 2>&1 || true
docker network rm "$NET" >/dev/null 2>&1 || true
rm -f "$ROOT/deploy/byzantine/.run/env" 2>/dev/null || true
echo "cluster down."
