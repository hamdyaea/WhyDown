//! Read-only, zero-dependency interactive terminal browser for WhyDown reports.
//!
//! Deliberately line-oriented (number + Enter): this works in ordinary TTYs
//! and SSH without switching the terminal into raw mode or invoking stty.
//! It does not change files, services, network state, terminal settings,
//! or run any additional commands beyond the existing read-only collectors.

use crate::cli::Cli;
use crate::model::{Focus, Report, Severity};
use crate::progress::{Progress, ProgressMode};
use crate::{analyze, collect, demo, time};
use std::io::{self, BufRead, IsTerminal, Write};

const LOOKBACKS: [u64; 4] = [3_600, 4 * 3_600, 86_400, 7 * 86_400];
const MAX_LIST: usize = 30;

/// Run only with interactive stdin AND stdout; never consume pipeline input.
pub fn run(cli: Cli) -> Result<u8, String> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err("The interactive TUI requires stdin and stdout to be terminals. Use 'whydown diagnose --format text|json|html' for pipes, scripts and CI.".into());
    }
    let use_clear = std::env::var("TERM")
        .map(|term| !term.is_empty() && term != "dumb")
        .unwrap_or(false);
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let stdout = io::stdout();
    let mut output = stdout.lock();
    run_with_io(&mut input, &mut output, cli, use_clear)
        .map_err(|e| format!("Interactive terminal I/O failed: {e}"))?;
    Ok(0)
}

