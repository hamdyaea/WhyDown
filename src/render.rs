//! Text, JSON and HTML outputs. No runtime third-party dependencies.

use crate::model::{Finding, Report, Severity};
use crate::time::utc_timestamp;
use std::fmt::Write as _;

pub fn text(report: &Report) -> String {
    let mut out = String::new();
    let demo = if report.demo { " [DEMO]" } else { "" };
    writeln!(out, "WhyDown {}{demo}", report.version).unwrap();
    writeln!(out, "{}", "=".repeat(68)).unwrap();
    writeln!(out, "Generated : {}", utc_timestamp(report.generated_at)).unwrap();
    writeln!(out, "System    : {}", report.snapshot.os).unwrap();
    writeln!(out, "Kernel    : {}", report.snapshot.kernel).unwrap();
    writeln!(out, "Focus     : {}", report.focus).unwrap();
    writeln!(out, "Journal    : last {} minute(s), newest entries only", report.lookback_seconds / 60).unwrap();
    writeln!(out, "Findings  : {}", report.findings.len()).unwrap();
    writeln!(out).unwrap();

    for (index, finding) in report.findings.iter().enumerate() {
        let label = match finding.severity {
            Severity::Warning => "WARN",
            Severity::Info => "INFO",
        };
        writeln!(out, "{}. [{label}] {}", index + 1, finding.title).unwrap();
        writeln!(out, "   ID: {} | Confidence: {}", finding.id, finding.confidence.as_str()).unwrap();
        writeln!(out, "   {}", finding.explanation).unwrap();
        for item in &finding.evidence {
            writeln!(out, "   Evidence: {item}").unwrap();
        }
        for item in &finding.checks {
            writeln!(out, "   Check: {item}").unwrap();
        }
        writeln!(out).unwrap();
    }

    if !report.timeline.is_empty() {
        writeln!(out, "CLASSIFIED EVENT TIMELINE (last {} events)", report.timeline.len()).unwrap();
        for event in &report.timeline {
            writeln!(out, "  {}  {}", utc_timestamp(event.timestamp), event.kind.label()).unwrap();
        }
        writeln!(out).unwrap();
    }

    if !report.snapshot.notes.is_empty() {
        writeln!(out, "COLLECTION LIMITATIONS").unwrap();
        for note in &report.snapshot.notes {
            writeln!(out, "  - {note}").unwrap();
        }
        writeln!(out).unwrap();
    }

    writeln!(out, "PRIVACY: no raw journal messages are included in this report.").unwrap();
    writeln!(out, "CAUTION: findings are observations and hypotheses, not verified root causes.").unwrap();
    out
}

#[derive(Clone)]
enum JsonValue {
    String(String),
    Number(u64),
    Bool(bool),
    Null,
    Array(Vec<JsonValue>),
    Object(Vec<(&'static str, JsonValue)>),
}

fn js(s: impl Into<String>) -> JsonValue { JsonValue::String(s.into()) }
fn n(v: u64) -> JsonValue { JsonValue::Number(v) }
fn arr(v: Vec<JsonValue>) -> JsonValue { JsonValue::Array(v) }
fn object(v: Vec<(&'static str, JsonValue)>) -> JsonValue { JsonValue::Object(v) }
fn opt_bool(v: Option<bool>) -> JsonValue { v.map(JsonValue::Bool).unwrap_or(JsonValue::Null) }
fn opt_num(v: Option<u64>) -> JsonValue { v.map(JsonValue::Number).unwrap_or(JsonValue::Null) }

fn write_escaped_json(out: &mut String, content: &str) {
    out.push('"');
    for ch in content.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            ch if ch <= '\u{1f}' => write!(out, "\\u{:04x}", ch as u32).unwrap(),
            ch => out.push(ch),
        }
    }
    out.push('"');
}

fn write_json(out: &mut String, value: &JsonValue, depth: usize) {
    match value {
        JsonValue::String(s) => write_escaped_json(out, s),
        JsonValue::Number(number) => write!(out, "{number}").unwrap(),
        JsonValue::Bool(value) => out.push_str(if *value { "true" } else { "false" }),
        JsonValue::Null => out.push_str("null"),
        JsonValue::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 { out.push(','); }
                out.push('\n');
                out.push_str(&"  ".repeat(depth + 1));
                write_json(out, item, depth + 1);
            }
            if !items.is_empty() { out.push('\n'); out.push_str(&"  ".repeat(depth)); }
            out.push(']');
        }
        JsonValue::Object(items) => {
            out.push('{');
            for (i, (key, val)) in items.iter().enumerate() {
                if i > 0 { out.push(','); }
                out.push('\n');
                out.push_str(&"  ".repeat(depth + 1));
                write_escaped_json(out, key);
                out.push_str(": ");
                write_json(out, val, depth + 1);
            }
            if !items.is_empty() { out.push('\n'); out.push_str(&"  ".repeat(depth)); }
            out.push('}');
        }
    }
}

