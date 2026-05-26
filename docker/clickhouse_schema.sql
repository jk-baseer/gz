-- ClickHouse analytics schema for GZ Ads
-- Run once after starting ClickHouse (or let event-consumer bootstrap it automatically).
-- Connect: clickhouse-client --host localhost

CREATE DATABASE IF NOT EXISTS gz;

-- ─── Impressions ─────────────────────────────────────────────────────────────
-- One row per won auction. Synced from PostgreSQL by event-consumer.

CREATE TABLE IF NOT EXISTS gz.impression_events (
    id                   UUID,
    campaign_id          UUID,
    creative_id          UUID,
    exchange             String,
    auction_id           String,
    bid_price_cents      Int64,
    clearing_price_cents Int64,
    geo_country          String,
    device_type          String,
    os                   String,
    site_domain          String,
    created_at           DateTime64(3, 'UTC'),
    viewed_at            Nullable(DateTime64(3, 'UTC'))
) ENGINE = MergeTree()
PARTITION BY toYYYYMM(created_at)
ORDER BY (campaign_id, created_at)
TTL created_at + INTERVAL 2 YEAR;

-- ─── Clicks ──────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS gz.click_events (
    id           UUID,
    campaign_id  UUID,
    creative_id  UUID,
    auction_id   String,
    exchange     String,
    created_at   DateTime64(3, 'UTC')
) ENGINE = MergeTree()
PARTITION BY toYYYYMM(created_at)
ORDER BY (campaign_id, created_at)
TTL created_at + INTERVAL 2 YEAR;

-- ─── Conversions ─────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS gz.conversion_events (
    id           UUID,
    campaign_id  UUID,
    value_cents  Int64,
    created_at   DateTime64(3, 'UTC')
) ENGINE = MergeTree()
PARTITION BY toYYYYMM(created_at)
ORDER BY (campaign_id, created_at)
TTL created_at + INTERVAL 2 YEAR;

-- ─── Handy aggregation views ─────────────────────────────────────────────────

CREATE OR REPLACE VIEW gz.campaign_daily AS
SELECT
    campaign_id,
    toDate(created_at)              AS date,
    countIf(clearing_price_cents>0) AS impressions,
    sum(clearing_price_cents)       AS spend_cents,
    avg(clearing_price_cents)       AS avg_cpm_cents,
    countIf(viewed_at IS NOT NULL)  AS viewable
FROM gz.impression_events
GROUP BY campaign_id, date;

CREATE OR REPLACE VIEW gz.exchange_daily AS
SELECT
    exchange,
    toDate(created_at)    AS date,
    count()               AS impressions,
    sum(clearing_price_cents) AS spend_cents
FROM gz.impression_events
GROUP BY exchange, date;
