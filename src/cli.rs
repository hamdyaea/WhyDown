use crate::model::Focus;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Diagnose,
    Demo,
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
    pub focus: Focus,
    pub since_seconds: u64,
    pub max_events: usize,
    pub format: OutputFormat,
    pub output: Option<PathBuf>,
    pub overwrite: bool,
    pub disk_details: bool,
}

impl Default for Cli {
    fn default() -> Self {
        Self {
            command: Command::Diagnose,
            focus: Focus::All,
            since_seconds: 86_400,
            max_events: 500,
            format: OutputFormat::Text,
            output: None,
            overwrite: false,
            disk_details: false,
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
            if flag == "--disk-details" {
                cli.disk_details = true;
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
  whydown --help

COMMANDS
  diagnose         Inspect the local Linux system (default)
  demo             Show a safe, deterministic example (no system inspection)
  help             Show this help
  version          Show program version

OPTIONS
  --focus VALUE    all | network | services | system (default: all)
  --since VALUE    journal lookback: 30m | 24h | 7d (default: 24h)
  --max-events N   maximum journal entries examined (default: 500)
  --format VALUE   text | json | html (default: text)
  --output FILE    write report to a new file (use --overwrite to replace)
  --overwrite      allow replacing an existing output file
  --disk-details   inspect large subdirectories on near-full filesystems (may take time)
  -h, --help       display help
  -V, --version    display version

EXAMPLES
  whydown diagnose --focus network --since 2h
  whydown diagnose --focus system --disk-details
  whydown diagnose --format json --output report.json
  whydown diagnose --format html --output report.html
  whydown demo --format html --output example.html

Read-only. Offline. No root required (journal access may be limited).
Reports are diagnostic hints, not proof of a root cause.
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults() {
        let c = Cli::parse(Vec::<String>::new()).unwrap();
        assert_eq!(c.command, Command::Diagnose);
        assert_eq!(c.since_seconds, 86_400);
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
    fn reject_unknown_or_unsafe_durations() {
        for value in ["0h", "1s", "31d", "999999999999999999999d", "-1d", "12h;rm"] {
            assert!(parse_duration(value).is_err(), "{value}");
        }
        assert!(Cli::parse(["--dangerous"]).is_err());
        assert!(Cli::parse(["--since"]).is_err());
    }
}