fn finding_json(f: &Finding) -> JsonValue {
    object(vec![
        ("id", js(f.id)),
        ("severity", js(f.severity.as_str())),
        ("confidence", js(f.confidence.as_str())),
        ("title", js(f.title.clone())),
        ("explanation", js(f.explanation.clone())),
        ("evidence", arr(f.evidence.iter().cloned().map(js).collect())),
        ("checks", arr(f.checks.iter().cloned().map(js).collect())),
    ])
}

pub fn json(report: &Report) -> String {
    let s = &report.snapshot;
    let value = object(vec![
        ("schema_version", n(1)),
        ("tool", js("whydown")),
        ("version", js(report.version)),
        ("generated_at", js(utc_timestamp(report.generated_at))),
        ("lookback_seconds", n(report.lookback_seconds)),
        ("focus", js(report.focus.as_str())),
        ("demo", JsonValue::Bool(report.demo)),
        ("system", object(vec![
            ("os", js(s.os.clone())),
            ("kernel", js(s.kernel.clone())),
            ("uptime_seconds", opt_num(s.uptime_seconds)),
            ("ipv4_default_route", opt_bool(s.ipv4_default_route)),
            ("ipv6_default_route", opt_bool(s.ipv6_default_route)),
            ("dns_nameservers", opt_num(s.dns_nameservers.map(|v| v as u64))),
            ("dns_stub", JsonValue::Bool(s.dns_stub)),
            ("resolved_active", opt_bool(s.resolved_active)),
            ("failed_units", arr(s.failed_units.iter().cloned().map(js).collect())),
            ("interfaces", arr(s.interfaces.iter().map(|iface| object(vec![
                ("name", js(iface.name.clone())),
                ("rx_errors", n(iface.rx_errors)),
                ("rx_dropped", n(iface.rx_dropped)),
                ("tx_errors", n(iface.tx_errors)),
                ("tx_dropped", n(iface.tx_dropped)),
            ])).collect())),
            ("disks", arr(s.disks.iter().map(|disk| object(vec![
                ("mountpoint", js(disk.mountpoint.clone())),
                ("used_percent", n(disk.used_percent as u64)),
            ])).collect())),
        ])),
        ("findings", arr(report.findings.iter().map(finding_json).collect())),
        ("timeline", arr(report.timeline.iter().map(|event| object(vec![
            ("timestamp", js(utc_timestamp(event.timestamp))),
            ("kind", js(event.kind.as_str())),
            ("label", js(event.kind.label())),
        ])).collect())),
        ("collection_notes", arr(s.notes.iter().cloned().map(js).collect())),
        ("privacy", js("No raw journal lines; may contain unit names and local mountpoints")),
        ("disclaimer", js("Observations and hypotheses, not verified root causes")),
    ]);
    let mut out = String::new();
    write_json(&mut out, &value, 0);
    out.push('\n');
    out
}

fn html_escape(s: &str) -> String {
    let mut escaped = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            c => escaped.push(c),
        }
    }
    escaped
}

