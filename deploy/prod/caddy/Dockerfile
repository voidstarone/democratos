# Caddy with the Cloudflare DNS plugin.
#
# The stock caddy:2 image can only solve ACME HTTP-01 / TLS-ALPN challenges,
# which need inbound :80/:443 on the challenged name. Our internal fabric names
# (n1/n2/minio.ratbum.com) resolve to PRIVATE LAN IPs and are never reachable
# from the internet, so we solve DNS-01 instead — Caddy writes a TXT record via
# the Cloudflare API and Let's Encrypt verifies it. That needs this plugin.
FROM caddy:2-builder AS builder
RUN xcaddy build --with github.com/caddy-dns/cloudflare

FROM caddy:2
COPY --from=builder /usr/bin/caddy /usr/bin/caddy
