//! Nagios / Icinga-compatible on-demand checks with bounded local sampling.
//! Default checks are read-only, do not use the expensive diagnostic journal
//! collector, and do not perform network probes or create any files.
//!
//! Return codes: OK=0, WARNING=1, CRITICAL=2, UNKNOWN=3.
//! History is appended only with an explicit --record path.

use crate::{collect, health};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

pub const HELP: &str = r#"WhyDown Nagios/Icinga plugin and offline metric charts

USAGE
  whydown nagios check [--metric NAME] [--warning RANGE] [--critical RANGE]
                      [--path /mount] [--timeout SECONDS] [--record FILE]
                      [--verbose] [--demo]
  whydown nagios [--metric NAME] [OPTIONS]    # shorthand for check
  whydown nagios graph --history FILE --metric PERF_LABEL --output FILE.svg
                      [--hours 24] [--overwrite]
  whydown nagios dashboard --history FILE --output FILE.html
                          [--hours 24] [--overwrite]

CHECK METRICS
  all (default) | memory | swap | disk | inodes | load | cpu-psi
  memory-psi | io-psi | file-handles | route | dns | failed-units

NAGIOS PLUGIN CONTRACT
  0 OK | 1 WARNING | 2 CRITICAL | 3 UNKNOWN
  Output: WHYDOWN STATE - description | 'metric'=valueUOM;warn;crit;min;max
  Ranges: Nagios-compatible, e.g. 80, 0:90, 10:, ~:20, @10:20.
  --warning / --critical apply to one metric only, not --metric all.

EXAMPLES
  whydown nagios check --metric memory --warning 80 --critical 95
  whydown nagios check --metric disk --path / --warning 85 --critical 95
  whydown nagios check --metric failed-units
  whydown nagios check --metric all --timeout 8
  whydown nagios check --metric all --record ~/.local/state/whydown/nagios.tsv
  whydown nagios graph --history nagios.tsv --metric mem_used_pct --output memory.svg
  whydown nagios dashboard --history nagios.tsv --output dashboard.html
  whydown nagios check --demo

Graph history requires multiple scheduled checks with --record; a single
snapshot cannot establish a time series. Exports are offline SVG or HTML.
No package installation, escalation, service restart or telemetry occurs.
Read the full Nagios/Icinga guide at docs/NAGIOS.md.
"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State { Ok, Warning, Critical, Unknown }
impl State {
    pub fn code(self) -> u8 {
        match self { Self::Ok => 0, Self::Warning => 1, Self::Critical => 2, Self::Unknown => 3 }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Ok => "OK", Self::Warning => "WARNING",
            Self::Critical => "CRITICAL", Self::Unknown => "UNKNOWN",
        }
    }
    fn rank(self) -> u8 {
        match self { Self::Ok => 0, Self::Unknown => 1, Self::Warning => 2, Self::Critical => 3 }
    }
}

#[derive(Debug, Clone)]
pub struct Range {
    pub raw: String,
    start: f64,
    end: f64,
    alert_inside: bool,
}
impl Range {
    pub fn parse(raw: &str) -> Result<Self, String> {
        if raw.is_empty() || raw.len() > 48 || raw.chars().any(char::is_whitespace) {
            return Err("empty or invalid Nagios threshold range".into());
        }
        let (alert_inside, body) = if let Some(x) = raw.strip_prefix('@') {
            (true, x)
        } else { (false, raw) };
        let (start, end) = if let Some((left, right)) = body.split_once(':') {
            if right.contains(':') { return Err(format!("invalid range: {raw}")); }
            let start = match left {
                "" => 0.0,
                "~" => f64::NEG_INFINITY,
                v => parse_finite(v)?,
            };
            let end = if right.is_empty() { f64::INFINITY } else { parse_finite(right)? };
            (start, end)
        } else {
            (0.0, parse_finite(body)?)
        };
        if start > end { return Err(format!("range start exceeds end: {raw}")); }
        Ok(Self { raw: raw.into(), start, end, alert_inside })
    }
    pub fn triggered(&self, value: f64) -> bool {
        let inside = (self.start..=self.end).contains(&value);
        if self.alert_inside { inside } else { !inside }
    }
}
fn parse_finite(raw: &str) -> Result<f64, String> {
    let value: f64 = raw.parse().map_err(|_| format!("invalid number in range: {raw}"))?;
    if !value.is_finite() { return Err("threshold must be a finite number".into()); }
    Ok(value)
}

