use crate::model::Focus;
use crate::progress::ProgressMode;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Diagnose,
    Demo,
    Tui,
    Gui,
    Help,
    Version,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Text,
    Json,
    Html,
}

impl OutputFormat {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "text" => Some(Self::Text),
            "json" => Some(Self::Json),
            "html" => Some(Self::Html),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Cli {
    pub command: Command,
    /// For `tui --demo` or `gui --demo`: fictional evidence, no host collection.
    pub tui_demo: bool,
    pub focus: Focus,
    pub since_seconds: u64,
    pub max_events: usize,
    pub format: OutputFormat,
    pub output: Option<PathBuf>,
    pub overwrite: bool,
    pub disk_details: bool,
    /// Opt in to remote filesystems and known network-backed FUSE types.
    pub include_remote_mounts: bool,
    /// Restrict journal observations to the currently running Linux boot.
    pub current_boot: bool,
    /// For automation: exit with status 1 when a warning is present.
    pub exit_on_warning: bool,
    pub progress_mode: ProgressMode,
}

impl Default for Cli {
    fn default() -> Self {
        Self {
            command: Command::Diagnose,
            tui_demo: false,
            focus: Focus::All,
            since_seconds: 86_400,
            max_events: 5_000,
            format: OutputFormat::Text,
            output: None,
            overwrite: false,
            disk_details: false,
            include_remote_mounts: false,
            current_boot: false,
            exit_on_warning: false,
            progress_mode: ProgressMode::Auto,
        }
    }
}

impl Cli {
    pub fn parse<I, S>(args: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let args: Vec<String> = args.into_iter().map(|a| a.as_ref().to_string()).collect();
        let mut cli = Self::default();
        let mut i = 0;
        if let Some(first) = args.first() {
            if !first.starts_with('-') {
                cli.command = match first.as_str() {
                    "diagnose" | "doctor" => Command::Diagnose,
                    "demo" => Command::Demo,
                    "tui" => Command::Tui,
                    "gui" => Command::Gui,
                    "help" => Command::Help,
                    "version" => Command::Version,
                    _ => return Err(format!("Unknown command: {first}")),
                };
                i += 1;
            }
        }

        while i < args.len() {
            let flag = args[i].as_str();
            if flag == "--help" || flag == "-h" {
                cli.command = Command::Help;
                return Ok(cli);
            }
            if flag == "--version" || flag == "-V" {
                cli.command = Command::Version;
                return Ok(cli);
            }
            if flag == "--demo" {
                if !matches!(cli.command, Command::Tui | Command::Gui) {
                    return Err("--demo is supported only by 'whydown tui' or 'whydown gui'; use 'whydown demo' otherwise".into());
                }
                cli.tui_demo = true;
                i += 1;
                continue;
            }
            if matches!(cli.command, Command::Tui | Command::Gui) &&
                matches!(flag, "--format" | "--output" | "--overwrite" | "--exit-on-warning" | "--progress" | "--no-progress") {
                return Err(format!("{flag} is not supported in interactive mode; use 'whydown diagnose' or 'whydown demo' for reports/automation"));
            }
            if flag == "--disk-details" {
                cli.disk_details = true;
                i += 1;
                continue;
            }
            if flag == "--include-remote-mounts" {
                cli.include_remote_mounts = true;
                i += 1;
                continue;
            }
            if flag == "--current-boot" || flag == "--since-boot" {
                cli.current_boot = true;
                i += 1;
                continue;
            }
            if flag == "--exit-on-warning" {
                cli.exit_on_warning = true;
                i += 1;
                continue;
            }
            if flag == "--progress" {
                cli.progress_mode = ProgressMode::Always;
                i += 1;
                continue;
            }
            if flag == "--no-progress" {
                cli.progress_mode = ProgressMode::Never;
                i += 1;
                continue;
            }
            if flag == "--overwrite" {
                cli.overwrite = true;
                i += 1;
                continue;
            }
            i += 1;
            let value = args.get(i).ok_or_else(|| format!("Missing value for {flag}"))?;
            match flag {
                "--focus" => {
                    cli.focus = Focus::parse(value)
                        .ok_or_else(|| format!("Invalid focus: {value} (use all, network, services, system)"))?;
                }
                "--since" => cli.since_seconds = parse_duration(value)?,
                "--max-events" => {
                    let n: usize = value.parse().map_err(|_| "--max-events must be a number".to_string())?;
                    if !(1..=10_000).contains(&n) {
                        return Err("--max-events must be between 1 and 10000".to_string());
                    }
                    cli.max_events = n;
                }
                "--format" => {
                    cli.format = OutputFormat::parse(value)
                        .ok_or_else(|| "--format must be text, json, or html".to_string())?;
                }
                "--output" => cli.output = Some(PathBuf::from(value)),
                _ => return Err(format!("Unknown option: {flag}")),
            }
            i += 1;
        }

        // The interactive views start with a smaller, current-boot-first
        // diagnostic window and the full 10k journal cap. The command-line
        // diagnostic defaults remain unchanged for scripts and automation.
        if matches!(cli.command, Command::Tui | Command::Gui) {
            if !args.iter().any(|arg| arg == "--since") {
                cli.since_seconds = 4 * 3_600;
            }
            if !args.iter().any(|arg| arg == "--max-events") {
                cli.max_events = 10_000;
            }
            if !args.iter().any(|arg| arg == "--current-boot" || arg == "--since-boot") {
                cli.current_boot = true;
            }
        }
        Ok(cli)
    }
}

/// Accept strictly 1..=30 days, expressed in m, h, or d.
pub fn parse_duration(s: &str) -> Result<u64, String> {
    if s.len() < 2 {
        return Err("Duration must look like 30m, 24h, or 7d".to_string());
    }
    let (digits, suffix) = s.split_at(s.len() - 1);
    if digits.is_empty() || !digits.bytes().all(|c| c.is_ascii_digit()) {
        return Err("Duration must look like 30m, 24h, or 7d".to_string());
    }
    let number: u64 = digits.parse().map_err(|_| "Duration too large".to_string())?;
    let factor: u64 = match suffix {
        "m" => 60,
        "h" => 3_600,
        "d" => 86_400,
        _ => return Err("Duration unit must be m, h, or d".to_string()),
    };
    let seconds = number.checked_mul(factor).ok_or_else(|| "Duration too large".to_string())?;
    if !(60..=30 * 86_400).contains(&seconds) {
        return Err("Duration must be from 1 minute to 30 days".to_string());
    }
    Ok(seconds)
}

pub const HELP: &str = r#"WhyDown — evidence-based Linux troubleshooting

USAGE
  whydown [diagnose] [OPTIONS]
  whydown demo [OPTIONS]
  whydown tui [OPTIONS]
  whydown tui --demo
  whydown gui [OPTIONS]
  whydown gui --demo
  whydown --help

COMMANDS
  diagnose         Inspect the local Linux system (default)
  demo             Show a safe, deterministic example (no system inspection)
  tui              Guided terminal evidence browser (requires a terminal)
  gui              Desktop GUI (Python 3 + Tkinter, no extra Rust crates)
  help             Show this help
  version          Show program version

OPTIONS
  --demo           TUI/GUI: browse fictional evidence without host inspection
  --focus VALUE    all | network | services | system (default: all)
  --since VALUE    journal lookback: 30m | 24h | 7d (default: 24h)
  --max-events N   maximum journal entries examined (default: 5000; max: 10000)
  --current-boot   restrict journal observations to the current boot
  --exit-on-warning  exit with code 1 if any WARNING was found (default: 0)
  --progress       force live diagnostic progress on stderr (even without TTY)
  --no-progress    suppress live diagnostic progress (auto on a terminal)
  --format VALUE   text | json | html (default: text)
  --output FILE    write report to a new file (use --overwrite to replace)
  --overwrite      allow replacing an existing output file
  --disk-details   inspect large subdirectories on near-full filesystems (may take time)
  --include-remote-mounts  include remote/FUSE mounts in disk checks (may hang)
  -h, --help       display help
  -V, --version    display version

EXAMPLES
  whydown diagnose --focus network --since 2h --current-boot
  whydown diagnose --focus system --exit-on-warning
  whydown diagnose --focus all --current-boot --progress
  whydown diagnose --focus system --disk-details
  whydown diagnose --format json --output report.json
  whydown diagnose --format html --output report.html
  whydown demo --format html --output example.html
  whydown tui --focus all --since 4h --current-boot
  whydown tui --demo
  whydown gui                     # all checks / 4h / current boot / 10000 entries
  whydown gui --demo              # fictional GUI, no host inspection

Read-only. Offline. No root required (journal access may be limited).
Reports are diagnostic hints, not proof of a root cause.
Exit codes: 0 = command completed, 1 = warning found with --exit-on-warning,
2 = invocation or output error. Collection limitations do not mean healthy.
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tui_parsing_and_output_safety() {
        let tui = Cli::parse(["tui", "--demo", "--focus", "services", "--since", "4h",
            "--current-boot", "--max-events", "10000"]).unwrap();
        assert_eq!(tui.command, Command::Tui);
        assert!(tui.tui_demo);
        assert_eq!(tui.focus, Focus::Services);
        assert_eq!(tui.since_seconds, 14_400);
        assert!(tui.current_boot);
        assert_eq!(tui.max_events, 10_000);
        assert!(Cli::parse(["diagnose", "--demo"]).is_err());
        for invalid in ["--format", "--output", "--overwrite", "--progress",
            "--no-progress", "--exit-on-warning"] {
            let args = if invalid == "--format" {
                vec!["tui", "--format", "json"]
            } else if invalid == "--output" {
                vec!["tui", "--output", "report.json"]
            } else {
                vec!["tui", invalid]
            };
            assert!(Cli::parse(args).is_err(), "{invalid}");
        }
    }

