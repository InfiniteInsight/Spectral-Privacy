//! Job scheduling — determines when queued jobs are due.

use chrono::DateTime;

/// Returns true if `next_run_at` is in the past relative to `now`.
pub fn is_job_due(next_run_at: &str, now: &str) -> bool {
    let next = DateTime::parse_from_rfc3339(next_run_at).ok();
    let current = DateTime::parse_from_rfc3339(now).ok();
    match (next, current) {
        (Some(n), Some(c)) => n <= c,
        _ => false,
    }
}

/// Return the ISO-8601 timestamp for `now + interval_days`.
pub fn next_run_timestamp(interval_days: u32) -> String {
    use chrono::Utc;
    // nosemgrep: llm-prompt-injection-risk - false positive, this is chrono date arithmetic
    let next = Utc::now() + chrono::Duration::days(interval_days as i64);
    next.to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_is_due_past_next_run() {
        let now = "2026-02-17T12:00:00Z".to_string();
        let next_run = "2026-02-17T11:00:00Z".to_string();
        assert!(is_job_due(&next_run, &now));
    }

    #[test]
    fn test_job_not_due_future_next_run() {
        let now = "2026-02-17T12:00:00Z".to_string();
        let next_run = "2026-02-17T13:00:00Z".to_string();
        assert!(!is_job_due(&next_run, &now));
    }
}
