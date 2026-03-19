//! Thread-pool based PII scanner

use crate::patterns::Matcher;
use crate::types::*;
use crossbeam_channel::{bounded, Receiver, Sender};
use rayon::prelude::*;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use walkdir::WalkDir;

const EXCLUDED_DIRS: &[&str] = &[
    "AppData",
    ".cache",
    "node_modules",
    ".git",
    "target",
    "build",
    "Windows",
    "Program Files",
    "ProgramData",
    "$Recycle.Bin",
    ".npm",
    ".cargo",
    ".rustup",
    "Temp",
    "tmp",
    "venv",
    ".venv",
    ".vscode",
    ".idea",
    "Steam",
    "Games",
    "Videos",
    ".docker",
];

/// Results from a completed scan
#[derive(Debug)]
pub struct ScanResult {
    /// Total files scanned
    pub files_scanned: usize,
    /// All findings from the scan
    pub findings: Vec<FileScanResult>,
    /// Whether the scan was stopped early
    pub was_stopped: bool,
}

/// Scanner that runs on a thread pool
pub struct Scanner {
    patterns: Matcher,
    ignored_paths: HashSet<String>,
    command_rx: Receiver<ScanCommand>,
    progress_tx: Sender<ScanProgress>,
    stop_flag: Arc<AtomicBool>,
    pause_flag: Arc<AtomicBool>,
    files_scanned: Arc<AtomicUsize>,
    files_with_findings: Arc<AtomicUsize>,
}

impl Scanner {
    /// Create a new scanner
    #[must_use]
    pub fn new(
        user_pii: UserPii,
        config: ScanConfig,
        ignored_paths: HashSet<String>,
        command_rx: Receiver<ScanCommand>,
        progress_tx: Sender<ScanProgress>,
    ) -> Self {
        Self {
            patterns: Matcher::new(&user_pii, &config),
            ignored_paths,
            command_rx,
            progress_tx,
            stop_flag: Arc::new(AtomicBool::new(false)),
            pause_flag: Arc::new(AtomicBool::new(false)),
            files_scanned: Arc::new(AtomicUsize::new(0)),
            files_with_findings: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Run the scan on the given directories
    pub fn scan(self, directories: Vec<PathBuf>) -> ScanResult {
        tracing::info!("Scanner starting with {} directories", directories.len());

        if self.patterns.is_empty() {
            tracing::warn!("No patterns configured - scan will not find anything");
            return ScanResult {
                files_scanned: 0,
                findings: Vec::new(),
                was_stopped: false,
            };
        }

        tracing::info!("Starting scan - files will be scanned as discovered");

        let mut all_findings = Vec::new();

        // Process each directory's files immediately without collecting them all first
        for dir in directories {
            if !dir.exists() {
                continue;
            }

            if self.stop_flag.load(Ordering::Relaxed) {
                break;
            }

            // Scan files from this directory in parallel as they're discovered
            let dir_findings: Vec<FileScanResult> = WalkDir::new(&dir)
                .max_depth(MAX_SCAN_DEPTH)
                .follow_links(false)
                .into_iter()
                .filter_entry(|e| !self.should_skip(e.path()))
                .filter_map(|entry| entry.ok())
                .filter(|entry| {
                    let path = entry.path();
                    !self
                        .ignored_paths
                        .contains(&path.to_string_lossy().to_string())
                        && path.is_file()
                        && self.is_scannable(path)
                })
                .map(|e| e.path().to_path_buf())
                .par_bridge()
                .filter_map(|path| {
                    if self.stop_flag.load(Ordering::Relaxed) {
                        return None;
                    }
                    self.check_commands();
                    self.wait_if_paused();

                    let result = self.scan_file(&path);
                    // nosemgrep: llm-prompt-injection-risk
                    let count = self.files_scanned.fetch_add(1, Ordering::Relaxed) + 1;

                    if result.is_some() {
                        self.files_with_findings.fetch_add(1, Ordering::Relaxed);
                    }

                    if count % 100 == 0 {
                        let _ = self.progress_tx.try_send(ScanProgress {
                            files_scanned: count,
                            files_with_findings: self.files_with_findings.load(Ordering::Relaxed),
                            current_directory: path.to_string_lossy().to_string(),
                            is_complete: false,
                            was_stopped: false,
                        });
                    }

                    result
                })
                .collect();

            all_findings.extend(dir_findings);
        }

        let findings = all_findings;

        let was_stopped = self.stop_flag.load(Ordering::Relaxed);
        let files_scanned = self.files_scanned.load(Ordering::Relaxed);

        let _ = self.progress_tx.try_send(ScanProgress {
            files_scanned,
            files_with_findings: self.files_with_findings.load(Ordering::Relaxed),
            current_directory: String::new(),
            is_complete: true,
            was_stopped,
        });

        ScanResult {
            files_scanned,
            findings,
            was_stopped,
        }
    }

    fn should_skip(&self, path: &Path) -> bool {
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            // Check exact matches against excluded list
            if EXCLUDED_DIRS.iter().any(|d| d.eq_ignore_ascii_case(name)) {
                return true;
            }
        }
        false
    }

    fn is_scannable(&self, path: &Path) -> bool {
        let ext_ok = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| SCANNABLE_EXTENSIONS.contains(&e.to_lowercase().as_str()));

        if !ext_ok {
            return false;
        }
        fs::metadata(path).is_ok_and(|m| m.len() <= MAX_FILE_SIZE)
    }

