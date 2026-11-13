//! Dependency-free offline SVG trend charts and HTML dashboard for expressly
//! recorded Nagios samples. No synthetic trends are generated.

use crate::nagios::{spec_for, Spec, State, SPECS};
use crate::time::utc_timestamp;
use std::collections::BTreeMap;
use std::fmt::Write as FmtWrite;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

const MAX_HISTORY_BYTES: u64 = 8 * 1024 * 1024;
const MAX_HISTORY_ROWS: usize = 60_000;
const MAX_SERIES_POINTS: usize = 1_200;

#[derive(Debug, Clone)]
pub struct Point {
    pub timestamp: u64,
    pub value: f64,
    pub state: State,
    pub demo: bool,
    pub warning: Option<f64>,
    pub critical: Option<f64>,
}
#[derive(Debug)]
struct Config {
    history: PathBuf,
    output: PathBuf,
    metric: Option<&'static str>,
    hours: u64,
    overwrite: bool,
}
fn parse_config(mode: &str, args: &[String]) -> Result<Config, String> {
    let mut history = None;
    let mut output = None;
    let mut metric = None;
    let mut hours = 24_u64;
    let mut overwrite = false;
    let mut i = 0;
    while let Some(arg) = args.get(i) {
        match arg.as_str() {
            "--overwrite" => overwrite = true,
            "--history" | "--output" | "--metric" | "--hours" => {
                i += 1;
                let value = args.get(i).ok_or_else(|| format!("{arg} requires a value"))?;
                match arg.as_str() {
                    "--history" => history = Some(PathBuf::from(value)),
                    "--output" => output = Some(PathBuf::from(value)),
                    "--metric" => {
                        metric = Some(spec_for(value).ok_or_else(|| format!(
                            "unknown metric for graph: {value}"))?.label);
                    }
                    "--hours" => {
                        hours = value.parse().map_err(|_| "hours must be an integer".to_owned())?;
                        if !(1..=8_760).contains(&hours) {
                            return Err("--hours must be 1..8760".into());
                        }
                    }
                    _ => unreachable!(),
                }
            }
            other => return Err(format!("unknown graph option: {other}")),
        }
        i += 1;
    }
    let history: PathBuf = history.ok_or("use --history FILE containing --record samples")?;
    let output: PathBuf = output.ok_or("use --output FILE.svg / FILE.html")?;
    if history.as_os_str().is_empty() || output.as_os_str().is_empty()
        || output.to_str() == Some("-") {
        return Err("history and output must be separate file paths".into());
    }
    if mode == "graph" && metric.is_none() {
        return Err("SVG graph needs --metric NAME (e.g. memory or mem_used_pct)".into());
    }
    if mode == "dashboard" && metric.is_some() {
        return Err("HTML dashboard includes every recorded metric; omit --metric".into());
    }
    if history == output || (history.canonicalize().is_ok()
        && output.canonicalize().ok() == history.canonicalize().ok()) {
        return Err("graph output must never overwrite the metric history".into());
    }
    Ok(Config { history, output, metric, hours, overwrite })
}

