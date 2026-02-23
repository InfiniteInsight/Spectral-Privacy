-- Add current_broker_name to scan_jobs for displaying which broker is being scanned
ALTER TABLE scan_jobs ADD COLUMN current_broker_name TEXT;