#[derive(Debug, Clone, Copy)]
pub struct Spec {
    pub choice: &'static str,
    pub label: &'static str,
    pub uom: &'static str,
    pub warn: &'static str,
    pub crit: &'static str,
    pub max: Option<f64>,
}
pub const SPECS: &[Spec] = &[
    Spec {choice:"memory", label:"mem_used_pct", uom:"%", warn:"80", crit:"95", max:Some(100.0)},
    Spec {choice:"swap", label:"swap_used_pct", uom:"%", warn:"70", crit:"90", max:Some(100.0)},
    Spec {choice:"disk", label:"disk_used_pct", uom:"%", warn:"80", crit:"90", max:Some(100.0)},
    Spec {choice:"inodes", label:"inode_used_pct", uom:"%", warn:"80", crit:"95", max:Some(100.0)},
    Spec {choice:"load", label:"load1_per_cpu_pct", uom:"%", warn:"85", crit:"150", max:None},
    Spec {choice:"cpu-psi", label:"cpu_psi_some_pct", uom:"%", warn:"50", crit:"90", max:Some(100.0)},
    Spec {choice:"memory-psi", label:"mem_psi_full_pct", uom:"%", warn:"5", crit:"15", max:Some(100.0)},
    Spec {choice:"io-psi", label:"io_psi_full_pct", uom:"%", warn:"10", crit:"20", max:Some(100.0)},
    Spec {choice:"file-handles", label:"fd_used_pct", uom:"%", warn:"80", crit:"95", max:Some(100.0)},
    // Range "~:" spans all finite values, so this default CRITICAL never
    // triggers for route/DNS absence. Missing configuration is a warning,
    // not proof that Internet/DNS actually failed.
    Spec {choice:"route", label:"default_route", uom:"", warn:"1:", crit:"~:", max:Some(1.0)},
    Spec {choice:"dns", label:"dns_servers", uom:"", warn:"1:", crit:"~:", max:None},
    Spec {choice:"failed-units", label:"failed_units", uom:"", warn:"0", crit:"0:2", max:None},
];
pub fn spec_for(choice: &str) -> Option<Spec> {
    SPECS.iter().find(|x| x.choice == choice || x.label == choice).copied()
}

#[derive(Debug)]
pub struct Options {
    pub metric: String,
    pub warning: Option<Range>,
    pub critical: Option<Range>,
    pub path: String,
    pub timeout: Duration,
    pub record: Option<PathBuf>,
    pub demo: bool,
    pub verbose: bool,
}
impl Default for Options {
    fn default() -> Self {
        Self { metric: "all".into(), warning: None, critical: None,
            path: "/".into(), timeout: Duration::from_secs(8),
            record: None, demo: false, verbose: false }
    }
}
pub fn parse_options(args: &[String]) -> Result<Options, String> {
    let mut o = Options::default();
    let mut i = 0;
    if args.first().is_some_and(|v| v == "check") { i += 1; }
    if let Some(value) = args.get(i) {
        if !value.starts_with('-') {
            o.metric = value.to_owned();
            i += 1;
        }
    }
    while let Some(arg) = args.get(i) {
        match arg.as_str() {
            "--demo" => o.demo = true,
            "--verbose" | "-v" => o.verbose = true,
            "--metric" | "--check" | "--warning" | "-w" | "--critical" | "-c" |
            "--path" | "--timeout" | "--record" => {
                i += 1;
                let value = args.get(i).ok_or_else(|| format!("{arg} requires a value"))?;
                match arg.as_str() {
                    "--metric" | "--check" => o.metric = value.clone(),
                    "--warning" | "-w" => o.warning = Some(Range::parse(value)?),
                    "--critical" | "-c" => o.critical = Some(Range::parse(value)?),
                    "--path" => o.path = value.clone(),
                    "--timeout" => {
                        let seconds: u64 = value.parse().map_err(|_| "timeout must be an integer".to_owned())?;
                        if !(1..=60).contains(&seconds) { return Err("timeout must be 1..60 seconds".into()); }
                        o.timeout = Duration::from_secs(seconds);
                    }
                    "--record" => o.record = Some(PathBuf::from(value)),
                    _ => unreachable!(),
                }
            }
            other => return Err(format!("unsupported Nagios check option: {other}")),
        }
        i += 1;
    }
    if o.metric != "all" && spec_for(&o.metric).is_none() {
        return Err(format!("unknown check metric: {}", o.metric));
    }
    if o.metric == "all" && (o.warning.is_some() || o.critical.is_some()) {
        return Err("custom thresholds require --metric NAME (not all)".into());
    }
    if !o.path.starts_with('/') || o.path.len() > 255 ||
        o.path.chars().any(char::is_control) {
        return Err("--path must be an absolute filesystem path without control characters".into());
    }
    if o.record.as_ref().is_some_and(|p| p.as_os_str().is_empty() || p.to_str() == Some("-")) {
        return Err("--record requires a real file path".into());
    }
    Ok(o)
}

