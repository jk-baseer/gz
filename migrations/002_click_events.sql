-- Click events: fired when a user clicks an ad creative
CREATE TABLE click_events (
    id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    campaign_id UUID        NOT NULL,
    creative_id UUID        NOT NULL,
    auction_id  TEXT,
    exchange    TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX click_events_campaign_idx ON click_events(campaign_id);
CREATE INDEX click_events_created_idx  ON click_events(created_at DESC);
