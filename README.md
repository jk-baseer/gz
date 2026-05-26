# GZ Ads — Rust-Native Demand-Side Platform

A fully Rust-native DSP built for programmatic advertising in the **GCC/MENA region**. It connects directly to SSPs via OpenRTB 2.6, serves banner, native, and video ads, and gives advertisers a self-serve dashboard to run and measure campaigns.

---

## Table of Contents

1. [What It Does](#what-it-does)
2. [Architecture](#architecture)
3. [Services](#services)
4. [Data Flow](#data-flow)
5. [Database Schema](#database-schema)
6. [Redis Key Space](#redis-key-space)
7. [API Reference](#api-reference)
8. [Running Locally](#running-locally)
9. [Production Deployment](#production-deployment)
10. [Environment Variables](#environment-variables)
11. [Exchange Integration](#exchange-integration)
12. [Ad Formats](#ad-formats)
13. [Targeting Options](#targeting-options)

---

## What It Does

| Capability | Detail |
|---|---|
| **OpenRTB 2.6 bidding** | Accepts bid requests from exchanges, evaluates campaigns in memory, returns bid in <5ms |
| **Budget enforcement** | Atomic Lua scripts in Redis; never overspends even under concurrent load |
| **Pacing** | Smooths daily budget evenly across remaining minutes; bidder checks per-minute cap |
| **Frequency capping** | Per-user per-campaign per-day limit, enforced atomically in Redis |
| **Ad formats** | Banner (any size), Native (OpenRTB 1.2 JSON), Video (VAST 4.0 XML) |
| **Conversion tracking** | 1×1 GIF pixel; records order value in cents |
| **Self-serve dashboard** | Alpine.js + Tailwind UI served by the API; no build step |
| **Stripe billing** | Checkout Session → webhook → wallet top-up |
| **Analytics** | PostgreSQL → ClickHouse sync every 60s via event-consumer |
| **JWT auth** | Argon2 password hashing, HS256 JWT, Bearer + cookie support |
| **Admin approval** | Creatives require admin approval before going live |

---

## Architecture

```
                        Exchanges (ArabyAds, InMobi, Smaato, …)
                               │  OpenRTB 2.6
                               ▼
                ┌──────────────────────────┐
                │         Bidder           │  :8080
                │  • Evaluates bid reqs    │
                │  • In-memory campaign    │
                │    index (30s refresh)   │
                │  • Budget / freq checks  │
                │  • Win notice handler    │
                │  • Imp / click / conv    │
                │    tracking pixels       │
                │  • VAST XML endpoint     │
                └────────┬─────────────────┘
                         │                  ▲ pace_cap:{id} (read)
                         │                  │
              ┌──────────▼──────────┐  ┌────┴────────────┐
              │      PostgreSQL      │  │     Redis        │
              │  campaigns, events   │  │  budgets, freq,  │
              │  advertisers, etc.   │  │  pending_imp,    │
              └──────────┬──────────┘  │  pace caps       │
                         │             └────────┬──────────┘
              ┌──────────▼──────────┐           │ pace_cap:{id} (write)
              │   event-consumer    │  ┌────────▼──────────┐
              │  PG → ClickHouse    │  │     Pacing         │
              │  every 60s          │  │  Updates per-min   │
              └──────────┬──────────┘  │  spend caps        │
                         │             └────────────────────┘
              ┌──────────▼──────────┐
              │     ClickHouse       │  Analytics store
              │  impression_events   │  (MergeTree, 2yr TTL)
              │  click_events        │
              │  conversion_events   │
              └─────────────────────┘

                ┌──────────────────────────┐
                │      Campaign API         │  :8081
                │  • REST CRUD             │
                │  • Advertiser auth       │
                │  • Exchange registry     │
                │  • Reporting             │
                │  • Stripe billing        │
                │  • Dashboard UI (HTML)   │
                └──────────────────────────┘
                         ▲
                         │  HTTPS (app.gz-ads.com)
                    Advertiser's browser
```

---

## Services

### `bidder` — The hot path

Receives OpenRTB bid requests from exchanges. For each incoming auction:

1. Loads the in-memory campaign index (refreshed from PostgreSQL every 30s)
2. Filters campaigns by targeting (geo, device, OS, IAB category, language, daypart, size, bid floor)
3. Picks the highest-CPM matching campaign
4. Checks pacing cap (`pace_cap:{campaign_id}` in Redis)
5. Checks frequency cap (`freq:{campaign_id}:{user_id}:{date}` in Redis via Lua script)
6. Atomically reserves budget in Redis (`spend:daily:{id}:{date}` + `spend:total:{id}`) via Lua script
7. Stores a `PendingImpression` in Redis (TTL 1h) keyed by bid_id
8. Returns an OpenRTB `BidResponse` with win URL and ad markup

On **win notice** (`GET /win`):
- Retrieves and deletes the `PendingImpression` from Redis
- Adjusts Redis spend to the actual clearing price
- Inserts `impression_events` row into PostgreSQL
- Atomically updates `campaigns.spend_total_cents` and `advertisers.wallet_balance_cents` via a CTE UPDATE

**Why in-memory index?** Exchange latency budgets are 100–150ms. A Redis or DB lookup per auction would add 2–5ms per campaign candidate. With an in-memory `Arc<RwLock<Vec<CampaignRecord>>>` the targeting loop runs in microseconds.

**Why Lua scripts for budget?** `GET` then `INCRBY` is a TOCTOU race under high concurrency. A Lua script executes atomically on the Redis server — no locking needed from the Rust side.

### `campaign-api` — The management plane

REST API + dashboard UI for advertisers. Handles:
- JWT auth (register/login/logout) with Argon2 password hashing
- Advertiser CRUD and wallet top-up
- Campaign lifecycle (draft → active → paused → ended)
- Targeting configuration (upsert per campaign)
- Creative management with admin approve/reject workflow
- Campaign performance reporting
- Exchange registry (configure SSP connections)
- Stripe Checkout + webhook for wallet funding
- Serves `dashboard.html` at `/` — Alpine.js + Tailwind, no build step

### `pacing` — Budget smoother

Runs every 60 seconds. For each active campaign with a daily budget:
```
remaining_budget = daily_budget - spent_today_in_redis
remaining_minutes = minutes left in UTC day
pace_cap = remaining_budget / remaining_minutes
```
Writes `pace_cap:{campaign_id}` to Redis with a 120s TTL. The bidder reads this before bidding — if the cap is 0, it skips the campaign for that minute.

### `event-consumer` — Analytics sync

Runs every 60 seconds. Maintains a cursor per event type in `ch_sync_cursors`. On each tick it:
1. Reads new rows from `impression_events / click_events / conversion_events` since the last cursor
2. POSTs them to ClickHouse via HTTP JSONEachRow
3. Advances the cursor

PostgreSQL is always the source of truth. ClickHouse is eventually consistent for analytics.

### `openrtb` — Shared types

Serde-annotated structs for the full OpenRTB 2.6 BidRequest/BidResponse hierarchy including Banner, Native, Video, Site, App, Device, User, Geo. Used by both bidder and any future reporting that needs to decode stored requests.

### `common` — Shared utilities

Config loading (env-vars via `dotenvy` + `config` crate), structured JSON logging setup via `tracing-subscriber`.

---

## Data Flow

### Bid lifecycle

```
Exchange                 Bidder                      Redis              PostgreSQL
   │                       │                           │                    │
   │  POST /bid/arabyads   │                           │                    │
   │──────────────────────►│                           │                    │
   │                       │ get_all() from index      │                    │
   │                       │ [in-memory, ~0ms]         │                    │
   │                       │                           │                    │
   │                       │ pace_cap:{id}?            │                    │
   │                       │──────────────────────────►│                    │
   │                       │◄──────────────────────────│                    │
   │                       │                           │                    │
   │                       │ freq Lua check+incr        │                    │
   │                       │──────────────────────────►│                    │
   │                       │◄──────────────────────────│                    │
   │                       │                           │                    │
   │                       │ budget Lua check+reserve   │                    │
   │                       │──────────────────────────►│                    │
   │                       │◄──────────────────────────│                    │
   │                       │                           │                    │
   │                       │ SET pending_imp:{bid_id}  │                    │
   │                       │──────────────────────────►│                    │
   │                       │                           │                    │
   │  200 BidResponse      │                           │                    │
   │◄──────────────────────│                           │                    │
   │                       │                           │                    │
   │  GET /win?price=1.80  │                           │                    │
   │──────────────────────►│                           │                    │
   │                       │ GETDEL pending_imp:…      │                    │
   │                       │──────────────────────────►│                    │
   │                       │◄──────────────────────────│                    │
   │                       │ record_win Lua (adjust δ) │                    │
   │                       │──────────────────────────►│                    │
   │                       │                           │                    │
   │                       │                           │ INSERT impression  │
   │                       │                           │ UPDATE campaign    │
   │                       │                           │ UPDATE advertiser  │
   │                       │                           │───────────────────►│
   │  200 OK               │                           │                    │
   │◄──────────────────────│                           │                    │
```

### Impression and conversion pixels

```
User's browser
   │
   │  GET /imp/{token}        → marks viewed_at on impression_events → returns 1×1 GIF
   │  GET /click/{token}      → inserts click_events → 302 to advertiser's click_url
   │  GET /conv/{campaign_id} → inserts conversion_events (value=query param) → 1×1 GIF
   │  GET /vast/{token}       → returns VAST 4.0 XML for video creatives
```

---

## Database Schema

### PostgreSQL (source of truth)

```
advertisers
  id                  UUID  PK
  email               TEXT  UNIQUE
  company_name        TEXT
  wallet_balance_cents BIGINT  CHECK >= 0
  password_hash       TEXT    (Argon2id)
  email_verified      BOOL
  created_at / updated_at

campaigns
  id                   UUID  PK
  advertiser_id        UUID  FK → advertisers
  name                 TEXT
  status               TEXT  (draft | active | paused | ended)
  bid_price_cpm_cents  BIGINT
  budget_total_cents   BIGINT
  budget_daily_cents   BIGINT  nullable
  spend_total_cents    BIGINT  default 0
  frequency_cap_daily  INT     nullable
  start_date / end_date  TIMESTAMPTZ  nullable

campaign_targeting        (1:1 with campaigns)
  campaign_id        UUID  PK FK
  geo_countries      TEXT[]   ISO-3166-1-alpha-3
  device_types       TEXT[]   mobile | desktop | tablet
  os_types           TEXT[]   ios | android | windows | macos
  site_categories    TEXT[]   IAB taxonomy codes
  languages          TEXT[]   BCP-47
  hours_of_day       INT[]    0–23 UTC (empty = all hours)
  days_of_week       INT[]    0=Sun…6=Sat (empty = all days)

creatives
  id           UUID  PK
  campaign_id  UUID  FK
  format       TEXT  (banner | native | video)
  width/height INT   nullable (banner)
  asset_url    TEXT  CDN URL
  click_url    TEXT
  status       TEXT  (pending_review | approved | rejected)
  title_text   TEXT  native only
  description  TEXT  native only
  cta_text     TEXT  native only
  sponsored_by TEXT  native only

exchange_configs
  id              UUID  PK
  slug            TEXT  UNIQUE  (used in /bid/:slug URL)
  name            TEXT
  status          TEXT  (active | paused | testing)
  endpoint_url    TEXT
  win_price_macro TEXT  default '${AUCTION_PRICE}'
  min_floor_cents INT

impression_events       (append-only)
  id                   UUID  PK
  campaign_id          UUID
  creative_id          UUID
  exchange             TEXT
  auction_id           TEXT
  bid_price_cents      BIGINT
  clearing_price_cents BIGINT
  geo_country          TEXT
  device_type          TEXT
  os                   TEXT
  site_domain          TEXT
  created_at           TIMESTAMPTZ
  viewed_at            TIMESTAMPTZ  (set by /imp pixel)

click_events            (append-only)
  id / campaign_id / creative_id / auction_id / exchange / created_at

conversion_events       (append-only)
  id / campaign_id / value_cents / geo_country / created_at

ch_sync_cursors
  topic           TEXT  PK  (impressions | clicks | conversions)
  last_synced_at  TIMESTAMPTZ
```

### ClickHouse (analytics)

Three MergeTree tables mirror the PostgreSQL event tables. Partitioned by `toYYYYMM(created_at)`, ordered by `(campaign_id, created_at)`, 2-year TTL. Used for heavy aggregation queries at scale (the PostgreSQL tables are fine for daily reporting volumes).

---

## Redis Key Space

| Key | Type | TTL | Purpose |
|---|---|---|---|
| `spend:daily:{campaign_id}:{YYYY-MM-DD}` | String (int) | 24h | Daily spend counter in cents |
| `spend:total:{campaign_id}` | String (int) | none | Lifetime spend counter |
| `pace_cap:{campaign_id}` | String (int) | 120s | Per-minute bid allowance in cents; 0 = no more bids this minute |
| `freq:{campaign_id}:{user_id}:{YYYY-MM-DD}` | String (int) | 25h | Daily impression count per user |
| `pending_imp:{bid_id}` | String (JSON) | 1h | Full bid context for win-notice reconciliation |

All budget and frequency operations use Lua scripts for atomicity.

---

## API Reference

### Auth

```
POST /auth/register   {"email","company_name","password"}  → {token, advertiser_id, email}
POST /auth/login      {"email","password"}                 → {token, ...}  + Set-Cookie: gz_token
POST /auth/logout                                          → clears cookie
```

All protected routes accept `Authorization: Bearer <token>` or the `gz_token` cookie.

### Advertisers

```
POST /advertisers              create (admin use; registration preferred)
GET  /advertisers/:id          get profile (wallet balance, etc.)
POST /advertisers/:id/topup    {"amount_cents": 10000}  → updated advertiser
```

### Campaigns

```
GET  /campaigns               ?advertiser_id=...  → list
POST /campaigns               create
GET  /campaigns/:id           get
PUT  /campaigns/:id           update (name, bid, budgets, dates)
POST /campaigns/:id/activate  → status: active
POST /campaigns/:id/pause     → status: paused
```

### Targeting

```
GET /campaigns/:id/targeting
PUT /campaigns/:id/targeting  {"geo_countries":["ARE","SAU"],"device_types":["mobile"],...}
```

All fields are optional on PUT — missing fields are left unchanged.

### Creatives

```
GET  /campaigns/:id/creatives   list creatives for campaign
POST /campaigns/:id/creatives   create creative
POST /creatives/:id/approve     → status: approved  (admin)
POST /creatives/:id/reject      → status: rejected  (admin)
```

### Reporting

```
GET /campaigns/:id/report?from=YYYY-MM-DD&to=YYYY-MM-DD
```

Response:
```json
{
  "campaign_id": "...",
  "from": "2026-05-26",
  "to": "2026-05-26",
  "impressions": 1,
  "spend_cents": 180,
  "clicks": 1,
  "ctr_pct": 100.0,
  "avg_cpm_cents": 180000,
  "conversions": 1,
  "conversion_value_cents": 14999,
  "daily": [
    {"date":"2026-05-26","impressions":1,"spend_cents":180,"clicks":1,"conversions":1,"conversion_value_cents":14999}
  ]
}
```

### Exchanges

```
GET  /exchanges           list all exchanges with status
POST /exchanges           create
GET  /exchanges/:id       get one
PUT  /exchanges/:id       update (name, status, endpoint, floor, macro)
GET  /exchanges/stats     daily impressions + spend per exchange (last 30d)
```

### Billing

```
POST /billing/checkout   {"advertiser_id","amount_cents"}  → {session_id, url}
POST /billing/webhook    Stripe webhook (raw body + Stripe-Signature header)
```

### Bidder endpoints (exchange-facing)

```
POST /bid/:exchange                    OpenRTB bid request → bid response or 204
GET  /win?aid=&bid=&cid=&exchange=&price=  Win notice
GET  /imp/:token                       Impression pixel (1×1 GIF)
GET  /click/:token                     Click redirect → advertiser URL
GET  /vast/:token                      VAST 4.0 XML for video
GET  /conv/:campaign_id?value=49.99    Conversion pixel (1×1 GIF)
GET  /health                           200 OK
```

---

## Running Locally

### Prerequisites

- Rust 1.82+ (`rustup update`)
- PostgreSQL 16 (`brew install postgresql@16`)
- Redis 7 (`brew install redis`)

### Setup

```bash
# 1. Start databases
brew services start postgresql@16
brew services start redis

# 2. Create database user and DB
psql postgres -c "CREATE USER gz WITH PASSWORD 'gz';"
psql postgres -c "CREATE DATABASE gz_ads OWNER gz;"

# 3. Copy env file
cp .env.example .env
# Edit .env — at minimum set JWT_SECRET to something real

# 4. Install sqlx-cli and run migrations
cargo install sqlx-cli --no-default-features --features postgres
export DATABASE_URL=postgres://gz:gz@localhost:5432/gz_ads
sqlx migrate run

# 5. Build release binaries (first build ~90s, subsequent ~25s)
cargo build --release --bin bidder --bin campaign-api

# 6. Start services (in separate terminals or background)
./target/release/campaign-api   # listens on :8081
./target/release/bidder         # listens on :8080

# Optional: pacing + event-consumer
./target/release/pacing
./target/release/event-consumer
```

### Quick smoke test

```bash
# Register
curl -s -X POST http://localhost:8081/auth/register \
  -H "Content-Type: application/json" \
  -d '{"email":"you@co.ae","company_name":"Acme Gulf","password":"pass123"}' | jq .

# Create campaign, targeting, creative, then:

# Send a bid request as ArabyAds
curl -s -X POST http://localhost:8080/bid/arabyads \
  -H "Content-Type: application/json" \
  -d '{
    "id":"test-001",
    "imp":[{"id":"i1","banner":{"w":320,"h":50},"bidfloor":0.5}],
    "site":{"id":"s1","domain":"zawya.com","cat":["IAB13"]},
    "device":{"geo":{"country":"ARE"},"devicetype":4,"os":"ios","language":"ar","ifa":"user-abc"},
    "user":{"id":"u1"},"at":2
  }' | jq .
```

---

## Production Deployment

Uses Docker Compose + nginx + Certbot (Let's Encrypt). All services run as Docker containers with a single multi-stage Dockerfile — the `BIN` build arg selects which binary to compile.

### First-time setup on a fresh VPS

```bash
# 1. Clone the repo
git clone git@github.com:jk-baseer/gz.git && cd gz

# 2. Copy and fill the prod env file
cp .env.example docker/.env
# Fill: POSTGRES_PASSWORD, REDIS_PASSWORD, JWT_SECRET,
#        BID_DOMAIN, APP_DOMAIN, STRIPE_SECRET_KEY, STRIPE_WEBHOOK_SECRET

# 3. Start databases only
docker compose -f docker/docker-compose.prod.yml up -d postgres redis

# 4. Obtain SSL certificates
docker compose -f docker/docker-compose.prod.yml run --rm certbot \
  certonly --webroot -w /var/www/certbot \
  -d bid.gz-ads.com -d app.gz-ads.com \
  --email your@email.com --agree-tos --no-eff-email

# 5. Start everything
docker compose -f docker/docker-compose.prod.yml up -d
```

### Services in production

| Service | Image | Port (internal) | Purpose |
|---|---|---|---|
| `postgres` | postgres:16-alpine | 5432 | Primary database |
| `redis` | redis:7-alpine | 6379 | Budget/freq/pacing cache |
| `clickhouse` | clickhouse/clickhouse-server:24-alpine | 8123 | Analytics store |
| `bidder` | gz-bidder:latest | 8080 | Exchange-facing OpenRTB |
| `campaign-api` | gz-api:latest | 8081 | Advertiser API + dashboard |
| `pacing` | gz-pacing:latest | — | Budget pacing (background) |
| `event-consumer` | gz-events:latest | — | PG → ClickHouse sync |
| `nginx` | nginx:1.27-alpine | 80, 443 | SSL termination + reverse proxy |
| `certbot` | certbot/certbot | — | Certificate renewal (loop) |

nginx routes:
- `bid.gz-ads.com` → `bidder:8080` (200ms proxy timeout for exchange compliance)
- `app.gz-ads.com` → `campaign-api:8081` (gzip enabled)

---

## Environment Variables

| Variable | Required | Default | Description |
|---|---|---|---|
| `DATABASE_URL` | Yes | — | `postgres://user:pass@host/db` |
| `REDIS_URL` | Yes | — | `redis://:pass@host:6379` |
| `JWT_SECRET` | Yes | `change-me` | HS256 signing key — use 32+ random bytes in prod |
| `PUBLIC_HOSTNAME` | Yes | `localhost:8080` | Used in win URLs and tracking pixels |
| `BIDDER_HOST` | No | `0.0.0.0` | Bind address for bidder |
| `BIDDER_PORT` | No | `8080` | |
| `API_HOST` | No | `0.0.0.0` | Bind address for campaign-api |
| `API_PORT` | No | `8081` | |
| `LOG_LEVEL` | No | `info` | `error | warn | info | debug | trace` |
| `STRIPE_SECRET_KEY` | Billing | `""` | `sk_live_...` or `sk_test_...` |
| `STRIPE_WEBHOOK_SECRET` | Billing | `""` | `whsec_...` from Stripe dashboard |
| `CLICKHOUSE_URL` | Analytics | `http://localhost:8123` | ClickHouse HTTP endpoint |
| `CLICKHOUSE_DB` | Analytics | `gz` | ClickHouse database name |

---

## Exchange Integration

Exchanges call us at `POST /bid/{exchange_slug}`. The slug must match a row in `exchange_configs`. Pre-seeded slugs: `arabyads`, `sovrn`, `inmobi`, `smaato`, `pubmatic`, `magnite`.

Configure the win notice (loss notification) URL in the exchange's DSP settings as:

```
https://bid.gz-ads.com/win?aid={AUCTION_ID}&bid={BIDID}&cid={CAMPAIGN_ID}&exchange=arabyads&price=${AUCTION_PRICE}
```

The `${AUCTION_PRICE}` token is replaced by the exchange with the clearing CPM. Different exchanges use different macros — update the `win_price_macro` field in `exchange_configs` accordingly (e.g. some use `%%WINNING_PRICE%%`).

### Exchange-specific notes

| Exchange | Status | Win macro | Notes |
|---|---|---|---|
| ArabyAds | Testing | `${AUCTION_PRICE}` | Primary MENA target — apply at arabyads.com |
| InMobi | Testing | `${AUCTION_PRICE}` | Strong MENA mobile inventory |
| Smaato | Testing | `${AUCTION_PRICE}` | Mobile-first, good GCC reach |
| Sovrn | Testing | `${AUCTION_PRICE}` | Good for integration validation |
| PubMatic | Testing | `${AUCTION_PRICE}` | Scale exchange for Phase 3 |
| Magnite | Testing | `${AUCTION_PRICE}` | Largest independent SSP |

---

## Ad Formats

### Banner

Any IAB standard size. Creative must have `width` and `height` set. Targeting matches by exact creative dimensions against the impression's banner size offer.

Ad markup delivered in `BidResponse.seatbid[].bid[].adm`:
```html
<a href="/click/{token}" target="_blank"><img src="{asset_url}" width="320" height="50"/></a>
<img src="/imp/{token}" width="1" height="1" style="display:none"/>
```

### Native

OpenRTB Native 1.2 JSON string in `adm`. Fields: `title` (text), `image` (url, w, h), `data` (desc, sponsored), `link` (url, clicktrackers).

### Video

VAST 4.0 XML served from `/vast/{token}`. Exchange puts the VAST URL in the `adm` field. Contains:
- `<Impression>` tracking URI
- `<ClickThrough>` with advertiser URL
- `<MediaFile>` with the video asset URL

---

## Targeting Options

| Dimension | Field | Values |
|---|---|---|
| Geography | `geo_countries` | ISO-3166-1-alpha-3 codes: `ARE`, `SAU`, `KWT`, `QAT`, `BHR`, `OMN` |
| Device | `device_types` | `mobile`, `tablet`, `desktop`, `connected_tv` |
| OS | `os_types` | `ios`, `android`, `windows`, `macos` |
| Content | `site_categories` | IAB taxonomy: `IAB13` (Finance), `IAB19` (Tech), etc. |
| Language | `languages` | BCP-47: `ar`, `en` |
| Hour | `hours_of_day` | UTC hours 0–23 (empty array = all hours) |
| Day | `days_of_week` | 0=Sunday … 6=Saturday (empty = all days) |
| Frequency | `frequency_cap_daily` | Max impressions per user per day (uses IFA/IDFA) |

Empty arrays mean "no constraint" — set `geo_countries: []` to target all geographies. All constraints are AND'd together. Within each array, values are OR'd (matching any is sufficient).

### Bid floor

The bidder only bids if `bid_price_cpm_cents / 1000 >= impression.bidfloor`. Floors are respected per-impression.

---

## Conversion Tracking

Place this pixel on your order confirmation page:

```html
<img src="https://bid.gz-ads.com/conv/{CAMPAIGN_ID}?value=ORDER_VALUE_USD"
     width="1" height="1" style="display:none"/>
```

The `value` query parameter is the order value in USD (e.g. `value=149.99`). Stored as `value_cents = round(value * 100)`. Visible in the campaign report under `conversions` and `conversion_value_cents`.

---

## Workspace Structure

```
gz/
├── Cargo.toml                  # Workspace root; shared dependencies
├── Cargo.lock
├── Dockerfile                  # Multi-stage; ARG BIN selects binary
├── .env.example
│
├── crates/
│   ├── openrtb/                # OpenRTB 2.6 request/response types
│   ├── common/                 # Config, telemetry (shared by all services)
│   ├── bidder/                 # Exchange-facing bid server
│   │   ├── src/lib.rs          # budget, frequency, index, targeting, token modules
│   │   ├── src/main.rs         # Router, AppState, startup
│   │   └── tests/targeting_tests.rs   # 18 unit tests
│   ├── campaign-api/           # Advertiser API + dashboard
│   │   └── src/dashboard.html  # Alpine.js + Tailwind UI (served at /)
│   ├── pacing/                 # Budget pacing loop
│   ├── event-consumer/         # PostgreSQL → ClickHouse sync
│   ├── reporting/              # (stub; reporting is in campaign-api today)
│   └── publisher-tag/          # Direct publisher tag endpoint
│
├── migrations/
│   ├── 001_initial.sql         # Core schema
│   ├── 002_click_events.sql    # Click tracking table
│   ├── 003_phase2.sql          # Auth, freq cap, exchange configs
│   ├── 004_native_creatives.sql # Native ad fields on creatives
│   └── 005_conversions.sql     # Conversion events + ClickHouse cursors
│
└── docker/
    ├── docker-compose.yml       # Dev (postgres, redis, clickhouse)
    ├── docker-compose.prod.yml  # Production (all services + nginx + certbot)
    ├── nginx.conf               # SSL termination, reverse proxy
    └── clickhouse_schema.sql    # ClickHouse tables + views
```

---

## Tests

```bash
cargo test --all
```

18 targeting unit tests cover: geo match/mismatch, device type, OS, IAB category, language, bid floor, date range (start/end dates), combined GCC profile, dayparting.

CI runs on GitHub Actions with PostgreSQL and Redis service containers.
