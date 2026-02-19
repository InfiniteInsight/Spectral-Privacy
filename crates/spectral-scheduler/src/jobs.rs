//! Job type definitions.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum JobType {
    ScanAll,
    VerifyRemovals,
    PollImap,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledJob {
    pub id: String,
    pub job_type: JobType,
    pub interval_days: u32,
    pub next_run_at: String,
    pub last_run_at: Option<String>,
    pub enabled: bool,
}