/// Public for controlled, no-host-data tests or custom terminal frontends.
/// In particular, a demo session never runs host collectors.
pub fn run_with_io<R: BufRead, W: Write>(
    input: &mut R, output: &mut W, mut cli: Cli, clear_screen: bool,
) -> io::Result<()> {
    let demo_mode = cli.tui_demo;
    let mut last: Option<Report> = None;
    // Never silently discard an action typed on a sub-page's continue prompt.
    let mut pending_selection: Option<String> = None;
    let mut notice = String::from("Type a menu number followed by Enter.");
    loop {
        heading(output, clear_screen, "Dashboard")?;
        writeln!(output, "Mode: {}  |  Focus: {}  |  Lookback: {}  |  Boot: {}  |  Journal cap: {}",
            if demo_mode { "DEMO (fictional)" } else { "LIVE (read-only)" },
            cli.focus.as_str(), lookback(cli.since_seconds),
            if cli.current_boot { "current only" } else { "all accessible" },
            cli.max_events)?;
        writeln!(output, "\nSTATUS: {notice}")?;
        match &last {
            Some(report) => {
                let warnings = report.findings.iter()
                    .filter(|f| f.severity == Severity::Warning).count();
                let info = report.findings.len().saturating_sub(warnings);
                writeln!(output, "Last diagnostic: {warnings} warning(s), {info} info finding(s); {} journal entries{}",
                    report.snapshot.journal_entries_examined,
                    if report.snapshot.journal_truncated { " [PARTIAL]" } else { "" })?;
                writeln!(output, "Current local evidence is a snapshot; historical warnings may have recovered.")?;
            }
            None => writeln!(output, "No report for these settings yet. Select 1 to run a diagnostic.")?,
        }
        writeln!(output, "\n  1  Run diagnostic")?;
        writeln!(output, "  2  Change focus")?;
        writeln!(output, "  3  Change journal lookback")?;
        writeln!(output, "  4  Toggle current-boot filter")?;
        writeln!(output, "  5  Toggle journal entry cap (5000 / 10000)")?;
        writeln!(output, "  6  Browse findings and evidence")?;
        writeln!(output, "  7  Recent classified timeline")?;
        writeln!(output, "  8  Coverage and limitations")?;
        writeln!(output, "  9  Copyable safe export commands")?;
        writeln!(output, "  0  Quit")?;
        writeln!(output, "\nChoose a number and press Enter. Options 2-5 require option 1 to refresh.")?;
        let selected = match pending_selection.take() {
            Some(next) => Some(next),
            None => choice(input, output, "Selection (number + Enter)")?,
        };
        match selected.as_deref() {
            Some("0" | "q" | "Q") | None => break,
            Some("1") => {
                heading(output, clear_screen, "Collecting evidence")?;
                writeln!(output, "Running {} diagnostic. No repairs, network probes or telemetry.",
                    if demo_mode { "fictional" } else { "read-only live" })?;
                output.flush()?;
                last = Some(diagnose_now(&cli, demo_mode));
                let report = last.as_ref().expect("diagnostic report just created");
                let warnings = report.findings.iter()
                    .filter(|f| f.severity == Severity::Warning).count();
                notice = format!(
                    "Diagnostic completed: {warnings} warning(s), {} finding(s), {} journal entries{}. Choose 6 to view evidence, 7 for events, or 8 for coverage.",
                    report.findings.len(), report.snapshot.journal_entries_examined,
                    if report.snapshot.journal_truncated { " [PARTIAL: try option 5]" } else { "" }
                );
                // No extra pause: the next number is a dashboard action.
            }
            Some("2") => {
                cli.focus = match cli.focus {
                    Focus::All => Focus::Network,
                    Focus::Network => Focus::Services,
                    Focus::Services => Focus::System,
                    Focus::System => Focus::All,
                };
                last = None; // no stale report shown with new configuration
                notice = format!("Focus changed to {}. Select 1 to refresh the diagnostic.", cli.focus.as_str());
            }
            Some("3") => {
                let index = LOOKBACKS.iter().position(|v| *v == cli.since_seconds);
                cli.since_seconds = LOOKBACKS[index.map(|i| (i + 1) % LOOKBACKS.len()).unwrap_or(0)];
                last = None;
                notice = format!("Lookback changed to {}. Select 1 to refresh the diagnostic.", lookback(cli.since_seconds));
            }
            Some("4") => {
                cli.current_boot = !cli.current_boot;
                last = None;
                notice = format!("Current boot filter {}. Select 1 to refresh the diagnostic.",
                    if cli.current_boot { "enabled" } else { "disabled" });
            }
            Some("5") => {
                cli.max_events = if cli.max_events == 10_000 { 5_000 } else { 10_000 };
                last = None;
                notice = format!("Journal cap changed to {}. Select 1 to refresh the diagnostic.", cli.max_events);
            }
            Some("6") => {
                pending_selection = findings(input, output, clear_screen, last.as_ref())?;
                notice = "Returned from findings. Enter a menu number and press Enter.".into();
            }
            Some("7") => {
                pending_selection = timeline(input, output, clear_screen, last.as_ref())?;
                notice = "Returned from timeline. Enter a menu number and press Enter.".into();
            }
            Some("8") => {
                pending_selection = coverage(input, output, clear_screen, last.as_ref())?;
                notice = "Returned from coverage. Enter a menu number and press Enter.".into();
            }
            Some("9") => {
                pending_selection = export_guide(input, output, clear_screen, &cli, demo_mode)?;
                notice = "Returned from export guidance. Enter a menu number and press Enter.".into();
            }
            _ => notice = "Unknown selection: enter one of the displayed numbers followed by Enter.".into(),
        }
    }
    writeln!(output, "\nWhyDown TUI closed; no system changes made.")?;
    output.flush()
}

fn diagnose_now(cli: &Cli, demo_mode: bool) -> Report {
    let since = time::now_unix().saturating_sub(cli.since_seconds);
    let mut snapshot = if demo_mode {
        demo::snapshot()
    } else {
        let silent = Progress::new(ProgressMode::Never, 0);
        collect::collect_with_options(
            cli.focus, since, cli.max_events, cli.disk_details,
            cli.current_boot, cli.include_remote_mounts, &silent,
        )
    };
    if demo_mode {
        snapshot.events.retain(|event| event.timestamp >= since);
    }
    analyze(snapshot, cli.since_seconds, cli.focus, demo_mode)
}

fn heading<W: Write>(out: &mut W, clear: bool, page: &str) -> io::Result<()> {
    if clear {
        write!(out, "\x1b[2J\x1b[H")?;
    } else {
        writeln!(out, "\n{}", "=".repeat(64))?;
    }
    writeln!(out, "WhyDown {}  |  Interactive terminal  |  {}", env!("CARGO_PKG_VERSION"), page)?;
    writeln!(out, "{}", "-".repeat(64))?;
    Ok(())
}

fn lookback(seconds: u64) -> String {
    if seconds % 86_400 == 0 { format!("{}d", seconds / 86_400) }
    else if seconds % 3_600 == 0 { format!("{}h", seconds / 3_600) }
    else { format!("{}m", seconds / 60) }
}

