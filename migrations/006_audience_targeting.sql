-- Audience targeting: domain lists, keyword contextual, age range

ALTER TABLE campaign_targeting
    -- Domain targeting: empty = all domains
    ADD COLUMN IF NOT EXISTS domain_allowlist TEXT[]    NOT NULL DEFAULT '{}',
    ADD COLUMN IF NOT EXISTS domain_blocklist TEXT[]    NOT NULL DEFAULT '{}',
    -- Keyword contextual: match against site.keywords in bid request
    ADD COLUMN IF NOT EXISTS keywords         TEXT[]    NOT NULL DEFAULT '{}',
    -- Age range: null = no constraint; matched against user.yob when exchange sends it
    ADD COLUMN IF NOT EXISTS age_min          INTEGER,
    ADD COLUMN IF NOT EXISTS age_max          INTEGER;