#[derive(Debug, Clone)]
pub struct Sample {
    pub spec: Spec,
    pub value: Option<f64>,
    pub note: String,
    pub warning: Range,
    pub critical: Range,
}
impl Sample {
    pub fn state(&self) -> State {
        let Some(value) = self.value else { return State::Unknown; };
        if self.critical.triggered(value) { State::Critical }
        else if self.warning.triggered(value) { State::Warning }
        else { State::Ok }
    }
    pub fn perfdata(&self) -> String {
        let value = match self.value {
            Some(x) => format!("{x:.2}{}", self.spec.uom),
            None => "U".into(),
        };
        let max = self.spec.max.map(|v| format!("{v:.0}")).unwrap_or_default();
        format!("'{}'={};{};{};0;{}",
                self.spec.label, value, self.warning.raw, self.critical.raw, max)
    }
}

fn bounded_read(path: &str) -> Result<String, String> {
    let file = File::open(path).map_err(|_| format!("{path} unavailable"))?;
    let mut text = String::new();
    file.take(131_072).read_to_string(&mut text)
        .map_err(|_| format!("{path} unreadable"))?;
    Ok(text)
}
fn pct(numerator: f64, denominator: f64) -> Result<f64, String> {
    if denominator <= 0.0 || numerator < 0.0 || numerator > denominator {
        return Err("invalid kernel counter totals".into());
    }
    Ok(100.0 * numerator / denominator)
}

fn command_output(binary: &str, args: &[&str], remaining: Duration) -> Result<String, String> {
    let max = remaining.min(Duration::from_millis(1800));
    if max.is_zero() { return Err(format!("{binary} not run: Nagios timeout exhausted")); }
    let mut child = Command::new(binary).args(args)
        .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null())
        .spawn().map_err(|_| format!("{binary} is unavailable"))?;
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                let output = child.wait_with_output()
                    .map_err(|_| format!("cannot read {binary} output"))?;
                if !output.status.success() { return Err(format!("{binary} returned an error")); }
                if output.stdout.len() > 131_072 { return Err(format!("{binary} output too large")); }
                return String::from_utf8(output.stdout)
                    .map_err(|_| format!("{binary} returned non-UTF8 data"));
            }
            Ok(None) if start.elapsed() < max => thread::sleep(Duration::from_millis(20)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("{binary} check exceeded bounded timeout"));
            }
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("{binary} status unavailable"));
            }
        }
    }
}
fn df_percent(path: &str, inodes: bool, remaining: Duration) -> Result<f64, String> {
    let args = if inodes {
        vec!["-P", "-i", "-l", "--", path]
    } else {
        vec!["-P", "-k", "-l", "--", path]
    };
    let text = command_output("df", &args, remaining)?;
    let line = text.lines().skip(1).find(|v| !v.trim().is_empty())
        .ok_or_else(|| "df returned no filesystem observation".to_string())?;
    let cols: Vec<_> = line.split_whitespace().collect();
    if cols.len() < 6 { return Err("df returned incomplete data".into()); }
    let used = cols[4].strip_suffix('%').ok_or("df percentage missing")?
        .parse::<f64>().map_err(|_| "df usage percentage invalid")?;
    if !(0.0..=100.0).contains(&used) { return Err("df percentage out of bounds".into()); }
    Ok(used)
}

