# GZ Ad Platform — DSP Build Plan

## Product Summary

A fully Rust-native Demand-Side Platform (DSP) targeting **e-commerce and fintech advertisers in the GCC/MENA region**. Two supply sources:

1. **Direct inventory** — 5–20 known publishers (apps + websites in GCC) integrate our lightweight tag/SDK. We serve ads directly, keeping full margin.
2. **Exchange inventory** — OpenRTB connections to SSPs with strong MENA coverage, for scale beyond our known publishers.

Advertisers use our platform to run campaigns across both supply sources transparently.

---

## Architecture

```
┌────────────────────────────────────────────────────────┐
│                 Advertiser Dashboard                    │
│              (Web UI served by Rust/Axum)               │
└──────────────────────┬─────────────────────────────────┘
                       │
┌──────────────────────▼─────────────────────────────────┐
│                  Campaign API (Axum)                    │
│  - Campaign CRUD           - Creative management        │
│  - Targeting config        - Reporting endpoints        │
│  - Publisher management    - Billing / wallet           │
└──────┬───────────────┬──────────────────┬──────────────┘
       │               │                  │
┌──────▼──────┐  ┌─────▼──────┐  ┌───────▼──────┐
│   Bidder    │  │   Pacing   │  │  Reporting   │
│   Engine   │  │   Service  │  │   Service    │
│   (Rust)   │  │   (Rust)   │  │   (Rust)     │
└──────┬──────┘  └─────┬──────┘  └───────┬──────┘
       │               │                  │
       │          ┌────▼────┐       ┌─────▼──────┐
       │          │  Redis  │       │ ClickHouse │
       │          └─────────┘       └────────────┘
       │
       │  [Direct inventory]
       ├──────────────► Publisher Tag (JS) → Known GCC websites
       ├──────────────► Publisher SDK (iOS/Android) → Known GCC apps
       │
       │  [Exchange inventory — OpenRTB 2.6]
       ├──────────────► InMobi (strong MENA mobile)
       ├──────────────► Smaato (MENA mobile)
       ├──────────────► ArabyAds (MENA specialist)
       ├──────────────► Choueiri Group / DMS (largest GCC ad network)
       └──────────────► PubMatic (broad MENA web coverage)
```

---

## Supply Strategy

### Direct Publishers (your known contacts)
- You personally onboard 5–20 publishers in GCC
- They paste a single JS tag on their site or integrate a lightweight mobile SDK
- Requests hit your ad server directly — no exchange fee, full margin
- Publisher gets a simple dashboard: earnings, fill rate, eCPM

### Exchange Inventory (scale)
- OpenRTB connections to exchanges with MENA coverage
- Used when direct publisher inventory doesn't fill, or for targeting reach beyond your known publishers
- GCC-relevant exchanges to prioritize (see table below)

---

## GCC/MENA Exchange Priority

| Exchange | Focus | MENA Strength | Access Difficulty |
|---|---|---|---|
| **ArabyAds** | MENA specialist, e-commerce | Native | Easier — MENA-focused, will want partners |
| **Choueiri Group (DMS)** | Largest GCC ad network | Premium GCC web + app | Medium — regional player |
| **InMobi** | Mobile in-app | Strong MENA mobile | Medium |
| **Smaato** | Mobile, global | Good MENA mobile | Medium |
| **PubMatic** | Web + app | Decent MENA web | Medium |
| **Magnite** | Web | Some MENA | Harder |
| **Google AdX** | Everything | Strong but generic | Very Hard |

**Start with ArabyAds** — they are a MENA-native business, actively looking for DSP partnerships, and will have the highest relevance for your GCC advertiser clients.

---

## Ad Formats (Phased)

| Format | Phase | Notes |
|---|---|---|
| Display banners (300x250, 728x90, 320x50) | 1 — MVP | Simplest to implement; all exchanges support |
| Native ads | 2 | Higher CTR for e-commerce; supported by ArabyAds, InMobi |
| Interstitial (full-screen) | 2 | High impact for mobile apps |
| Video / VAST pre-roll | 3 | Complex; needs video creative management |
| Push notifications | 3 | Separate integration; publisher-side setup required |

---

## Tech Stack