pub fn html(report: &Report) -> String {
    let mut out = String::from(r#"<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="color-scheme" content="dark light"><title>WhyDown diagnostic report</title>
<style>
:root{font-family:system-ui,-apple-system,sans-serif;color-scheme:dark;--bg:#101722;--card:#192535;--text:#eff5fd;--muted:#acbccf;--accent:#61a6ff;--border:#354458}
*{box-sizing:border-box}body{background:var(--bg);color:var(--text);margin:0}main{max-width:980px;margin:auto;padding:35px 20px 60px}header{border-bottom:1px solid var(--border);padding-bottom:20px;margin-bottom:22px}
h1{font-size:2.3rem;margin:0 0 6px}h2{margin:30px 0 15px}p{line-height:1.6}small,.muted{color:var(--muted)}.lead{font-size:1.03rem;color:var(--muted)}.meta{display:flex;gap:10px;flex-wrap:wrap;margin:18px 0}
.chip{border:1px solid var(--border);border-radius:999px;padding:7px 12px;background:var(--card);font-size:.87rem}
.card{background:var(--card);border:1px solid var(--border);border-radius:14px;padding:20px;margin-bottom:14px}
.card h3{margin:0 0 8px;font-size:1.15rem}.card p{margin:8px 0}.tag{font-size:.73rem;letter-spacing:.05em;font-weight:800;text-transform:uppercase;border-radius:4px;padding:4px 7px;display:inline-block;margin-bottom:9px}
.warning{color:#ffcd8c;background:#603818}.info{color:#aad7fc;background:#213e5b}ul{margin:8px 0 12px;padding-left:22px}li{margin:7px 0;line-height:1.45}
.timeline{border-left:2px solid var(--border);margin-left:10px;padding-left:18px}.event{padding:9px 0;border-bottom:1px solid var(--border)}.event time{color:var(--muted);font-family:ui-monospace,monospace;font-size:.82rem;display:block;margin-bottom:4px}
code{color:#c7e0ff}footer{margin-top:35px;border-top:1px solid var(--border);padding-top:20px;color:var(--muted);font-size:.85rem}
@media(prefers-color-scheme:light){:root{color-scheme:light;--bg:#f5f7fb;--card:#fff;--text:#1b2938;--muted:#56677c;--border:#dbe4ee}.warning{color:#8d4500;background:#fff0dc}.info{color:#1a5692;background:#e5f0fb}code{color:#165c9f}}
</style></head><body><main><header><h1>WhyDown</h1><p class="lead">Evidence-based Linux diagnostics — read-only and offline</p>"#);
    if report.demo { out.push_str("<span class=\"tag info\">DEMO DATA</span>"); }
    let data = [
        ("System", report.snapshot.os.as_str()),
        ("Kernel", report.snapshot.kernel.as_str()),
        ("Generated", &utc_timestamp(report.generated_at)),
        ("Focus", report.focus.as_str()),
    ];
    out.push_str("<div class=\"meta\">");
    for (label, value) in data {
        write!(out, "<span class=\"chip\"><strong>{}</strong> {}</span>", html_escape(label), html_escape(value)).unwrap();
    }
    out.push_str("</div></header><h2>Findings</h2>");
    for finding in &report.findings {
        let class = finding.severity.as_str();
        write!(out, "<article class=\"card\"><span class=\"tag {class}\">{class}</span><h3>{}</h3><small>{} · confidence: {}</small><p>{}</p>",
            html_escape(&finding.title), html_escape(finding.id), finding.confidence.as_str(), html_escape(&finding.explanation)).unwrap();
        out.push_str("<strong>Evidence</strong><ul>");
        for item in &finding.evidence { write!(out, "<li>{}</li>", html_escape(item)).unwrap(); }
        out.push_str("</ul><strong>Suggested checks</strong><ul>");
        for item in &finding.checks { write!(out, "<li>{}</li>", html_escape(item)).unwrap(); }
        out.push_str("</ul></article>");
    }
    if !report.timeline.is_empty() {
        out.push_str("<h2>Classified event timeline</h2><div class=\"timeline\">");
        for event in &report.timeline {
            write!(out, "<div class=\"event\"><time>{}</time>{}</div>", utc_timestamp(event.timestamp), html_escape(event.kind.label())).unwrap();
        }
        out.push_str("</div>");
    }
    if !report.snapshot.notes.is_empty() {
        out.push_str("<h2>Collection limitations</h2><ul>");
        for note in &report.snapshot.notes { write!(out, "<li>{}</li>", html_escape(note)).unwrap(); }
        out.push_str("</ul>");
    }
    out.push_str("<footer>No raw journal messages are included. Reports may contain unit names and local mountpoints. Findings are not verified root causes. No external resources, scripts, or telemetry.</footer></main></body></html>\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{demo, engine};
    use crate::model::Focus;

    #[test]
    fn render_different_formats() {
        let report = engine::analyze(demo::snapshot(), 3_600, Focus::All, true);
        let plain = text(&report);
        assert!(plain.contains("LINK_BEFORE_DNS"));
        let structured = json(&report);
        assert!(structured.contains("\"schema_version\": 1"));
        assert!(structured.contains("\"demo\": true"));
        let page = html(&report);
        assert!(page.contains("<!doctype html>"));
        assert!(page.contains("DEMO DATA"));
    }

    #[test]
    fn escapes_html_and_json() {
        assert_eq!(html_escape("<script>\"&'"), "&lt;script&gt;&quot;&amp;&#39;");
        let mut s = String::new();
        write_escaped_json(&mut s, "a\n\"b\\c\t");
        assert_eq!(s, "\"a\\n\\\"b\\\\c\\t\"");
    }
}
