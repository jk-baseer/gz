-- Creative variant optimization: Thompson Sampling bandit + AI-generated variants

ALTER TABLE creatives
    ADD COLUMN IF NOT EXISTS parent_creative_id UUID REFERENCES creatives(id),
    ADD COLUMN IF NOT EXISTS variant_hypothesis  TEXT,
    -- Full HTML5 banner markup; {{CLICK_URL}} and {{IMP_URL}} are injected at serve time.
    -- NULL = use existing asset_url image banner logic.
    ADD COLUMN IF NOT EXISTS html_adm            TEXT;

-- Optimizer writes serving_probability per creative every 5 minutes.
-- Bidder reads this via the 30-second index refresh.
CREATE TABLE IF NOT EXISTS creative_serving_weights (
    creative_id          UUID PRIMARY KEY REFERENCES creatives(id),
    campaign_id          UUID NOT NULL,
    serving_probability  FLOAT NOT NULL DEFAULT 1.0,
    impressions_snapshot BIGINT NOT NULL DEFAULT 0,
    clicks_snapshot      BIGINT NOT NULL DEFAULT 0,
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_csw_campaign ON creative_serving_weights (campaign_id);