| Layer | Technology |
|---|---|
| Language | Rust (entire backend) |
| Async runtime | Tokio |
| HTTP framework | Axum |
| Serialization | Serde / serde_json |
| Database | PostgreSQL (via sqlx) |
| Cache / spend counters | Redis |
| Event streaming | Redpanda (Kafka-compatible, simpler ops) |
| Analytics store | ClickHouse |
| Object storage | Cloudflare R2 (S3-compatible, cheaper egress) |
| CDN | Cloudflare |
| Containerization | Docker Compose (dev) → Kubernetes (prod) |
| CI/CD | GitHub Actions |

---

## Workspace Structure

```
gz/
├── PLAN.md
├── Cargo.toml                  # workspace root
├── crates/
│   ├── openrtb/                # OpenRTB 2.6 types (BidRequest, BidResponse, etc.)
│   ├── common/                 # Config, error types, tracing setup
│   ├── bidder/                 # Core bidder engine (OpenRTB HTTP server)
│   ├── publisher-tag/          # JS tag generator + direct ad serving endpoint
│   ├── campaign-api/           # REST API: campaigns, creatives, reporting
│   ├── pacing/                 # Budget pacing service
│   ├── reporting/              # ClickHouse query service
│   └── event-consumer/         # Redpanda consumer → ClickHouse writer
├── migrations/                 # sqlx PostgreSQL migrations
├── publisher-tag/
│   └── tag.js                  # The JS snippet publishers paste on their sites
├── docker/
│   └── docker-compose.yml      # postgres, redis, redpanda, clickhouse
└── .github/
    └── workflows/
        └── ci.yml
```

---

## Core Data Models

### Advertiser
```
id, email, company_name, wallet_balance_usd, created_at
```

### Campaign
```
id, advertiser_id, name, status (draft/active/paused/ended)
budget_total_usd, budget_daily_usd, spend_to_date_usd
start_date, end_date
bid_price_cpm (max CPM willing to pay)
```

### Targeting
```
campaign_id
geo_countries[]         -- e.g. ["AE", "SA", "KW", "QA", "BH", "OM"]
device_types[]          -- desktop | mobile | tablet
os_types[]              -- ios | android | windows | macos
site_categories[]       -- IAB taxonomy (e.g. IAB13 = Finance)
hour_of_day[]           -- for dayparting (important during Ramadan)
day_of_week[]
languages[]             -- ["ar", "en"]
```

### Creative
```
id, campaign_id, format (banner|native|video)
width, height
asset_url (CDN), click_url
status (pending_review | approved | rejected)
```

### Publisher (direct)
```
id, name, contact_email, domain, type (web|app)
tag_id (unique token embedded in their JS tag)
floor_price_cpm, status (active|inactive)
```

### ImpressionEvent (ClickHouse)
```
timestamp, campaign_id, publisher_id_or_exchange
auction_id, bid_price, clearing_price
geo_country, device_type, os, site_domain
```

---

## Phase 1 — MVP (Weeks 1–6, target: live in 2 months)

**Goal**: Place real bids on ArabyAds, serve direct ads to 1–2 known publishers, track spend.

### Week 1–2: Foundation
- [ ] Cargo workspace: `common`, `openrtb`, `bidder` crates
- [ ] `openrtb` crate: OpenRTB 2.6 BidRequest / BidResponse / Imp types
- [ ] `common`: config (env-based), structured logging (`tracing`), error types
- [ ] Docker Compose: PostgreSQL, Redis, Redpanda, ClickHouse
- [ ] PostgreSQL schema + sqlx migrations (campaigns, targeting, creatives)

### Week 3–4: Bidder Engine
- [ ] Axum HTTP server in `bidder` with `/bid` endpoint
- [ ] Deserialize OpenRTB BidRequest, evaluate against active campaigns
- [ ] Targeting logic: geo, device type, language, IAB category
- [ ] Real-time budget check via Redis atomic counters
- [ ] Serialize and return OpenRTB BidResponse with banner creative
- [ ] Win notice handler (`/win`): record clearing price, update spend
- [ ] Publish impression events to Redpanda

### Week 5: Publisher Tag + Direct Serving
- [ ] `publisher-tag` crate: `/serve/{tag_id}` endpoint (returns ad HTML/JSON)
- [ ] `publisher-tag/tag.js`: async JS snippet publishers paste on their site
- [ ] Tag calls `/serve` → bidder selects best campaign for that publisher → returns creative
- [ ] Click tracking redirect endpoint (`/click/{token}`)
- [ ] Impression pixel endpoint (`/imp/{token}`)