fn sample_value(choice: &str, options: &Options, remaining: Duration)
    -> Result<(f64, String), String>
{
    match choice {
        "memory" | "swap" => {
            let m = health::parse_meminfo(&bounded_read("/proc/meminfo")?)
                .ok_or("MemAvailable or MemTotal unavailable")?;
            if choice == "memory" {
                Ok((pct((m.total_kib - m.available_kib) as f64, m.total_kib as f64)?,
                    format!("MemAvailable={} MiB, MemTotal={} MiB (includes reclaimable cache)",
                        m.available_kib / 1024, m.total_kib / 1024)))
            } else {
                let total = m.swap_total_kib.ok_or("SwapTotal unavailable")?;
                let free = m.swap_free_kib.ok_or("SwapFree unavailable")?;
                if free > total { return Err("swap counters inconsistent".into()); }
                let used = if total == 0 { 0.0 } else { pct((total-free) as f64, total as f64)? };
                Ok((used, if total == 0 { "No swap configured; used swap = 0".into() }
                    else { format!("Swap used {} MiB of {} MiB", (total-free)/1024, total/1024) }))
            }
        }
        "disk" | "inodes" => {
            let used = df_percent(&options.path, choice == "inodes", remaining)?;
            Ok((used, format!("Local filesystem at {}: {:.0}% used; remote mounts excluded",
                options.path, used)))
        }
        "load" => {
            let load = bounded_read("/proc/loadavg")?;
            let load1 = load.split_whitespace().next()
                .ok_or("loadavg unavailable")?
                .parse::<f64>().map_err(|_| "loadavg invalid")?;
            let stat = bounded_read("/proc/stat")?;
            let cpus = stat.lines().filter(|line| line.starts_with("cpu")
                && line.as_bytes().get(3).is_some_and(u8::is_ascii_digit)).count();
            if cpus == 0 || !load1.is_finite() || load1 < 0.0 {
                return Err("CPU count or loadavg invalid".into());
            }
            Ok((load1 / cpus as f64 * 100.0,
                format!("1-minute load average {:.2} across {} logical CPUs (load/core ratio, not CPU utilization)",
                    load1, cpus)))
        }
        "cpu-psi" | "memory-psi" | "io-psi" => {
            let resource = if choice == "cpu-psi" { "cpu" }
                else if choice == "memory-psi" { "memory" } else { "io" };
            let path = format!("/proc/pressure/{resource}");
            let text = bounded_read(&path)?;
            let p = health::parse_pressure(resource, &text)
                .ok_or_else(|| format!("{resource} PSI unavailable"))?;
            let value = if choice == "cpu-psi" { p.some_avg60_x100 }
                else { p.full_avg60_x100 }
                .ok_or_else(|| format!("{resource} PSI avg60 unavailable"))?;
            Ok((value as f64 / 100.0, format!("{resource} pressure-stall avg60 over 60 seconds")))
        }
        "file-handles" => {
            let h = health::parse_file_nr(&bounded_read("/proc/sys/fs/file-nr")?)
                .ok_or("file-nr unavailable")?;
            let used = h.allocated - h.unused;
            Ok((pct(used as f64, h.maximum as f64)?,
                format!("{used} allocated handles in use of {} maximum (system-wide)", h.maximum)))
        }
        "route" => {
            let v4 = bounded_read("/proc/net/route").ok();
            let v6 = bounded_read("/proc/net/ipv6_route").ok();
            if v4.is_none() && v6.is_none() { return Err("IPv4/IPv6 route tables unavailable".into()); }
            let found = v4.as_ref().is_some_and(|v| collect::parse_ipv4_default_route(v))
                || v6.as_ref().is_some_and(|v| collect::parse_ipv6_default_route(v));
            Ok((if found { 1.0 } else { 0.0 },
                "Default route present in accessible local routing tables; external reachability NOT tested".into()))
        }
        "dns" => {
            let (count, _) = collect::parse_resolv_conf(&bounded_read("/etc/resolv.conf")?);
            Ok((count as f64,
                "Valid literal nameserver entries in /etc/resolv.conf; resolver health NOT tested".into()))
        }
        "failed-units" => {
            let text = command_output("systemctl", &[
                "--failed", "--no-pager", "--no-legend", "--plain"
            ], remaining)?;
            let n = text.lines().filter(|line| {
                let mut parts = line.split_whitespace();
                let first = parts.next().unwrap_or("");
                let name = if first == "●" { parts.next().unwrap_or("") } else { first };
                [".service",".mount",".socket",".timer",".target",".path",".swap"]
                    .iter().any(|suffix| name.ends_with(suffix))
            }).count();
            Ok((n as f64,
                "Failed units from systemd at check time; historical failures not included".into()))
        }
        _ => Err("unsupported metric".into()),
    }
}

