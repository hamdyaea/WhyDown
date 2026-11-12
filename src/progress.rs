//! Human-visible diagnostic progress without contaminating JSON/text/HTML on stdout.
//! A task-count progress bar shows completed *stages*, never invents an ETA or
//! suggests that bytes/hosts/events have been exhaustively checked.

use std::io::{self, IsTerminal, Write};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressMode {
    /// Show only when stderr is an interactive terminal.
    Auto,
    /// Force output, including in redirected stderr (log-friendly lines).
    Always,
    /// Never write progress output.
    Never,
}

#[derive(Clone)]
struct Status {
    completed: usize,
    total: usize,
    stage: String,
    detail: String,
    start: Instant,
    stage_start: Instant,
    finished: bool,
    succeeded: bool,
    paused_for_output: bool,
}

pub struct Progress {
    state: Option<Arc<Mutex<Status>>>,
    worker: Option<JoinHandle<()>>,
    terminal: bool,
}

impl Progress {
    pub fn new(mode: ProgressMode, stages: usize) -> Self {
        let terminal = io::stderr().is_terminal();
        if stages == 0 || mode == ProgressMode::Never
            || (mode == ProgressMode::Auto && !terminal) {
            return Self { state: None, worker: None, terminal };
        }

        let started = Instant::now();
        let shared = Arc::new(Mutex::new(Status {
            completed: 0, total: stages,
            stage: "Preparing diagnostic".into(),
            detail: String::new(),
            start: started, stage_start: started,
            finished: false, succeeded: false, paused_for_output: false,
        }));
        let reader = Arc::clone(&shared);
        let worker = thread::spawn(move || {
            let mut last_line = String::new();
            let mut last_nonterminal_print = Instant::now() - Duration::from_secs(3);
            loop {
                let snapshot = match reader.lock() {
                    Ok(guard) => guard.clone(),
                    Err(_) => break,
                };
                // Stop repainting and clear the terminal line before the
                // parent writes the report or a saved-file notification.
                if snapshot.paused_for_output {
                    if terminal {
                        let mut err = io::stderr().lock();
                        let _ = write!(err, "\r\x1b[2K");
                        let _ = err.flush();
                    }
                    break;
                }
                let line = format_status(&snapshot);
                // Terminal: animate a stable single line. Forced output to a
                // file/pipe: avoid ANSI and carriage returns; print on stage
                // changes or every 2 seconds while waiting.
                let should_print = terminal ||
                    line != last_line && snapshot.stage_start.elapsed() < Duration::from_millis(260) ||
                    last_nonterminal_print.elapsed() >= Duration::from_secs(2) ||
                    snapshot.finished;
                if should_print {
                    let mut err = io::stderr().lock();
                    if terminal {
                        let _ = write!(err, "\r\x1b[2K{line}");
                        if snapshot.finished { let _ = writeln!(err); }
                    } else {
                        let _ = writeln!(err, "{line}");
                        last_nonterminal_print = Instant::now();
                    }
                    let _ = err.flush();
                    last_line = line;
                }
                if snapshot.finished { break; }
                thread::sleep(Duration::from_millis(160));
            }
        });
        Self { state: Some(shared), worker: Some(worker), terminal }
    }

    pub fn begin(&self, stage: &str) {
        self.modify(|s| {
            s.stage = stage.into();
            s.detail.clear();
            s.stage_start = Instant::now();
        });
    }

    pub fn detail(&self, detail: &str) {
        self.modify(|s| s.detail = detail.into());
    }

    /// Mark a completed stage. Percent reflects the stage plan, not an ETA.
    pub fn complete_step(&self) {
        self.modify(|s| {
            s.completed = (s.completed + 1).min(s.total);
            s.detail.clear();
        });
    }

    /// Join the live renderer before writing stdout or a saved-file notice.
    /// Keep the progress below 100% until the output operation succeeds.
    pub fn prepare_for_output(&mut self) {
        if self.worker.is_some() {
            self.modify(|s| s.paused_for_output = true);
            self.join();
        }
    }

    pub fn finish(&mut self) {
        self.modify(|s| {
            s.completed = s.total;
            s.stage = "Diagnostic complete".into();
            s.detail.clear();
            s.finished = true;
            s.succeeded = true;
        });
        if self.worker.is_some() { self.join(); }
        else { self.print_completion(); }
    }

    fn print_completion(&self) {
        let Some(state) = &self.state else { return };
        let Ok(status) = state.lock() else { return };
        let line = format_status(&status);
        let mut err = io::stderr().lock();
        if self.terminal {
            let _ = writeln!(err, "\r\x1b[2K{line}");
        } else {
            let _ = writeln!(err, "{line}");
        }
        let _ = err.flush();
    }

    fn modify(&self, change: impl FnOnce(&mut Status)) {
        if let Some(state) = &self.state {
            if let Ok(mut status) = state.lock() {
                change(&mut status);
            }
        }
    }

    fn join(&mut self) {
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

impl Drop for Progress {
    fn drop(&mut self) {
        // Early exits must not fake 100% success.
        self.modify(|s| {
            if !s.finished {
                s.stage = "Diagnostic interrupted".into();
                s.detail.clear();
                s.finished = true;
                s.succeeded = false;
            }
        });
        if self.worker.is_some() { self.join(); }
        else { self.print_completion(); }
    }
}

fn format_status(s: &Status) -> String {
    let width = 20usize;
    let filled = s.completed.saturating_mul(width) / s.total.max(1);
    let percent = s.completed.saturating_mul(100) / s.total.max(1);
    let bar = format!("[{}{}]", "#".repeat(filled), "-".repeat(width - filled));
    let suffix = if s.detail.is_empty() { String::new() }
        else { format!(" | {}", s.detail) };
    let outcome = if s.finished && !s.succeeded { " [interrupted]" } else { "" };
    format!(
        "{bar} {percent:>3}% ({}/{}) {}{} | elapsed {}s{}",
        s.completed, s.total, s.stage, suffix,
        s.start.elapsed().as_secs(), outcome
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completed_stage_percent_is_not_elapsed_time() {
        let now = Instant::now();
        let mut status = Status {
            completed: 0, total: 4, stage: "Pacman database".into(),
            detail: "pacman -Dk".into(), start: now, stage_start: now,
            finished: false, succeeded: false, paused_for_output: false,
        };
        let line = format_status(&status);
        assert!(line.contains("[--------------------]   0% (0/4)"));
        assert!(line.contains("pacman -Dk"));
        status.completed = 2;
        assert!(format_status(&status).contains("[##########----------]  50% (2/4)"));
        status.completed = 4;
        assert!(format_status(&status).contains("100% (4/4)"));
    }

    #[test]
    fn forced_progress_pauses_before_report_without_marking_success() {
        let mut p = Progress::new(ProgressMode::Always, 3);
        p.begin("Collecting");
        p.complete_step();
        p.prepare_for_output();
        let paused = p.state.as_ref().unwrap().lock().unwrap().clone();
        assert_eq!(paused.completed, 1);
        assert!(!paused.finished);
        assert!(paused.paused_for_output);
        p.finish();
        let finished = p.state.as_ref().unwrap().lock().unwrap().clone();
        assert!(finished.finished && finished.succeeded);
        assert_eq!(finished.completed, 3);
    }

    #[test]
    fn disabled_and_finished_paths_are_safe() {
        let p = Progress::new(ProgressMode::Never, 3);
        p.begin("test");
        p.detail("subprocess");
        p.complete_step();
        drop(p);
        let mut p = Progress::new(ProgressMode::Never, 1);
        p.prepare_for_output();
        p.finish();
    }
}
