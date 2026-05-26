-- GZ Ad Platform — initial schema

CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- ─── Advertisers ─────────────────────────────────────────────────────────────

CREATE TABLE advertisers (
    id                  UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    email               TEXT        NOT NULL UNIQUE,
    company_name        TEXT        NOT NULL,
    wallet_balance_cents BIGINT     NOT NULL DEFAULT 0 CHECK (wallet_balance_cents >= 0),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ─── Campaigns ────────────────────────────────────────────────────────────────

CREATE TABLE campaigns (
    id                   UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    advertiser_id        UUID        NOT NULL REFERENCES advertisers(id) ON DELETE CASCADE,
    name                 TEXT        NOT NULL,
    status               TEXT        NOT NULL DEFAULT 'draft'
                                     CHECK (status IN ('draft','active','paused','ended')),
    -- CPM bid price in US cents (200 = $2.00 CPM)
    bid_price_cpm_cents  BIGINT      NOT NULL CHECK (bid_price_cpm_cents > 0),
    -- Budgets in US cents
    budget_total_cents   BIGINT      NOT NULL CHECK (budget_total_cents > 0),
    budget_daily_cents   BIGINT               CHECK (budget_daily_cents > 0),
    spend_total_cents    BIGINT      NOT NULL DEFAULT 0,
    start_date           TIMESTAMPTZ,
    end_date             TIMESTAMPTZ,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX campaigns_status_idx ON campaigns(status);
CREATE INDEX campaigns_advertiser_idx ON campaigns(advertiser_id);

-- ─── Campaign targeting ───────────────────────────────────────────────────────

CREATE TABLE campaign_targeting (
    campaign_id     UUID        PRIMARY KEY REFERENCES campaigns(id) ON DELETE CASCADE,
    -- ISO-3166-1-alpha-3 country codes: ARE, SAU, KWT, QAT, BHR, OMN
    geo_countries   TEXT[]      NOT NULL DEFAULT '{}',
    -- "mobile", "desktop", "tablet"
    device_types    TEXT[]      NOT NULL DEFAULT '{}',
    -- "ios", "android", "windows", "macos"
    os_types        TEXT[]      NOT NULL DEFAULT '{}',
    -- IAB taxonomy: IAB13 (Finance), IAB19 (Tech), IAB1 (Arts)…
    site_categories TEXT[]      NOT NULL DEFAULT '{}',
    -- BCP-47: "ar", "en"
    languages       TEXT[]      NOT NULL DEFAULT '{}',
    -- 0–23 (UTC hours)
    hours_of_day    INTEGER[]   NOT NULL DEFAULT '{}',
    -- 0=Sunday … 6=Saturday
    days_of_week    INTEGER[]   NOT NULL DEFAULT '{}'
);

-- ─── Creatives ────────────────────────────────────────────────────────────────

CREATE TABLE creatives (
    id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    campaign_id UUID        NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
    format      TEXT        NOT NULL CHECK (format IN ('banner','native','video')),
    width       INTEGER,
    height      INTEGER,
    asset_url   TEXT        NOT NULL,
    click_url   TEXT        NOT NULL,
    status      TEXT        NOT NULL DEFAULT 'pending_review'
                             CHECK (status IN ('pending_review','approved','rejected')),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX creatives_campaign_idx ON creatives(campaign_id);
CREATE INDEX creatives_status_idx ON creatives(status);

-- ─── Impression events (append-only log) ─────────────────────────────────────
-- In production this moves to ClickHouse. PostgreSQL handles it for MVP.

CREATE TABLE impression_events (
    id                  UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    campaign_id         UUID        NOT NULL,
    creative_id         UUID        NOT NULL,
    exchange            TEXT,
    auction_id          TEXT,
    bid_price_cents     BIGINT      NOT NULL,
    clearing_price_cents BIGINT,
    geo_country         TEXT,
    device_type         TEXT,
    os                  TEXT,
    site_domain         TEXT,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Set when the 1×1 pixel fires — confirms browser rendered the ad
    viewed_at            TIMESTAMPTZ
);

CREATE INDEX imp_events_campaign_idx ON impression_events(campaign_id);
CREATE INDEX imp_events_created_idx  ON impression_events(created_at DESC);

-- ─── Helper: auto-update updated_at ──────────────────────────────────────────

CREATE OR REPLACE FUNCTION set_updated_at()
RETURNS TRIGGER AS $$
BEGIN NEW.updated_at = NOW(); RETURN NEW; END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER advertisers_updated_at BEFORE UPDATE ON advertisers
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE TRIGGER campaigns_updated_at BEFORE UPDATE ON campaigns
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
