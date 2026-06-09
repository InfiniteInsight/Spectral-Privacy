//! Background scheduler runner — polls for due jobs and dispatches them.
//!
//! The runner is decoupled from any database or scanner crate via three
//! async callbacks supplied by the host application:
//!
//! * `get_jobs`  — load the current list of `ScheduledJob`s
//! * `dispatch`  — execute a single `JobType`
//! * `update`    — persist the new `next_run_at` / `last_run_at` after dispatch

use crate::{is_job_due, next_run_timestamp, JobType, ScheduledJob};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

type GetJobsFuture = Pin<Box<dyn Future<Output = Vec<ScheduledJob>> + Send>>;
type DispatchFuture = Pin<Box<dyn Future<Output = ()> + Send>>;

/// Polls for due scheduled jobs at a fixed interval and invokes caller-supplied
/// async callbacks for job loading, dispatching, and timestamp updates.
pub struct SchedulerRunner {
    poll_secs: u64,
    get_jobs: Arc<dyn Fn() -> GetJobsFuture + Send + Sync>,
    dispatch: Arc<dyn Fn(JobType) -> DispatchFuture + Send + Sync>,
    update: Arc<dyn Fn(String, String, String) -> DispatchFuture + Send + Sync>,
}

impl SchedulerRunner {
    /// Create a new runner.
    ///
    /// # Arguments
    /// * `poll_secs`   — seconds between poll ticks
    /// * `get_jobs`    — async callback returning all scheduled jobs
    /// * `dispatch`    — async callback that executes the given job type
    /// * `update`      — async callback receiving `(job_id, next_run_at, last_run_at)`
    pub fn new(
        poll_secs: u64,
        get_jobs: impl Fn() -> GetJobsFuture + Send + Sync + 'static,
        dispatch: impl Fn(JobType) -> DispatchFuture + Send + Sync + 'static,
        update: impl Fn(String, String, String) -> DispatchFuture + Send + Sync + 'static,
    ) -> Self {
        Self {
            poll_secs,
            get_jobs: Arc::new(get_jobs),
            dispatch: Arc::new(dispatch),
            update: Arc::new(update),
        }
    }

    /// Run a single poll tick.
    ///
    /// Loads all jobs, dispatches enabled ones that are due, and advances their schedule.
    /// Calling this directly is useful in tests.
    pub async fn tick(&self) {
        let now = chrono::Utc::now().to_rfc3339();
        let jobs = (self.get_jobs)().await;

        for job in jobs {
            if !job.enabled {
                tracing::debug!(job_id = %job.id, "Skipping disabled job");
                continue;
            }
            if is_job_due(&job.next_run_at, &now) {
                tracing::info!(job_id = %job.id, job_type = ?job.job_type, "Dispatching due scheduled job");
                (self.dispatch)(job.job_type).await;
                let next = next_run_timestamp(job.interval_days);
                (self.update)(job.id, next, now.clone()).await;
            }
        }
    }

    /// Spawn a background tokio task that calls [`tick`](Self::tick) every `poll_secs` seconds.
    ///
    /// The first tick fires after one full interval (no immediate tick on start).
    /// The returned `JoinHandle` can be dropped; the task continues running.
    pub fn start(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        let runner = Arc::clone(&self);
        let interval_dur = tokio::time::Duration::from_secs(runner.poll_secs);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(interval_dur);
            interval.tick().await; // consume the immediate first tick
            loop {
                interval.tick().await;
                runner.tick().await;
            }
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::JobType;
    use std::sync::{Arc, Mutex};

    fn due_job(id: &str, job_type: JobType) -> ScheduledJob {
        ScheduledJob {
            id: id.to_string(),
            job_type,
            interval_days: 7,
            next_run_at: "2020-01-01T00:00:00Z".to_string(), // in the past
            last_run_at: None,
            enabled: true,
        }
    }

    fn future_job(id: &str, job_type: JobType) -> ScheduledJob {
        ScheduledJob {
            id: id.to_string(),
            job_type,
            interval_days: 7,
            next_run_at: "2099-01-01T00:00:00Z".to_string(), // far future
            last_run_at: None,
            enabled: true,
        }
    }

    fn make_runner(
        jobs: Vec<ScheduledJob>,
        dispatched: Arc<Mutex<Vec<JobType>>>,
        updated: Arc<Mutex<Vec<String>>>,
    ) -> Arc<SchedulerRunner> {
        Arc::new(SchedulerRunner::new(
            60,
            move || {
                let jobs = jobs.clone();
                Box::pin(async move { jobs })
            },
            move |job_type| {
                let d = Arc::clone(&dispatched);
                Box::pin(async move {
                    d.lock().expect("test mutex not poisoned").push(job_type);
                })
            },
            move |id, _next, _last| {
                let u = Arc::clone(&updated);
                Box::pin(async move {
                    u.lock().expect("test mutex not poisoned").push(id);
                })
            },
        ))
    }

    #[tokio::test]
    async fn test_tick_dispatches_due_job() {
        let dispatched: Arc<Mutex<Vec<JobType>>> = Arc::new(Mutex::new(vec![]));
        let updated: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));

