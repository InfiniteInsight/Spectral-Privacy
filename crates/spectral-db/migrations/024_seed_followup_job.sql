INSERT OR IGNORE INTO scheduled_jobs (id, job_type, interval_days, next_run_at, enabled)
VALUES ('default-followup-reminders', 'FollowUpReminders', 1, datetime('now'), 1);