    #[test]
    fn interactive_defaults_are_guided_but_cli_automation_is_unchanged() {
        for command in ["tui", "gui"] {
            let cli = Cli::parse([command]).unwrap();
            assert_eq!(cli.focus, Focus::All);
            assert_eq!(cli.since_seconds, 14_400);
            assert_eq!(cli.max_events, 10_000);
            assert!(cli.current_boot);
        }
        let explicit = Cli::parse(["gui", "--demo", "--focus", "network",
            "--since", "90m", "--max-events", "5000"]).unwrap();
        assert_eq!(explicit.command, Command::Gui);
        assert!(explicit.tui_demo);
        assert_eq!(explicit.focus, Focus::Network);
        assert_eq!(explicit.since_seconds, 5_400);
        assert_eq!(explicit.max_events, 5_000);
        for bad in ["--format", "--output", "--overwrite", "--exit-on-warning"] {
            let args = if bad == "--format" { vec!["gui", bad, "json"] }
                else if bad == "--output" { vec!["gui", bad, "test.json"] }
                else { vec!["gui", bad] };
            assert!(Cli::parse(args).is_err(), "flag {bad}");
        }
        assert_eq!(Cli::parse(["diagnose"]).unwrap().max_events, 5_000);
    }

    #[test]
    fn defaults() {
        let c = Cli::parse(Vec::<String>::new()).unwrap();
        assert_eq!(c.command, Command::Diagnose);
        assert_eq!(c.since_seconds, 86_400);
        assert_eq!(c.max_events, 5_000);
    }