### Week 6: Minimal Campaign Management
- [ ] `campaign-api`: create/edit/pause campaigns via REST
- [ ] Creative upload → Cloudflare R2 → CDN URL
- [ ] Internal admin UI (can be basic: curl or a minimal HTML form)
- [ ] Apply for ArabyAds DSP access (do this in Week 1 in parallel)

**MVP deliverable**: A Rust bidder running, connected to ArabyAds, serving banner ads. You can create a campaign via API and see impressions in ClickHouse. 1–2 of your known publishers have your JS tag running.

---

## Phase 2 — Self-Serve + More Formats (Months 3–5)

**Goal**: Advertisers can sign up and run campaigns without your help.

- [ ] Advertiser auth: JWT login, registration, email verification
- [ ] Stripe integration: wallet top-up (prepaid model)
- [ ] Full advertiser dashboard (React or HTMX + Axum)
- [ ] Publisher dashboard: earnings, fill rate, payout history
- [ ] Native ad format support (title, description, image, CTA)
- [ ] Interstitial format for mobile direct publishers
- [ ] Connect InMobi (mobile MENA reach)
- [ ] Connect Smaato
- [ ] `pacing` service: smooth daily budget distribution
- [ ] `event-consumer`: ClickHouse writer from Redpanda
- [ ] `reporting` service: campaign performance API
- [ ] Frequency capping (Redis-based, per user per campaign per day)

---

## Phase 3 — Scale + Video (Months 5–8)

- [ ] VAST video ad format
- [ ] Connect Choueiri Group / DMS (largest GCC network)
- [ ] Connect PubMatic
- [ ] Fraud detection: bot IP blacklists, click rate anomaly detection
- [ ] Mobile SDK for direct publisher app integration (iOS + Android)
- [ ] Kubernetes deployment
- [ ] Bidder load testing: target 50k bid requests/sec
- [ ] Conversion tracking (postback pixel for e-commerce)
- [ ] Lookalike / contextual audience targeting

---

## Phase 4 — Differentiation (Month 8+)

- [ ] Push notification ad format
- [ ] Arabic-language creative templates (RTL-aware)
- [ ] Ramadan / GCC seasonal campaign scheduling
- [ ] Programmatic advertiser API
- [ ] Private Marketplace (PMP) deal support
- [ ] Third-party audience data integration (regional DMP)
- [ ] Multi-user accounts (agency sub-accounts)

---

## GCC-Specific Considerations

- **Language**: Ads will need Arabic + English. Creative assets need RTL support.
- **Ramadan**: Major spending period. Dayparting is critical (iftar/suhoor peaks).
- **Mobile-first**: GCC has very high smartphone penetration. Mobile inventory will dominate.
- **Payment**: Advertisers in GCC may prefer wire transfer, not just Stripe. Plan for local payment methods.
- **Privacy**: GDPR doesn't apply in GCC, but Saudi Arabia has PDPL and UAE has its own data law. Contextual targeting is safer than user-identity targeting.

---

## Key Risks

| Risk | Mitigation |
|---|---|
| Exchange onboarding takes too long | Apply to ArabyAds in Week 1; use test bid request mocks to develop bidder in parallel |
| Bidder >100ms p99 | Rust + in-memory campaign index; benchmark from day one |
| Budget overspend | Redis atomic INCRBY with hard cap; per-campaign kill switch |
| Low fill rate on direct inventory | Exchange fallback fills when no campaign matches direct publisher |
| GCC publishers prefer local relationships | Your existing network is the moat here — lean on it |
| Ad fraud | Basic IP/UA filtering in Phase 1; anomaly detection in Phase 3 |

---

## Business Model

- **Direct inventory margin**: You control 100% of the CPM spread — no exchange fees.
- **Exchange margin**: Buy at clearing price, bill advertiser at clearing price + 20–25% platform fee.
- **Advertiser billing**: Prepaid wallet (reduces risk). Stripe for international cards; local bank transfer for GCC corporates.
- **Publisher payouts**: Monthly, via bank transfer (common in GCC) or Wise.

---

## Immediate Next Steps (This Week)

1. `git init` ✅
2. Set up Cargo workspace with initial crate skeletons
3. Write OpenRTB 2.6 types in `openrtb` crate
4. Spin up Docker Compose dev environment
5. **Apply for ArabyAds DSP access** (no code dependency — do this today)
6. **Reach out to 2–3 of your known publishers** to confirm they'll integrate your tag (so you have real inventory ready for Phase 1)