pub fn parse_history(text: &str) -> Result<BTreeMap<&'static str, Vec<Point>>, String> {
    let mut series = BTreeMap::<&'static str, Vec<Point>>::new();
    let mut rows = 0;
    for line in text.lines() {
        if line.trim().is_empty() || line.starts_with('#') { continue; }
        rows += 1;
        if rows > MAX_HISTORY_ROWS { return Err("metric history exceeds row safety limit".into()); }
        let fields: Vec<_> = line.split('\t').collect();
        if fields.len() != 5 && fields.len() != 7 { continue; }
        let Some(spec) = spec_for(fields[1]) else { continue; };
        // History labels must be canonical, not ambiguous CLI command names.
        if fields[1] != spec.label { continue; }
        let Ok(timestamp) = fields[0].parse::<u64>() else { continue; };
        if !(946_684_800..=4_102_444_800).contains(&timestamp) { continue; }
        let Ok(value) = fields[2].parse::<f64>() else { continue; };
        if !value.is_finite() || value < 0.0 || value > 100_000_000.0 { continue; }
        let state = match fields[3] {
            "OK" => State::Ok,
            "WARNING" => State::Warning,
            "CRITICAL" => State::Critical,
            "UNKNOWN" => State::Unknown,
            _ => continue,
        };
        let demo = match fields[4] {
            "demo" => true,
            "live" => false,
            _ => continue,
        };
        let bounded_threshold = |value: &str| value.parse::<f64>().ok()
            .filter(|v| v.is_finite() && *v >= 0.0 && *v < 100_000_000.0);
        let warning = if fields.len() == 7 {
            bounded_threshold(fields[5])
        } else { bounded_threshold(spec.warn) };
        let critical = if fields.len() == 7 {
            bounded_threshold(fields[6])
        } else { bounded_threshold(spec.crit) };
        series.entry(spec.label).or_default().push(Point {
            timestamp, value, state, demo, warning, critical
        });
    }
    for points in series.values_mut() {
        points.sort_by_key(|p| p.timestamp);
    }
    if series.is_empty() {
        return Err("no valid recorded measurements found (run nagios check --record FILE first)".into());
    }
    Ok(series)
}

fn load_history(path: &Path) -> Result<BTreeMap<&'static str, Vec<Point>>, String> {
    let meta = fs::metadata(path).map_err(|_| "metric history missing or inaccessible")?;
    if !meta.is_file() || meta.len() > MAX_HISTORY_BYTES {
        return Err("history must be a regular file no larger than 8 MiB".into());
    }
    let mut text = String::new();
    File::open(path).map_err(|_| "cannot open metric history")?
        .take(MAX_HISTORY_BYTES + 1).read_to_string(&mut text)
        .map_err(|_| "metric history cannot be read as UTF-8")?;
    if text.len() as u64 > MAX_HISTORY_BYTES {
        return Err("history changed and exceeded the 8 MiB safety limit".into());
    }
    parse_history(&text)
}

fn filter_points(points: &[Point], hours: u64) -> Vec<Point> {
    let Some(last) = points.last() else { return Vec::new(); };
    let min_time = last.timestamp.saturating_sub(hours * 3600);
    points.iter().filter(|p| p.timestamp >= min_time).cloned().collect()
}

/// Downsample by keeping extrema in each chronological bucket, so brief
/// spikes are not erased when drawing long histories.
fn bounded_points(points: &[Point]) -> Vec<Point> {
    if points.len() <= MAX_SERIES_POINTS { return points.to_vec(); }
    let divisor = MAX_SERIES_POINTS / 4;
    let bucket = (points.len() + divisor - 1) / divisor;
    let mut reduced = Vec::new();
    for chunk in points.chunks(bucket) {
        let min = chunk.iter().min_by(|a,b| a.value.total_cmp(&b.value)).unwrap();
        let max = chunk.iter().max_by(|a,b| a.value.total_cmp(&b.value)).unwrap();
        for candidate in [chunk.first().unwrap(), min, max, chunk.last().unwrap()] {
            if !reduced.iter().rev().take(4).any(|p: &Point|
                p.timestamp == candidate.timestamp && p.value == candidate.value) {
                reduced.push(candidate.clone());
            }
        }
    }
    reduced.sort_by_key(|p| p.timestamp);
    reduced
}

fn escaped(s: &str) -> String {
    let mut result = String::new();
    for c in s.chars() {
        match c {
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '"' => result.push_str("&quot;"),
            '\'' => result.push_str("&#39;"),
            _ if c.is_control() => {},
            _ => result.push(c),
        }
    }
    result
}