fn demo_value(choice: &str) -> (f64, &'static str) {
    match choice {
        "memory" => (62.5, "fictional MemAvailable, no host inspection"),
        "swap" => (18.0, "fictional swap usage"),
        "disk" => (88.0, "fictional local disk at 88%"),
        "inodes" => (34.0, "fictional inode usage"),
        "load" => (105.0, "fictional one-minute load per logical CPU"),
        "cpu-psi" => (24.0, "fictional CPU stall pressure"),
        "memory-psi" => (1.2, "fictional memory full-stall pressure"),
        "io-psi" => (2.0, "fictional I/O full-stall pressure"),
        "file-handles" => (12.0, "fictional handle use"),
        "route" => (1.0, "fictional default route present"),
        "dns" => (2.0, "fictional configured resolvers"),
        "failed-units" => (1.0, "fictional one failed systemd unit"),
        _ => (0.0, "fictional"),
    }
}

pub fn collect_samples(o: &Options) -> Vec<Sample> {
    let deadline = Instant::now() + o.timeout;
    let specs: Vec<_> = if o.metric == "all" {
        SPECS.to_vec()
    } else {
        spec_for(&o.metric).into_iter().collect()
    };
    specs.into_iter().map(|spec| {
        let warning = o.warning.clone().unwrap_or_else(||
            Range::parse(spec.warn).expect("constant threshold"));
        let critical = o.critical.clone().unwrap_or_else(||
            Range::parse(spec.crit).expect("constant threshold"));
        let result = if o.demo {
            let (value, note) = demo_value(spec.choice);
            Ok((value, note.to_string()))
        } else if Instant::now() >= deadline {
            Err("global check deadline exhausted".into())
        } else {
            sample_value(spec.choice, o, deadline.saturating_duration_since(Instant::now()))
        };
        let (value, note) = match result {
            Ok((v, note)) if v.is_finite() => (Some(v), note),
            Ok(_) => (None, "sensor returned nonfinite value".into()),
            Err(note) => (None, note),
        };
        Sample { spec, value, note, warning, critical }
    }).collect()
}
fn overall(samples: &[Sample]) -> State {
    samples.iter().map(Sample::state)
        .max_by_key(|state| state.rank()).unwrap_or(State::Unknown)
}

pub fn append_history(path: &Path, samples: &[Sample], demo: bool) -> Result<(), String> {
    if let Ok(meta) = fs::symlink_metadata(path) {
        if !meta.file_type().is_file() { return Err("history path is not a regular file".into()); }
        #[cfg(unix)]
        if meta.permissions().mode() & 0o077 != 0 {
            return Err("existing history is not private (require mode 0600 or stricter)".into());
        }
    }
    let epoch = SystemTime::now().duration_since(UNIX_EPOCH)
        .map_err(|_| "system clock is before epoch")?.as_secs();
    let mut rows = String::new();
    let origin = if demo { "demo" } else { "live" };
    for sample in samples {
        if let Some(value) = sample.value {
            rows.push_str(&format!("{}\t{}\t{:.4}\t{}\t{}\n",
                epoch, sample.spec.label, value, sample.state().label(), origin));
        }
    }
    if rows.is_empty() { return Err("no measured values to record".into()); }
    let mut options = OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options.open(path).map_err(|e| format!("cannot open private history: {e}"))?;
    file.write_all(rows.as_bytes()).map_err(|e| format!("cannot append history: {e}"))?;
    file.flush().map_err(|e| format!("cannot flush history: {e}"))?;
    Ok(())
}