    fn scan_file(&self, path: &Path) -> Option<FileScanResult> {
        let content = fs::read_to_string(path).ok()?;
        let matches = self.patterns.find_all(&content);
        if matches.is_empty() {
            None
        } else {
            Some(FileScanResult {
                path: path.to_path_buf(),
                matches,
            })
        }
    }

    fn check_commands(&self) {
        while let Ok(cmd) = self.command_rx.try_recv() {
            match cmd {
                ScanCommand::Stop => {
                    self.stop_flag.store(true, Ordering::Relaxed);
                }
                ScanCommand::Pause => {
                    self.pause_flag.store(true, Ordering::Relaxed);
                }
                ScanCommand::Continue => {
                    self.pause_flag.store(false, Ordering::Relaxed);
                }
            }
        }
    }

    fn wait_if_paused(&self) {
        while self.pause_flag.load(Ordering::Relaxed) && !self.stop_flag.load(Ordering::Relaxed) {
            std::thread::sleep(std::time::Duration::from_millis(100));
            self.check_commands();
        }
    }
}

/// Create scanner channels for commands and progress
#[must_use]
pub fn create_scanner_channels() -> (
    Sender<ScanCommand>,
    Receiver<ScanCommand>,
    Sender<ScanProgress>,
    Receiver<ScanProgress>,
) {
    let (cmd_tx, cmd_rx) = bounded(10);
    let (progress_tx, progress_rx) = bounded(100);
    (cmd_tx, cmd_rx, progress_tx, progress_rx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_scanner_finds_email() {
        // nosemgrep: no-unwrap-in-production
        let temp = TempDir::new().unwrap();
        let file = temp.path().join("test.txt");
        // nosemgrep: no-unwrap-in-production
        fs::write(&file, "Contact: john@example.com").unwrap();

        let pii = UserPii {
            emails: vec!["john@example.com".into()],
            ..Default::default()
        };
        let (cmd_tx, cmd_rx, progress_tx, _) = create_scanner_channels();
        drop(cmd_tx);

        let scanner = Scanner::new(
            pii,
            ScanConfig::default(),
            HashSet::new(),
            cmd_rx,
            progress_tx,
        );
        let result = scanner.scan(vec![temp.path().to_path_buf()]);

        assert_eq!(result.files_scanned, 1);
        assert_eq!(result.findings.len(), 1);
    }
}
