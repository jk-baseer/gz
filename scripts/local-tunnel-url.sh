#!/usr/bin/env bash
# Watches cloudflared logs and prints the tunnel URL as soon as it's ready.
# Run in a second terminal after: docker compose -f docker/docker-compose.local.yml up
set -euo pipefail

echo "Waiting for Cloudflare tunnel URL..."

docker compose -f docker/docker-compose.local.yml logs -f cloudflared 2>&1 | \
  grep --line-buffered -oP 'https://[a-z0-9\-]+\.trycloudflare\.com' | \
  head -1 | tee /dev/tty | xargs -I{} echo "
╔══════════════════════════════════════════════════════════╗
║  Tunnel is live!                                         ║
║                                                          ║
║  Bidder URL (give to exchange):                          ║
║  {}/bid/<exchange_slug>              ║
║                                                          ║
║  Win notice URL:                                         ║
║  {}/win?...                          ║
║                                                          ║
║  Dashboard: http://localhost:8081                        ║
╚══════════════════════════════════════════════════════════╝"