fn choice<R: BufRead, W: Write>(input: &mut R, output: &mut W, prompt: &str) -> io::Result<Option<String>> {
    write!(output, "\n{prompt} > ")?;
    output.flush()?;
    let mut line = String::new();
    if input.read_line(&mut line)? == 0 { return Ok(None); }
    Ok(Some(line.trim().chars().take(32).collect()))
}

/// Continue or forward the next dashboard command instead of swallowing it.
fn pause<R: BufRead, W: Write>(input: &mut R, output: &mut W) -> io::Result<Option<String>> {
    let next = choice(input, output, "Enter = return, or type 1-9 + Enter for next action")?;
    Ok(next.filter(|value| matches!(value.as_str(),
        "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")))
}

/// Journal-controlled identifiers and local filesystem paths are not allowed
/// to inject escape sequences, carriage returns or other terminal control.
fn safe(value: &str) -> String {
    value.chars().take(240)
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}

fn findings<R: BufRead, W: Write>(
    input: &mut R, output: &mut W, clear: bool, report: Option<&Report>,
) -> io::Result<Option<String>> {
    heading(output, clear, "Findings")?;
    let Some(report) = report else {
        writeln!(output, "Run a diagnostic first (menu option 1).")?;
        return pause(input, output);
    };
    if report.findings.is_empty() {
        writeln!(output, "No classified findings. This is not a health guarantee.")?;
        return pause(input, output);
    }
    for (index, finding) in report.findings.iter().take(MAX_LIST).enumerate() {
        writeln!(output, "{:>2}. [{}] {} ({})",
            index + 1, finding.severity.as_str(), safe(&finding.title),
            finding.scope())?;
    }
    if report.findings.len() > MAX_LIST {
        writeln!(output, "Showing the first {MAX_LIST} findings; use --format json for all.")?;
    }
    writeln!(output, " 0. Return to dashboard")?;
    let Some(option) = choice(input, output, "Finding number")? else { return Ok(None); };
    let Ok(number) = option.parse::<usize>() else { return Ok(None); };
    if number == 0 || number > report.findings.len().min(MAX_LIST) { return Ok(None); }
    let finding = &report.findings[number - 1];
    heading(output, clear, "Finding evidence")?;
    writeln!(output, "{} [{}] | confidence={} | scope={}",
        safe(&finding.title), finding.severity.as_str(),
        finding.confidence.as_str(), finding.scope())?;
    writeln!(output, "\n{}", safe(&finding.explanation))?;
    writeln!(output, "\nEvidence:")?;
    for evidence in &finding.evidence {
        writeln!(output, "  - {}", safe(evidence))?;
    }
    writeln!(output, "\nSuggested read-only verification:")?;
    for check in &finding.checks {
        writeln!(output, "  - {}", safe(check))?;
    }
    writeln!(output, "\nFindings are hypotheses/observations, not verified root causes.")?;
    pause(input, output)
}

fn timeline<R: BufRead, W: Write>(
    input: &mut R, output: &mut W, clear: bool, report: Option<&Report>,
) -> io::Result<Option<String>> {
    heading(output, clear, "Recent classified events")?;
    let Some(report) = report else {
        writeln!(output, "Run a diagnostic first (menu option 1).")?;
        return pause(input, output);
    };
    if report.timeline.is_empty() {
        writeln!(output, "No classified events; journal coverage may be incomplete.")?;
    } else {
        for event in report.timeline.iter().rev().take(25) {
            writeln!(output, "{}  {}{}",
                time::utc_timestamp(event.timestamp),
                safe(event.kind.label()),
                event.subject.as_deref().map(|s| format!(" [{}]", safe(s))).unwrap_or_default())?;
        }
        if report.timeline.len() > 25 {
            writeln!(output, "Showing latest 25 / {} classified events. Use JSON for all.", report.timeline.len())?;
        }
        writeln!(output, "Journal entries are historical and ordered only to retained second precision.")?;
    }
    pause(input, output)
}

