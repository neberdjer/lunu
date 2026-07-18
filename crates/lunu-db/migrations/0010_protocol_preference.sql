ALTER TABLE quality_profiles ADD COLUMN preferred_protocol TEXT;
ALTER TABLE quality_profiles ADD COLUMN protocol_weight BIGINT NOT NULL DEFAULT 100;
