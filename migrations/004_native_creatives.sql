-- Phase 2: native creative fields

ALTER TABLE creatives
    ADD COLUMN IF NOT EXISTS title_text   TEXT,
    ADD COLUMN IF NOT EXISTS description  TEXT,
    ADD COLUMN IF NOT EXISTS cta_text     TEXT,
    ADD COLUMN IF NOT EXISTS sponsored_by TEXT;
