#!/usr/bin/env bash
# Generate an internal CA + etcd server cert + one client cert per node for the
# federation control plane (etcd), using only openssl.
#
# WHY an internal CA for etcd (but Let's Encrypt for everything else): the app's
# etcd client natively accepts a custom CA (DEMOCRATOS_ETCD_CA) and a client cert
# for mutual TLS (DEMOCRATOS_ETCD_CERT/KEY), so a private CA is the intended,
# well-supported path for the control plane. The peer-feed and S3/MinIO clients
# (reqwest/rust-s3, rustls + webpki roots) trust ONLY public CAs, so those links
# get real Let's Encrypt certs via Caddy — they cannot use this CA.
#
# Run this ONCE, on a trusted machine. Distribute the outputs as noted at the end.
#
#   ./gen-etcd-certs.sh
#
# Requires: openssl. Output goes to ./etcd-certs/.
set -euo pipefail

# Names/IPs the etcd SERVER cert must be valid for. node1 (same host) and node2
# (across the LAN) both connect to it as https://etcd.ratbum.com:2379.
ETCD_DNS="etcd.ratbum.com"
ETCD_HOST_IP="192.168.2.5"          # the box etcd runs on
DAYS=3650

OUT="$(cd "$(dirname "$0")" && pwd)/etcd-certs"
mkdir -p "$OUT"
cd "$OUT"

echo "==> internal CA"
openssl genrsa -out ca-key.pem 4096
openssl req -x509 -new -nodes -key ca-key.pem -sha256 -days "$DAYS" \
  -subj "/CN=democratos-etcd-ca" -out ca.pem

gen_leaf () { # $1=name  $2=extfile
  openssl genrsa -out "$1-key.pem" 4096
  openssl req -new -key "$1-key.pem" -subj "/CN=$1" -out "$1.csr"
  openssl x509 -req -in "$1.csr" -CA ca.pem -CAkey ca-key.pem -CAcreateserial \
    -out "$1.pem" -days "$DAYS" -sha256 -extfile "$2"
  rm -f "$1.csr"
}

echo "==> etcd server cert (server + client auth, SAN pinned)"
cat > etcd-server.ext <<EOF
subjectAltName = DNS:${ETCD_DNS},DNS:etcd,DNS:localhost,IP:${ETCD_HOST_IP},IP:127.0.0.1
extendedKeyUsage = serverAuth,clientAuth
EOF
gen_leaf etcd-server etcd-server.ext

echo "==> per-node client certs (mTLS: only these can talk to etcd)"
cat > client.ext <<EOF
extendedKeyUsage = clientAuth
EOF
gen_leaf node1-client client.ext
gen_leaf node2-client client.ext

rm -f ca.srl *.ext
chmod 600 *-key.pem

cat <<EOF

Done. Files in $OUT

Distribute:
  HOST .5  (etcd server + node1):
    ca.pem            -> mounted at /certs/etcd-ca.pem   (DEMOCRATOS_ETCD_CA)
    etcd-server.pem   -> mounted into the etcd service
    etcd-server-key.pem
    node1-client.pem      -> /certs/etcd-client.pem      (DEMOCRATOS_ETCD_CERT)
    node1-client-key.pem  -> /certs/etcd-client-key.pem  (DEMOCRATOS_ETCD_KEY)

  HOST .4  (node2):
    ca.pem            -> /certs/etcd-ca.pem
    node2-client.pem      -> /certs/etcd-client.pem
    node2-client-key.pem  -> /certs/etcd-client-key.pem

Keep every *-key.pem secret. ca-key.pem never leaves this machine.
EOF
