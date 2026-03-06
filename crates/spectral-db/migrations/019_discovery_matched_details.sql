-- Add matched value and line number columns to discovery findings
-- This allows showing users the actual PII found and where in the file

ALTER TABLE discovery_findings ADD COLUMN matched_value TEXT;
ALTER TABLE discovery_findings ADD COLUMN line_number INTEGER;
