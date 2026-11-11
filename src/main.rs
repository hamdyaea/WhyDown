use std::fs::OpenOptions;
use std::io::{self, Write};
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use whydown::cli::{Cli, Command, OutputFormat, HELP};
use whydown::{analyze, collect, demo, render, time};

fn run() -> Result<(), String> {
    let cli = Cli::parse(std::env::args().skip(1)).map_err(|e| format!("{e}\nRun `whydown --help` for usage."))?;

    match cli.command {
        Command::Help => {
            print!("{HELP}");
            return Ok(());
        }
        Command::Version => {
            println!("whydown {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        _ => {}
    }

    if !cfg!(target_os = "linux") && cli.command == Command::Diagnose {
        return Err("Live diagnostics currently require Linux. Use `whydown demo` on other systems.".into());
    }

    let mut snapshot = match cli.command {
        Command::Demo => demo::snapshot(),
        _ => collect::collect(cli.focus, time::now_unix().saturating_sub(cli.since_seconds), cli.max_events, cli.disk_details),
    };
    if cli.command == Command::Demo {
        let since_unix = time::now_unix().saturating_sub(cli.since_seconds);
        snapshot.events.retain(|event| event.timestamp >= since_unix);
    }
    let report = analyze(snapshot, cli.since_seconds, cli.focus, cli.command == Command::Demo);
    let output = match cli.format {
        OutputFormat::Text => render::text(&report),
        OutputFormat::Json => render::json(&report),
        OutputFormat::Html => render::html(&report),
    };

    match cli.output {
        Some(path) if path.to_string_lossy() != "-" => {
            let mut options = OpenOptions::new();
            options.write(true);
            if cli.overwrite { options.create(true).truncate(true); }
            else { options.create_new(true); }
            #[cfg(unix)]
            options.mode(0o600);
            let mut file = options.open(&path)
                .map_err(|e| format!("Cannot open {}: {e} (use --overwrite to replace an existing file)", path.display()))?;
            file.write_all(output.as_bytes()).map_err(|e| format!("Cannot write {}: {e}", path.display()))?;
            eprintln!("Report saved to {}", path.display());
        }
        _ => {
            let mut stdout = io::stdout().lock();
            stdout.write_all(output.as_bytes()).map_err(|e| format!("Cannot write to stdout: {e}"))?;
        }
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("Error: {error}");
        std::process::exit(2);
    }
}
