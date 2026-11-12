use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::process;
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use whydown::cli::{Cli, Command, OutputFormat, HELP};
use whydown::model::{Focus, Severity};
use whydown::progress::Progress;
use whydown::{analyze, collect, demo, render, time};

fn run() -> Result<u8, String> {
    let cli = Cli::parse(std::env::args().skip(1)).map_err(|e| format!("{e}\nRun `whydown --help` for usage."))?;

    match cli.command {
        Command::Help => {
            print!("{HELP}");
            return Ok(0);
        }
        Command::Version => {
            println!("whydown {}", env!("CARGO_PKG_VERSION"));
            return Ok(0);
        }
        _ => {}
    }

    if !cfg!(target_os = "linux") && cli.command == Command::Diagnose {
        return Err("Live diagnostics currently require Linux. Use `whydown demo` on other systems.".into());
    }

    let stages = 3
        + usize::from(cli.focus.includes(Focus::Network))
        + usize::from(cli.focus.includes(Focus::Services))
        + usize::from(cli.focus.includes(Focus::System));
    let mut progress = Progress::new(cli.progress_mode, if cli.command == Command::Demo { 3 } else { stages });
    let mut snapshot = match cli.command {
        Command::Demo => {
            progress.begin("Preparing demo evidence");
            let snapshot = demo::snapshot();
            progress.complete_step();
            snapshot
        }
        _ => collect::collect_with_options(
            cli.focus, time::now_unix().saturating_sub(cli.since_seconds),
            cli.max_events, cli.disk_details, cli.current_boot,
            cli.include_remote_mounts, &progress),
    };
    if cli.command == Command::Demo {
        let since_unix = time::now_unix().saturating_sub(cli.since_seconds);
        snapshot.events.retain(|event| event.timestamp >= since_unix);
    }
    progress.begin("Analyzing diagnostic evidence");
    let report = analyze(snapshot, cli.since_seconds, cli.focus, cli.command == Command::Demo);
    progress.complete_step();
    progress.begin("Generating and saving report");
    let output = match cli.format {
        OutputFormat::Text => render::text(&report),
        OutputFormat::Json => render::json(&report),
        OutputFormat::Html => render::html(&report),
    };

    let should_fail = cli.exit_on_warning &&
        report.findings.iter().any(|f| f.severity == Severity::Warning);
    // A live progress redraw must not overlap with the report written to
    // stdout or with the "Report saved to" stderr notification.
    progress.prepare_for_output();
    match cli.output {
        Some(path) if path.to_string_lossy() != "-" => {
            save_report(&path, output.as_bytes(), cli.overwrite)?;
            eprintln!("Report saved to {}", path.display());
        }
        _ => {
            let mut stdout = io::stdout().lock();
            stdout.write_all(output.as_bytes())
                .map_err(|e| format!("Cannot write to stdout: {e}"))?;
        }
    }
    progress.complete_step();
    progress.finish();
    Ok(if should_fail { 1 } else { 0 })
}

/// Reports may contain local configuration: create mode 0600 files.
/// With --overwrite, stage and sync in the destination directory before
/// atomically replacing the old report. A failed write preserves the old file.
fn save_report(path: &Path, bytes: &[u8], overwrite: bool) -> Result<(), String> {
    if !overwrite {
        let mut opts = OpenOptions::new();
        opts.write(true).create_new(true);
        #[cfg(unix)]
        opts.mode(0o600);
        let mut file = opts.open(path).map_err(|e| format!(
            "Cannot create {}: {e} (use --overwrite to replace an existing file)",
            path.display()
        ))?;
        let result = file.write_all(bytes).and_then(|_| file.sync_all());
        if let Err(error) = result {
            drop(file);
            let _ = fs::remove_file(path);
            return Err(format!("Cannot write {}: {error}", path.display()));
        }
        return Ok(());
    }

    let parent = path.parent().filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut staged: Option<(std::path::PathBuf, File)> = None;
    for index in 0..32 {
        let candidate = parent.join(format!(".whydown-report-{}-{index}.tmp", process::id()));
        let mut opts = OpenOptions::new();
        opts.write(true).create_new(true);
        #[cfg(unix)]
        opts.mode(0o600);
        match opts.open(&candidate) {
            Ok(file) => { staged = Some((candidate, file)); break; }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(format!("Cannot stage {}: {error}", path.display())),
        }
    }
    let (temp_path, mut file) = staged.ok_or_else(||
        "Cannot find a free temporary report filename".to_string())?;
    let result = (|| -> io::Result<()> {
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temp_path, path)?;
        Ok(())
    })();
    if let Err(error) = result {
        let _ = fs::remove_file(&temp_path);
        return Err(format!("Cannot replace {}: {error}; previous report preserved", path.display()));
    }
    Ok(())
}

fn main() {
    match run() {
        Ok(0) => {}
        Ok(code) => process::exit(code as i32),
        Err(error) => {
            eprintln!("Error: {error}");
            process::exit(2);
        }
    }
}
