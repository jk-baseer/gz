-- Phase 3: conversion tracking + ClickHouse sync state

CREATE TABLE IF NOT EXISTS conversion_events (
    id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    campaign_id UUID        NOT NULL,
    creative_id UUID,
    auction_id  TEXT,
    exchange    TEXT,
    -- Conversion value in US cents (0 if not reported)
    value_cents BIGINT      NOT NULL DEFAULT 0,
    geo_country TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX conv_events_campaign_idx ON conversion_events(campaign_id);
CREATE INDEX conv_events_created_idx  ON conversion_events(created_at DESC);

-- ClickHouse sync cursors — event-consumer tracks last synced timestamp per topic
CREATE TABLE IF NOT EXISTS ch_sync_cursors (
    topic           TEXT        PRIMARY KEY,
    last_synced_at  TIMESTAMPTZ NOT NULL DEFAULT '1970-01-01'
);

INSERT INTO ch_sync_cursors (topic) VALUES ('impressions'), ('clicks'), ('conversions')
ON CONFLICT DO NOTHING;
