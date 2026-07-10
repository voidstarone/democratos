#!/usr/bin/env bash
# Build the harness image (node + loadgen + redteam in one image). Self-contained
# multi-stage build — the host needs only Docker; compiles the same on colima/arm64
# and native Linux. First build is slow (compiles the workspace in-container);
# rebuilds are layer-cached. Usage: ./build.sh
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

echo "building $IMAGE from $ROOT/deploy/byzantine/Dockerfile (this compiles the workspace in-container; first run is slow)…"
docker build -f "$ROOT/deploy/byzantine/Dockerfile" -t "$IMAGE" "$ROOT"
echo "built $IMAGE"