pub fn run(args: &[String]) -> u8 {
    if args.iter().any(|a| a == "-h" || a == "--help" || a == "help") {
        println!("{HELP}");
        return 0;
    }
    if let Some(first) = args.first() {
        if first == "graph" || first == "dashboard" {
            return match crate::nagios_graph::run(first, &args[1..]) {
                Ok(path) => { println!("WhyDown {} exported: {}", first, path); 0 }
                Err(e) => { println!("WHYDOWN UNKNOWN - {}", e.replace(['\r','\n'], " ")); 3 }
            };
        }
    }
    let options = match parse_options(args) {
        Ok(opts) => opts,
        Err(e) => { println!("WHYDOWN UNKNOWN - {e}"); return 3; }
    };
    if !cfg!(target_os = "linux") && !options.demo {
        println!("WHYDOWN UNKNOWN - local Nagios checks currently require Linux");
        return 3;
    }
    let samples = collect_samples(&options);
    if let Some(path) = &options.record {
        if let Err(e) = append_history(path, &samples, options.demo) {
            println!("WHYDOWN UNKNOWN - could not record private metric history: {e}");
            return 3;
        }
    }
    let result = overall(&samples);
    let detail = if samples.len() == 1 {
        let s = &samples[0];
        match s.value {
            Some(v) => format!("{} {:.2}{} ({})", s.spec.choice, v, s.spec.uom, s.note),
            None => format!("{} unavailable ({})", s.spec.choice, s.note),
        }
    } else {
        let (mut ok, mut warn, mut crit, mut unk) = (0, 0, 0, 0);
        for s in &samples {
            match s.state() {
                State::Ok => ok += 1, State::Warning => warn += 1,
                State::Critical => crit += 1, State::Unknown => unk += 1,
            }
        }
        format!("{ok} OK, {warn} WARNING, {crit} CRITICAL, {unk} UNKNOWN ({} local checks)",
            samples.len())
    };
    let perf = samples.iter().map(Sample::perfdata).collect::<Vec<_>>().join(" ");
    println!("WHYDOWN {}{} - {} | {}", result.label(),
        if options.demo { " DEMO" } else { "" }, detail.replace(['\r','\n'], " "), perf);
    if options.verbose {
        for s in &samples {
            println!("{}: {} - {}", s.spec.label, s.state().label(),
                s.note.replace(['\r','\n'], " "));
        }
        println!("Values are point-in-time local observations; no connectivity tests or persistent monitoring.");
    }
    result.code()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nagios_ranges_follow_inside_outside_semantics() {
        let r = Range::parse("10").unwrap();
        assert!(!r.triggered(10.0));
        assert!(r.triggered(10.001));
        assert!(r.triggered(-1.0));
        let r = Range::parse("@10:20").unwrap();
        assert!(r.triggered(10.0));
        assert!(r.triggered(20.0));
        assert!(!r.triggered(9.0));
        assert!(Range::parse("~:5").unwrap().triggered(6.0));
        assert!(!Range::parse("~:").unwrap().triggered(1.0));
        assert!(Range::parse("10:").unwrap().triggered(9.99));
        for bad in ["", "NaN", "inf", "2:1", "a", "2:3:4", "@", "~:NaN", "1e999"] {
            assert!(Range::parse(bad).is_err(), "{bad}");
        }
    }
    #[test]
    fn option_validation_avoids_ambiguous_or_unsafe_invocations() {
        let input = ["check","--metric","memory","-w","0:75","-c","@96:100","--timeout","9"];
        let opts = parse_options(&input.map(str::to_owned)).unwrap();
        assert_eq!(opts.metric, "memory");
        assert_eq!(opts.timeout, Duration::from_secs(9));
        assert!(opts.critical.unwrap().triggered(99.0));
        for bad in [
            vec!["--metric","all","--warning","80"],
            vec!["--metric","unknown"],
            vec!["--metric","disk","--path","../../etc"],
            vec!["--metric","disk","--timeout","0"],
            vec!["--record","-"],
            vec!["--shell"],
        ] {
            assert!(parse_options(&bad.iter().map(|s| s.to_string()).collect::<Vec<_>>()).is_err(),
                "{bad:?}");
        }
    }
    #[test]
    fn demo_metrics_have_valid_status_perfdata_and_nonzero_states() {
        let mut opts = Options { demo: true, ..Options::default() };
        let all = collect_samples(&opts);
        assert_eq!(all.len(), SPECS.len());
        assert_eq!(overall(&all), State::Warning);
        assert!(all.iter().all(|s| s.value.is_some() && !s.perfdata().contains('\n')));
        assert!(all.iter().any(|s| s.perfdata().starts_with("'mem_used_pct'=62.50%;80;95;0;100")));
        opts.metric = "disk".into();
        opts.critical = Some(Range::parse("80").unwrap());
        assert_eq!(overall(&collect_samples(&opts)), State::Critical);
    }
    #[test]
    fn unknown_sensor_never_implies_zero_or_healthy() {
        let spec = spec_for("memory").unwrap();
        let sample = Sample {
            spec, value: None, note: "not readable".into(),
            warning: Range::parse(spec.warn).unwrap(),
            critical: Range::parse(spec.crit).unwrap(),
        };
        assert_eq!(sample.state(), State::Unknown);
        assert!(sample.perfdata().contains("=U;"));
        assert_eq!(overall(&[sample]), State::Unknown);
    }
    #[test]
    fn parse_df_usage_rejects_invalid_lines() {
        assert!(pct(200.0, 100.0).is_err());
        assert!(pct(0.0, 0.0).is_err());
        assert!((pct(5.0, 10.0).unwrap() - 50.0).abs() < 0.01);
    }
}
