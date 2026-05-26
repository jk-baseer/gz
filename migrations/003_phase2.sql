-- Phase 2: auth, frequency capping, exchange registry

-- ── Advertiser auth fields ─────────────────────────────────────────────────

ALTER TABLE advertisers
    ADD COLUMN IF NOT EXISTS password_hash    TEXT,
    ADD COLUMN IF NOT EXISTS email_verified   BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS verified_at      TIMESTAMPTZ;

-- ── Frequency capping per campaign ────────────────────────────────────────

ALTER TABLE campaigns
    ADD COLUMN IF NOT EXISTS frequency_cap_daily INTEGER;

-- ── Exchange configurations ────────────────────────────────────────────────
-- Stores each SSP/exchange we have an active connection with.
-- The exchange calls our bidder at /bid/<slug>.

CREATE TABLE IF NOT EXISTS exchange_configs (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    slug            TEXT        NOT NULL UNIQUE,   -- used in /bid/:exchange URL
    name            TEXT        NOT NULL,
    status          TEXT        NOT NULL DEFAULT 'active'
                                CHECK (status IN ('active','paused','testing')),
    -- Their endpoint (for future outbound requests / deal IDs)
    endpoint_url    TEXT,
    -- Win notice macro format differs by exchange
    -- Standard: ${AUCTION_PRICE}, some use %%WINNING_PRICE%%
    win_price_macro TEXT        NOT NULL DEFAULT '${AUCTION_PRICE}',
    -- Optional: minimum bid floor we accept from this exchange (CPM cents)
    min_floor_cents INTEGER     NOT NULL DEFAULT 0,
    notes           TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TRIGGER exchange_configs_updated_at BEFORE UPDATE ON exchange_configs
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- Seed the initial exchanges
INSERT INTO exchange_configs (slug, name, status, win_price_macro, notes) VALUES
    ('arabyads',  'ArabyAds',            'testing', '${AUCTION_PRICE}',  'Primary MENA exchange — apply at arabyads.com'),
    ('sovrn',     'Sovrn',               'testing', '${AUCTION_PRICE}',  'Global long-tail — good for onboarding validation'),
    ('inmobi',    'InMobi',              'testing', '${AUCTION_PRICE}',  'Strong MENA mobile'),
    ('smaato',    'Smaato',              'testing', '${AUCTION_PRICE}',  'Mobile MENA / global'),
    ('pubmatic',  'PubMatic',            'testing', '${AUCTION_PRICE}',  'Scale exchange — Phase 3'),
    ('magnite',   'Magnite',             'testing', '${AUCTION_PRICE}',  'Largest independent SSP — Phase 3')
ON CONFLICT (slug) DO NOTHING;

-- ── Exchange daily stats view (derived from impression_events) ─────────────

CREATE OR REPLACE VIEW exchange_daily_stats AS
SELECT
    exchange,
    DATE_TRUNC('day', created_at)::DATE AS date,
    COUNT(*)                            AS impressions,
    COALESCE(SUM(clearing_price_cents), 0) AS spend_cents,
    COALESCE(AVG(clearing_price_cents), 0) AS avg_cpm_cents
FROM impression_events
GROUP BY exchange, DATE_TRUNC('day', created_at)::DATE;