        let jobs = vec![
            due_job("job-1", JobType::ScanAll),
            future_job("job-2", JobType::FollowUpReminders),
        ];

        let runner = make_runner(jobs, Arc::clone(&dispatched), Arc::clone(&updated));
        runner.tick().await;

        let d = dispatched.lock().expect("test mutex not poisoned");
        assert_eq!(d.len(), 1, "Only the due job should be dispatched");
        assert_eq!(d[0], JobType::ScanAll);

        let u = updated.lock().expect("test mutex not poisoned");
        assert_eq!(u.len(), 1);
        assert_eq!(u[0], "job-1");
    }

    #[tokio::test]
    async fn test_tick_skips_disabled_jobs() {
        let dispatched: Arc<Mutex<Vec<JobType>>> = Arc::new(Mutex::new(vec![]));
        let updated: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));

        let mut job = due_job("job-1", JobType::ScanAll);
        job.enabled = false;

        let runner = make_runner(vec![job], Arc::clone(&dispatched), Arc::clone(&updated));
        runner.tick().await;

        assert!(
            dispatched
                .lock()
                .expect("test mutex not poisoned")
                .is_empty(),
            "Disabled jobs must not be dispatched"
        );
    }

    #[tokio::test]
    async fn test_tick_no_jobs_due() {
        let dispatched: Arc<Mutex<Vec<JobType>>> = Arc::new(Mutex::new(vec![]));
        let updated: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));

        let runner = make_runner(
            vec![future_job("job-1", JobType::ScanAll)],
            Arc::clone(&dispatched),
            Arc::clone(&updated),
        );
        runner.tick().await;

        assert!(
            dispatched
                .lock()
                .expect("test mutex not poisoned")
                .is_empty(),
            "Future jobs must not be dispatched"
        );
    }

    #[tokio::test]
    async fn test_tick_dispatches_multiple_due_jobs() {
        let dispatched: Arc<Mutex<Vec<JobType>>> = Arc::new(Mutex::new(vec![]));
        let updated: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));

        let jobs = vec![
            due_job("job-1", JobType::ScanAll),
            due_job("job-2", JobType::FollowUpReminders),
            future_job("job-3", JobType::VerifyRemovals),
        ];

        let runner = make_runner(jobs, Arc::clone(&dispatched), Arc::clone(&updated));
        runner.tick().await;

        let d = dispatched.lock().expect("test mutex not poisoned");
        assert_eq!(d.len(), 2, "Both due jobs should be dispatched");

        let u = updated.lock().expect("test mutex not poisoned");
        assert_eq!(u.len(), 2);
    }

    #[tokio::test]
    async fn test_tick_empty_job_list() {
        let dispatched: Arc<Mutex<Vec<JobType>>> = Arc::new(Mutex::new(vec![]));
        let updated: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));

        let runner = make_runner(vec![], Arc::clone(&dispatched), Arc::clone(&updated));
        runner.tick().await;

        assert!(dispatched
            .lock()
            .expect("test mutex not poisoned")
            .is_empty());
    }
}