    #[test]
    fn custom_arguments() {
        let c = Cli::parse(["demo", "--format", "json", "--focus", "network", "--since", "90m"]).unwrap();
        assert_eq!(c.command, Command::Demo);
        assert_eq!(c.format, OutputFormat::Json);
        assert_eq!(c.focus, Focus::Network);
        assert_eq!(c.since_seconds, 5400);
    }

    #[test]
    fn journal_scope_and_monitoring_exit_flags() {
        let c = Cli::parse(["diagnose", "--current-boot", "--exit-on-warning"]).unwrap();
        assert!(c.current_boot);
        assert!(c.exit_on_warning);
        let alias = Cli::parse(["--since-boot"]).unwrap();
        assert!(alias.current_boot);
        let defaults = Cli::default();
        assert!(!defaults.current_boot);
        assert!(!defaults.exit_on_warning);
    }

    #[test]
    fn remote_mounts_are_opt_in() {
        assert!(!Cli::default().include_remote_mounts);
        assert!(Cli::parse(["--include-remote-mounts"]).unwrap().include_remote_mounts);
    }

    #[test]
    fn progress_is_auto_by_default_and_flags_toggle_it() {
        assert_eq!(Cli::default().progress_mode, ProgressMode::Auto);
        assert_eq!(Cli::parse(["diagnose", "--progress"]).unwrap().progress_mode, ProgressMode::Always);
        assert_eq!(Cli::parse(["diagnose", "--no-progress"]).unwrap().progress_mode, ProgressMode::Never);
        assert_eq!(Cli::parse(["--progress", "--no-progress"]).unwrap().progress_mode, ProgressMode::Never);
    }

    #[test]
    fn reject_unknown_or_unsafe_durations() {
        for value in ["0h", "1s", "31d", "999999999999999999999d", "-1d", "12h;rm"] {
            assert!(parse_duration(value).is_err(), "{value}");
        }
        assert!(Cli::parse(["--dangerous"]).is_err());
        assert!(Cli::parse(["--since"]).is_err());
    }
}