fn coverage<R: BufRead, W: Write>(
    input: &mut R, output: &mut W, clear: bool, report: Option<&Report>,
) -> io::Result<Option<String>> {
    heading(output, clear, "Collection coverage & uncertainty")?;
    let Some(report) = report else {
        writeln!(output, "Run a diagnostic first (menu option 1).")?;
        return pause(input, output);
    };
    let s = &report.snapshot;
    writeln!(output, "Journal collected: {}  |  entries: {}  |  truncated: {}",
        s.journal_checked, s.journal_entries_examined, s.journal_truncated)?;
    writeln!(output, "Current boot filter requested: {}", s.journal_current_boot_only)?;
    writeln!(output, "Systemd manager checked: {}  |  failed units: {}",
        s.system_units_checked, s.failed_units.len())?;
    writeln!(output, "Filesystems checked: {}", s.disk_usage_checked)?;
    writeln!(output, "Current service states sampled: {}", s.service_statuses.len())?;
    for status in s.service_statuses.iter().take(10) {
        writeln!(output, "  {}: {}/{} (result={})",
            safe(&status.unit), safe(&status.active_state),
            safe(&status.sub_state), safe(&status.result))?;
    }
    if !s.notes.is_empty() {
        writeln!(output, "\nCollection limitations:")?;
        for note in s.notes.iter().take(15) {
            writeln!(output, "  - {}", safe(note))?;
        }
        if s.notes.len() > 15 { writeln!(output, "  ... additional notes in JSON export")?; }
    }
    writeln!(output, "\nNeither a clean snapshot nor an active service proves application or Internet readiness.")?;
    pause(input, output)
}

fn export_guide<R: BufRead, W: Write>(
    input: &mut R, output: &mut W, clear: bool, cli: &Cli, demo_mode: bool,
) -> io::Result<Option<String>> {
    heading(output, clear, "Private report export")?;
    let cmd = if demo_mode { "demo" } else { "diagnose" };
    let boot = if cli.current_boot && !demo_mode { " --current-boot" } else { "" };
    writeln!(output, "WhyDown does not automatically save or transmit diagnostic data.")?;
    writeln!(output, "To export later, leave the TUI and run (review before sharing):\n")?;
    writeln!(output, "  whydown {cmd} --focus {} --since {} --max-events {}{boot} --format json --output whydown-report.json",
        cli.focus.as_str(), lookback(cli.since_seconds), cli.max_events)?;
    writeln!(output, "\nUse --format html and --output whydown-report.html for a standalone report.")?;
    writeln!(output, "Existing output files are protected unless --overwrite is explicitly supplied.")?;
    writeln!(output, "JSON and HTML include some local identifiers; no raw journal messages.")?;
    pause(input, output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn demo_menu_is_interactive_without_host_collection() {
        // Run, choose finding 1, inspect timeline, coverage, export hint and quit.
        let mut input = Cursor::new(b"1\n\n6\n1\n\n7\n\n8\n\n9\n\n0\n".as_slice());
        let mut output = Vec::new();
        let cli = Cli { tui_demo: true, ..Cli::default() };
        run_with_io(&mut input, &mut output, cli, false).unwrap();
        let transcript = String::from_utf8(output).unwrap();
        assert!(transcript.contains("DEMO (fictional)"));
        assert!(transcript.contains("Demo") || transcript.contains("fictional"));
        assert!(transcript.contains("Finding evidence"));
        assert!(transcript.contains("Collection coverage"));
        assert!(transcript.contains("whydown demo"));
        assert!(transcript.contains("TUI closed; no system changes made."));
        assert!(!transcript.contains("\x1b["));
    }

    #[test]
    fn changing_configuration_invalidates_previous_report() {
        let mut input = Cursor::new(b"1\n\n2\n6\n\n0\n".as_slice());
        let mut output = Vec::new();
        let cli = Cli { tui_demo: true, ..Cli::default() };
        run_with_io(&mut input, &mut output, cli, false).unwrap();
        assert!(String::from_utf8(output).unwrap().contains("Run a diagnostic first"));
    }

    #[test]
    fn terminal_evidence_is_sanitized() {
        assert_eq!(safe("USB\x1b[2J\nmount\r"), "USB [2J mount ");
        assert_eq!(lookback(7 * 86_400), "7d");
        assert_eq!(lookback(4 * 3_600), "4h");
    }

    #[test]
    fn eof_exits_cleanly_without_running_collectors() {
        let mut input = Cursor::new(Vec::<u8>::new());
        let mut output = Vec::new();
        run_with_io(&mut input, &mut output, Cli::default(), false).unwrap();
        assert!(String::from_utf8(output).unwrap().contains("TUI closed"));
    }
}
