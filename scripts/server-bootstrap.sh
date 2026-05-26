#!/usr/bin/env bash
# Run this ONCE on a fresh Ubuntu 22.04 / Debian 12 server.
# It installs Docker, clones the repo, and starts the stack.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/jk-baseer/gz/main/scripts/server-bootstrap.sh | bash
#
# Or after cloning:
#   bash scripts/server-bootstrap.sh
set -euo pipefail

REPO_DIR=${REPO_DIR:-/opt/gz}
COMPOSE="docker compose -f $REPO_DIR/docker/docker-compose.prod.yml"

# ── 1. Docker ─────────────────────────────────────────────────────────────────
if ! command -v docker &>/dev/null; then
  echo "Installing Docker..."
  curl -fsSL https://get.docker.com | sh
  usermod -aG docker "$USER"
  echo "Docker installed. You may need to log out and back in for group membership."
fi

# ── 2. Clone repo ─────────────────────────────────────────────────────────────
if [ ! -d "$REPO_DIR" ]; then
  echo "Cloning repo to $REPO_DIR ..."
  git clone https://github.com/jk-baseer/gz "$REPO_DIR"
else
  echo "Repo already at $REPO_DIR"
fi

# ── 3. Create .env ────────────────────────────────────────────────────────────
if [ ! -f "$REPO_DIR/docker/.env" ]; then
  echo ""
  echo "Creating docker/.env — fill in secrets before continuing."
  cp "$REPO_DIR/.env.example" "$REPO_DIR/docker/.env"
  echo ""
  echo "Edit $REPO_DIR/docker/.env then re-run this script (or continue manually)."
  echo "Required: POSTGRES_PASSWORD, REDIS_PASSWORD, JWT_SECRET, BID_DOMAIN, APP_DOMAIN"
  exit 0
fi

# ── 4. Start databases ─────────────────────────────────────────────────────────
echo "Starting postgres and redis..."
$COMPOSE up -d postgres redis clickhouse

echo "Waiting for postgres to be healthy..."
until docker inspect --format='{{.State.Health.Status}}' \
    "$(docker compose -f $REPO_DIR/docker/docker-compose.prod.yml ps -q postgres)" \
    2>/dev/null | grep -q healthy; do
  sleep 2
done

# ── 5. SSL certificates ────────────────────────────────────────────────────────
source "$REPO_DIR/docker/.env"
if [ ! -d "/etc/letsencrypt/live/${BID_DOMAIN:-_}" ]; then
  echo ""
  echo "Obtaining SSL certificates..."
  echo "Enter your email for Let's Encrypt notifications:"
  read -r CERT_EMAIL

  $COMPOSE up -d nginx   # nginx must be up for ACME challenge

  $COMPOSE run --rm certbot certonly \
    --webroot -w /var/www/certbot \
    -d "${BID_DOMAIN}" -d "${APP_DOMAIN}" \
    --email "$CERT_EMAIL" --agree-tos --no-eff-email
else
  echo "Certificates already present, skipping."
fi

# ── 6. Start everything ────────────────────────────────────────────────────────
echo "Starting all services..."
$COMPOSE up -d

echo ""
echo "Done. Services running:"
$COMPOSE ps

echo ""
echo "Bidder health:      $(curl -sf "http://localhost:8080/health" && echo OK || echo FAIL)"
echo "Campaign API health: $(curl -sf "http://localhost:8081/health" && echo OK || echo FAIL)"
