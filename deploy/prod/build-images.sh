#!/usr/bin/env bash
# Build the two arm64 images the Raspberry Pis run, on a faster machine (your Mac),
# and pack them into a single tarball to ship. The Pis never compile anything.
#
# Works on an Apple-Silicon Mac whose Docker daemon is Linux/arm64 (e.g. Colima):
# a plain `docker build` there already targets linux/arm64 — the same arch the Pis
# need — so no buildx/QEMU is required. Verify with:  docker info | grep -i arch
#
#   ./build-images.sh              # build both images + write democratos-pi-images.tar.gz
#
# Then copy the tarball to each Pi and `docker load` it (see README "ship" step).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
OUT="$(cd "$(dirname "$0")" && pwd)/democratos-pi-images.tar.gz"

arch="$(docker info --format '{{.Architecture}}' 2>/dev/null || true)"
if [ "$arch" != "aarch64" ] && [ "$arch" != "arm64" ]; then
  echo "WARNING: docker daemon arch is '$arch', not arm64. The Pis are arm64." >&2
  echo "         Use an arm64 daemon (Colima on Apple Silicon), or build with" >&2
  echo "         'docker buildx build --platform linux/arm64 ... --load' instead." >&2
fi

echo "==> building democratos:latest (Rust; cold build is several minutes)"
docker build -t democratos:latest -f "$REPO_ROOT/Dockerfile" "$REPO_ROOT"

echo "==> building democratos-caddy:latest (Caddy + Cloudflare DNS plugin)"
docker build -t democratos-caddy:latest "$REPO_ROOT/deploy/prod/caddy"

echo "==> saving both images -> $OUT"
docker save democratos:latest democratos-caddy:latest | gzip > "$OUT"

echo
echo "Done. Ship it:"
echo "  scp $OUT  <you>@192.168.2.5:~/"
echo "  scp $OUT  <you>@192.168.2.4:~/"
echo "Then on each Pi:  gunzip -c ~/$(basename "$OUT") | docker load"
ls -lh "$OUT"