fn chart_svg(spec: Spec, source: &[Point]) -> String {
    // Charts never claim an interval when only one measurement exists.
    let points = bounded_points(source);
    let first = points.first().map_or(0, |p| p.timestamp);
    let last = points.last().map_or(first, |p| p.timestamp);
    let min_t = first;
    let span_t = last.saturating_sub(first).max(1);
    let peak = points.iter().fold(0.0_f64, |acc, p| acc.max(p.value));
    let max_y = spec.max.unwrap_or(0.0).max(peak * 1.12).max(2.0);
    let (left, top, right, bottom) = (66.0, 30.0, 860.0, 218.0);
    let width = right - left;
    let height = bottom - top;
    let x = |ts: u64| left + (ts.saturating_sub(min_t) as f64 / span_t as f64) * width;
    let y = |value: f64| bottom - (value.max(0.0) / max_y).min(1.0) * height;

    let mut svg = String::new();
    let _ = write!(svg, r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 910 292" role="img" aria-label="Trend chart for {}">"#, escaped(spec.label));
    svg.push_str(r##"<rect x="0" y="0" width="910" height="292" rx="16" fill="#152338"/>"##);
    for i in 0..=4 {
        let v = max_y * i as f64 / 4.0;
        let pos = y(v);
        let _ = write!(svg, r##"<path d="M{left:.1} {pos:.1} H{right:.1}" stroke="#314761" stroke-width="1"/>"##);
        let _ = write!(svg, r##"<text x="58" y="{:.1}" text-anchor="end" font-size="12" fill="#b0c6d8">{:.1}{}</text>"##,
            pos+4.0, v, escaped(spec.uom));
    }
    // Thin threshold guides are useful in standalone and Nagios graphs.
    let latest = points.last();
    for (threshold, color) in [
        (latest.and_then(|p| p.warning), "#f6ad55"),
        (latest.and_then(|p| p.critical), "#f87171"),
    ] {
        if let Some(v) = threshold {
            if v >= 0.0 && v <= max_y {
                let pos = y(v);
                let _ = write!(svg, r##"<path d="M{left:.1} {pos:.1} H{right:.1}" stroke="{color}" stroke-dasharray="7 5" stroke-width="1.5"/>"##);
            }
        }
    }
    if points.len() > 1 && first != last {
        let poly = points.iter().map(|p| format!("{:.2},{:.2}", x(p.timestamp), y(p.value)))
            .collect::<Vec<_>>().join(" ");
        let _ = write!(svg, r##"<polyline points="{poly}" fill="none" stroke="#37dfd0" stroke-width="3" stroke-linejoin="round" stroke-linecap="round"/>"##);
    }
    for p in points.iter().rev().take(1) {
        let _ = write!(svg, r##"<circle cx="{:.2}" cy="{:.2}" r="5" fill="#37dfd0" stroke="#fff" stroke-width="1.5"/>"##,
            x(p.timestamp), y(p.value));
    }
    let label_start = utc_timestamp(first);
    let label_end = utc_timestamp(last);
    let _ = write!(svg, r##"<text x="{left:.0}" y="250" font-size="12" fill="#b0c6d8">{}</text>"##, escaped(&label_start));
    let _ = write!(svg, r##"<text x="{right:.0}" y="250" text-anchor="end" font-size="12" fill="#b0c6d8">{}</text>"##, escaped(&label_end));
    let _ = write!(svg, r##"<text x="66" y="277" font-size="12" fill="#79e5dd">{} measurements; latest {:.2}{}</text>"##,
        source.len(), source.last().map_or(0.0, |p| p.value), escaped(spec.uom));
    if first == last {
        svg.push_str(r##"<text x="860" y="277" text-anchor="end" font-size="12" fill="#ffca85">One timestamp — no trend yet</text>"##);
    }
    if points.iter().any(|p| p.demo) {
        svg.push_str(r##"<text x="860" y="18" text-anchor="end" font-size="13" fill="#ffca85">DEMO DATA PRESENT</text>"##);
    }
    svg.push_str("</svg>");
    svg
}
fn html_dashboard(groups: &BTreeMap<&'static str, Vec<Point>>, hours: u64) -> Result<String, String> {
    let mut html = String::from(r#"<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>WhyDown Nagios Metrics</title><style>
:root{color-scheme:dark}*{box-sizing:border-box}body{margin:0;background:#0b1626;color:#edf6ff;font-family:system-ui,Arial,sans-serif}header{padding:42px max(22px,5vw) 28px;border-bottom:1px solid #26384d;background:#0a1524}h1{margin:0;font-size:2rem;letter-spacing:-.03em}h1 span{color:#30d6c7}p{line-height:1.55;color:#b9cbdc}.wrap{max-width:1500px;margin:auto;padding:30px max(22px,3vw) 55px}.grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(min(100%,480px),1fr));gap:22px}.panel{border:1px solid #2d405a;background:#142239;border-radius:18px;padding:22px;box-shadow:0 12px 24px #0003}.meta{display:flex;justify-content:space-between;gap:10px;align-items:center;flex-wrap:wrap}h2{margin:0 0 9px;font-size:1.1rem}.state{padding:6px 10px;border-radius:12px;font-size:12px;background:#1b564d;color:#a1f4de}.WARNING{background:#604525;color:#fbe0a3}.CRITICAL{background:#6b3441;color:#ffe0e0}.UNKNOWN{background:#4a4b65;color:#dce3ff}.metric{font-size:1.8rem;font-weight:700;margin:12px 0}.panel svg{max-width:100%;height:auto}small{color:#9eb4c9}.notice{margin-top:20px;padding:15px;border-left:3px solid #39dfd0;background:#142239;border-radius:4px}code{color:#8cebe1}</style></head><body><header><h1>Why<span>Down</span> &nbsp;·&nbsp; Nagios monitoring</h1><p>Offline dashboard from explicitly recorded local measurements. Historical values are samples, not continuously measured uptime or confirmed root causes.</p></header><main class="wrap"><div class="grid">"#);
    let mut cards = 0;
    let mut contains_demo = false;
    for spec in SPECS {
        let Some(series) = groups.get(spec.label) else { continue; };
        let points = filter_points(series, hours);
        let Some(last) = points.last() else { continue; };
        cards += 1;
        contains_demo |= points.iter().any(|p| p.demo);
        let state = last.state.label();
        let min = points.iter().fold(f64::INFINITY, |a,p| a.min(p.value));
        let max = points.iter().fold(f64::NEG_INFINITY, |a,p| a.max(p.value));
        let _ = write!(html, r#"<section class="panel"><div class="meta"><h2>{}</h2><span class="state {}">{}</span></div><p class="metric">{:.2}{}</p><small>min {:.2}, max {:.2} · {} samples · last {}</small>{}</section>"#,
            escaped(spec.label), state, state, last.value, escaped(spec.uom),
            min, max, points.len(), escaped(&utc_timestamp(last.timestamp)),
            chart_svg(*spec, &points));
    }
    html.push_str("</div>");
    if cards == 0 { return Err("no recorded metrics in the selected time window".into()); }
    if contains_demo {
        html.push_str(r#"<p class="notice"><strong>DEMO DATA PRESENT.</strong> Some graphed measurements were created with the fictional --demo flag. Do not interpret them as actual host observations.</p>"#);
    }
    let _ = write!(html, r#"<p class="notice">{} metric charts, last {} hours before the newest recorded sample. Data are stored locally only. Configure PNP4Nagios or another perfdata processor for centralized, continuously scheduled Nagios graphs.</p></main></body></html>"#, cards, hours);
    Ok(html)
}

fn write_private(path: &Path, bytes: &[u8], overwrite: bool) -> Result<(), String> {
    if !overwrite {
        let mut opts = OpenOptions::new();
        opts.write(true).create_new(true);
        #[cfg(unix)]
        opts.mode(0o600);
        let mut file = opts.open(path).map_err(|e| format!(
            "cannot create new output (use --overwrite to replace): {e}"))?;
        let result = file.write_all(bytes).and_then(|_| file.sync_all());
        if let Err(e) = result {
            drop(file);
            let _ = fs::remove_file(path);
            return Err(format!("cannot save graph: {e}"));
        }
        return Ok(());
    }
    let dir = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    for i in 0..16 {
        let tmp = dir.join(format!(".whydown-nagios-{}-{}.tmp", std::process::id(), i));
        let mut opts = OpenOptions::new();
        opts.write(true).create_new(true);
        #[cfg(unix)]
        opts.mode(0o600);
        let Ok(mut file) = opts.open(&tmp) else { continue; };
        let result = file.write_all(bytes)
            .and_then(|_| file.sync_all())
            .and_then(|_| fs::rename(&tmp, path));
        if let Err(e) = result {
            drop(file);
            let _ = fs::remove_file(&tmp);
            return Err(format!("cannot replace graph output: {e}"));
        }
        return Ok(());
    }
    Err("cannot allocate private graph staging file".into())
}

pub fn run(mode: &str, args: &[String]) -> Result<String, String> {
    let config = parse_config(mode, args)?;
    let groups = load_history(&config.history)?;
    let data = if mode == "graph" {
        let label = config.metric.ok_or("missing graph metric")?;
        let spec = spec_for(label).ok_or("invalid graph metric")?;
        let points = groups.get(spec.label).ok_or("requested metric has no recorded samples")?;
        let points = filter_points(points, config.hours);
        if points.is_empty() { return Err("no recorded samples in requested interval".into()); }
        chart_svg(spec, &points)
    } else {
        html_dashboard(&groups, config.hours)?
    };
    write_private(&config.output, data.as_bytes(), config.overwrite)?;
    Ok(config.output.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_recorded_history_and_render_real_time_series() {
        let fixture = "1794500000\tmem_used_pct\t65.0\tOK\tlive\n\
                       1794503600\tmem_used_pct\t91.0\tWARNING\tlive\n\
                       1794507200\tmem_used_pct\t96.0\tCRITICAL\tlive\n\
                       1794507200\tdns_servers\t1.0\tOK\tlive\n";
        let data = parse_history(fixture).unwrap();
        let points = data.get("mem_used_pct").unwrap();
        assert_eq!(points.len(), 3);
        assert_eq!(points[2].state, State::Critical);
        let graph = chart_svg(spec_for("memory").unwrap(), points);
        assert!(graph.contains("<polyline"));
        assert!(graph.contains("96.00%"));
        assert!(!graph.contains("17945"));
        let html = html_dashboard(&data, 24).unwrap();
        assert!(html.contains("Nagios monitoring"));
        assert!(html.contains("mem_used_pct"));
        assert!(html.contains("dns_servers"));
        assert!(!html.contains("DEMO DATA PRESENT"));
    }
    #[test]
    fn no_history_no_fake_graphs_and_escape_safely() {
        assert!(parse_history("malicious\tpath\tfoobar\n").is_err());
        assert!(!chart_svg(spec_for("memory").unwrap(),
            &[Point {timestamp:1794500000, value:48.0, state:State::Ok, demo:true, warning:Some(80.0), critical:Some(95.0)}])
            .contains("<polyline"));
        assert!(escaped("<script>&\"").contains("&lt;script&gt;&amp;&quot;"));
        let mut samples = parse_history(
            "1794500000\tmem_used_pct\t48\tOK\tdemo\n").unwrap();
        assert!(html_dashboard(&samples, 24).unwrap().contains("DEMO DATA PRESENT"));
        samples.clear();
        assert!(html_dashboard(&samples, 24).is_err());
    }
    #[test]
    fn guard_configuration_and_shared_history_file() {
        for args in [
            vec!["--history","same.tsv","--output","same.tsv","--metric","memory"],
            vec!["--history","x","--output","x.svg"],
            vec!["--history","x","--output","x.svg","--metric","fake"],
            vec!["--history","x","--output","x.svg","--metric","memory","--hours","0"],
        ] {
            let args: Vec<_> = args.iter().map(|s| s.to_string()).collect();
            assert!(parse_config("graph", &args).is_err());
        }
    }
}
