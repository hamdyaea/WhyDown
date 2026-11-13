//! Text, JSON and HTML outputs. No runtime third-party dependencies.

use crate::model::{count_word, Finding, Focus, Report, Severity};
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
    writeln!(out, "Journal   : last {} {}, newest entries only{}",
        report.lookback_seconds / 60,
        count_word((report.lookback_seconds / 60) as usize, "minute", "minutes"),
        if report.snapshot.journal_current_boot_only { " (current boot only)" } else { "" }).unwrap();
    if report.snapshot.journal_truncated {
        writeln!(out, "Journal   : PARTIAL — only newest {} entries examined", report.snapshot.journal_entries_examined).unwrap();
    }
    writeln!(out, "Findings  : {}", report.findings.len()).unwrap();
    writeln!(out).unwrap();

    for (index, finding) in report.findings.iter().enumerate() {
        let label = match finding.severity {
            Severity::Warning => "WARN",
            Severity::Info => "INFO",
        };
        writeln!(out, "{}. [{label}] {}", index + 1, finding.title).unwrap();
        writeln!(out, "   ID: {} | Confidence: {} | Scope: {}", finding.id, finding.confidence.as_str(), finding.scope()).unwrap();
        writeln!(out, "   {}", finding.explanation).unwrap();
        for item in &finding.evidence {
            writeln!(out, "   Evidence: {item}").unwrap();
        }
        for item in &finding.checks {
            writeln!(out, "   Check: {item}").unwrap();
        }
        writeln!(out).unwrap();
    }

    if report.focus.includes(Focus::Network) {
        writeln!(out, "CURRENT LOCAL NETWORK SNAPSHOT (no connectivity probing)").unwrap();
        writeln!(out, "  Default route: IPv4={} IPv6={}",
            state(report.snapshot.ipv4_default_route),
            state(report.snapshot.ipv6_default_route)).unwrap();
        if report.snapshot.interfaces.is_empty() {
            writeln!(out, "  Interface state: not available").unwrap();
        } else {
            for iface in report.snapshot.interfaces.iter().take(20) {
                writeln!(out, "  Interface {}: operstate={} (kernel snapshot; not internet reachability)",
                    iface.name, iface.operstate.as_deref().unwrap_or("unknown")).unwrap();
            }
        }
        if let Some(deltas) = &report.snapshot.interface_drop_deltas {
            writeln!(out, "  Counter changes observed during this diagnostic (not lifetime totals):").unwrap();
            if deltas.is_empty() { writeln!(out, "    No comparable interfaces or counters reset").unwrap(); }
            for d in deltas.iter().take(20) {
                writeln!(out, "    {}: RX dropped +{}, TX dropped +{} over {} ms",
                    d.name, d.rx_dropped_delta, d.tx_dropped_delta, d.elapsed_ms).unwrap();
            }
        } else {
            writeln!(out, "  Counter changes: no second comparable sample").unwrap();
        }
        writeln!(out).unwrap();
    }

    if report.focus.includes(Focus::System) {
        writeln!(out, "HOST RESOURCE SNAPSHOT (read-only; optional probes)").unwrap();
        if let Some(m) = &report.snapshot.memory_health {
            writeln!(out, "  Memory: {} MiB available / {} MiB total",
                m.available_kib / 1024, m.total_kib / 1024).unwrap();
            if let (Some(total), Some(free)) = (m.swap_total_kib, m.swap_free_kib) {
                writeln!(out, "  Swap: {} MiB free / {} MiB total", free/1024, total/1024).unwrap();
            }
        } else { writeln!(out, "  Memory: not verified").unwrap(); }
        for p in &report.snapshot.pressure_health {
            if let Some(some) = p.some_avg60_x100 {
                write!(out, "  PSI {}: some avg60 {}.{:02}%", p.resource, some/100, some%100).unwrap();
                if let Some(full) = p.full_avg60_x100 {
                    write!(out, "; full avg60 {}.{:02}%", full/100, full%100).unwrap();
                }
                writeln!(out).unwrap();
            }
        }
        if let Some(h) = &report.snapshot.file_handle_health {
            writeln!(out, "  File handles: {} / {} maximum",
                h.allocated.saturating_sub(h.unused), h.maximum).unwrap();
        }
        for inode in report.snapshot.inode_health.iter().take(20) {
            writeln!(out, "  Inodes {}: {}% used", inode.mountpoint, inode.used_percent).unwrap();
        }
        if report.snapshot.inode_health.len() > 20 {
            writeln!(out, "  More inode mounts omitted from text view").unwrap();
        }
        for md in &report.snapshot.raid_health {
            writeln!(out, "  MD {}: {}/{} active members", md.name, md.active, md.expected).unwrap();
        }
        writeln!(out).unwrap();
        if let Some(checked) = report.snapshot.pacman_checked {
            writeln!(out, "CURRENT PACMAN LOCAL DATABASE CHECK (read-only)").unwrap();
            let status = if !checked {
                "not verified (command unavailable or query failed)"
            } else if report.snapshot.pacman_consistent == Some(true) {
                "consistency check passed (not installed-file integrity)"
            } else {
                "check returned an error; inspect pacman -Dk output"
            };
            writeln!(out, "  {status}").unwrap();
            writeln!(out).unwrap();
        }
        if let Some(checked) = report.snapshot.dpkg_checked {
            writeln!(out, "CURRENT DPKG PACKAGE STATUS (dpkg database only; not a pacman check)").unwrap();
            if checked {
                writeln!(out, "  Incomplete package states: {}", report.snapshot.dpkg_problem_count).unwrap();
                for pkg in &report.snapshot.dpkg_problems {
                    writeln!(out, "  {}: {}{}", pkg.name, pkg.state,
                        if pkg.reinstall_required { " (reinstallation required)" } else { "" }).unwrap();
                }
                if report.snapshot.dpkg_problem_count > report.snapshot.dpkg_problems.len() {
                    writeln!(out, "  Remaining affected packages omitted from detailed display").unwrap();
                }
            } else {
                writeln!(out, "  Package database query unavailable; status not verified").unwrap();
            }
            writeln!(out).unwrap();
        }
    }

    if report.focus.includes(Focus::Services) {
        writeln!(out, "CURRENT SYSTEMD FAILED UNIT SNAPSHOT").unwrap();
        if report.snapshot.system_units_checked {
            writeln!(out, "  System manager: {} failed {}", report.snapshot.failed_units.len(),
                count_word(report.snapshot.failed_units.len(), "unit", "units")).unwrap();
        } else {
            writeln!(out, "  System manager: not checked").unwrap();
        }
        if let Some(failed) = &report.snapshot.failed_user_units {
            writeln!(out, "  User manager: {} failed {} (invoking user)", failed.len(),
                count_word(failed.len(), "unit", "units")).unwrap();
        } else {
            writeln!(out, "  User manager: not checked").unwrap();
        }
        if !report.snapshot.service_statuses.is_empty() {
            writeln!(out, "  Targeted unit states (verified system/user managers; not app health):").unwrap();
            for unit in &report.snapshot.service_statuses {
                writeln!(out, "    {} [{} manager]: load={} active={} sub={} result={} exit_status={} transient={} desktop_launcher={} NRestarts={}",
                    unit.unit, unit.manager, unit.load_state, unit.active_state,
                    unit.sub_state, unit.result,
                    unit.exec_main_status.map(|v| v.to_string()).unwrap_or_else(|| "unknown".into()),
                    unit.transient.map(|v| v.to_string()).unwrap_or_else(|| "unknown".into()),
                    unit.desktop_entry,
                    unit.n_restarts.map(|n| n.to_string()).unwrap_or_else(|| "unknown".into())).unwrap();
            }
        }
        writeln!(out).unwrap();
    }

    if !report.service_event_windows.is_empty() {
        writeln!(out, "HISTORICAL SERVICE MESSAGE WINDOWS (not independent failed-service counts)").unwrap();
        for (i, group) in report.service_event_windows.iter().enumerate() {
            writeln!(out, "  {}. {} – {} | {} classified {}; {} unattributed; unit(s): {}; boot: {}",
                i + 1, utc_timestamp(group.start), utc_timestamp(group.end),
                group.entry_count, count_word(group.entry_count, "entry", "entries"), group.unattributed_entries,
                if group.units.is_empty() { "unknown".to_string() } else { group.units.join(", ") },
                boot_relation(report, group.start, group.end)).unwrap();
        }
        writeln!(out).unwrap();
    }

    if !report.network_incidents.is_empty() {
        writeln!(out, "NETWORK INCIDENT WINDOWS ({} time {}; historical observations, current connectivity not verified)",
            report.network_incidents.len(), count_word(report.network_incidents.len(), "cluster", "clusters")).unwrap();
        for (i, group) in report.network_incidents.iter().enumerate() {
            writeln!(out, "  {}. {} – {} | {} {}: {}", i + 1,
                utc_timestamp(group.start), utc_timestamp(group.end),
                group.event_count(), count_word(group.event_count(), "event", "events"),
                match group.subject.as_deref() {
                    Some(name) => format!("{} [{}]", group.summary(), name),
                    None => format!("{} [interface unknown]", group.summary()),
                }).unwrap();
        }
        writeln!(out).unwrap();
    }

    if !report.timeline.is_empty() {
        writeln!(out, "CLASSIFIED EVENT TIMELINE (last {} {})",
            report.timeline.len(), count_word(report.timeline.len(), "event", "events")).unwrap();
        for event in &report.timeline {
            let indicator = report.snapshot.event_contexts.iter().find(|ctx|
                ctx.timestamp == event.timestamp && ctx.kind == event.kind
                && ctx.subject == event.subject).map(|ctx| ctx.signal.as_str());
            writeln!(out, "  {}  {}{}{}{}",
                utc_timestamp(event.timestamp), event.kind.label(),
                event.subject.as_deref().map(|v| format!(" [{}]", v)).unwrap_or_default(),
                event.source.as_deref().map(|v| format!(" via {}", v)).unwrap_or_default(),
                indicator.map(|v| format!(" | {v}")).unwrap_or_default()).unwrap();
        }
        writeln!(out).unwrap();
    }

    writeln!(out, "COLLECTION COVERAGE (unavailable is not healthy)").unwrap();
    if report.snapshot.journal_truncated {
        writeln!(out, "  Journal: PARTIAL (newest {} entries only; earlier records omitted)",
            report.snapshot.journal_entries_examined).unwrap();
    } else {
        writeln!(out, "  Journal: {}", if report.snapshot.journal_checked { "queried" } else { "not verified" }).unwrap();
    }
    if report.focus.includes(Focus::System) {
        writeln!(out, "  Filesystem capacity: {}", if report.snapshot.disk_usage_checked { "queried" } else { "not verified" }).unwrap();
        writeln!(out, "  Root mount: {}", match report.snapshot.root_filesystem_read_only {
            Some(true) => "read-only", Some(false) => "read-write", None => "not verified"
        }).unwrap();
    }
    if report.focus.includes(Focus::Services) {
        writeln!(out, "  Systemd system units: {}", if report.snapshot.system_units_checked { "queried" } else { "not verified" }).unwrap();
        writeln!(out, "  Systemd user units: {}", if report.snapshot.failed_user_units.is_some() { "queried" } else { "not verified" }).unwrap();
    }
    writeln!(out).unwrap();

    if !report.snapshot.collector_timings.is_empty() {
        writeln!(out, "COLLECTOR TIMINGS (measured wall time)").unwrap();
        for timing in &report.snapshot.collector_timings {
            writeln!(out, "  {}: {} ms ({})", timing.name, timing.elapsed_ms,
                if timing.succeeded { "success" } else { "incomplete or failed" }).unwrap();
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

/// Approximate relation of a journal window to this host's current boot.
/// Derived from wall clock minus uptime, not a journal boot ID; clock changes
/// can make this estimate inaccurate.
fn boot_relation(report: &Report, start: u64, end: u64) -> &'static str {
    // journalctl --boot=0 already filters to the running boot. Do not
    // downgrade verified boot selection to an uptime-based heuristic.
    if report.snapshot.journal_current_boot_only
        && report.snapshot.journal_checked && !report.demo {
        return "current_boot_filtered";
    }
    let Some(uptime) = report.snapshot.uptime_seconds else { return "unknown"; };
    let cutoff = report.generated_at.saturating_sub(uptime);
    if end < cutoff { "before_current_boot_estimate" }
    else if start >= cutoff { "since_current_boot_estimate" }
    else { "spans_boot_estimate" }
}

fn state(v: Option<bool>) -> &'static str {
    match v { Some(true) => "present", Some(false) => "absent", None => "unknown" }
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
fn opt_str(v: &Option<String>) -> JsonValue {
    v.as_ref().map(|s| js(s.clone())).unwrap_or(JsonValue::Null)
}

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
        ("scope", js(f.scope())),
        ("title", js(f.title.clone())),
        ("explanation", js(f.explanation.clone())),
        ("evidence", arr(f.evidence.iter().cloned().map(js).collect())),
        ("checks", arr(f.checks.iter().cloned().map(js).collect())),
    ])
}

pub fn json(report: &Report) -> String {
    let s = &report.snapshot;
    let value = object(vec![
        ("schema_version", n(2)),
        ("tool", js("whydown")),
        ("version", js(report.version)),
        ("generated_at", js(utc_timestamp(report.generated_at))),
        ("lookback_seconds", n(report.lookback_seconds)),
        ("journal_current_boot_only", JsonValue::Bool(s.journal_current_boot_only)),
        ("journal_checked", JsonValue::Bool(s.journal_checked)),
        ("journal_truncated", JsonValue::Bool(s.journal_truncated)),
        ("journal_entries_examined", n(s.journal_entries_examined as u64)),
        ("focus", js(report.focus.as_str())),
        ("demo", JsonValue::Bool(report.demo)),
        ("summary", object(vec![
            ("warning_count", n(report.findings.iter().filter(|f| f.severity == Severity::Warning).count() as u64)),
            ("info_count", n(report.findings.iter().filter(|f| f.severity == Severity::Info).count() as u64)),
            ("current_warnings", n(report.findings.iter().filter(|f| f.severity == Severity::Warning && f.scope() == "current_snapshot").count() as u64)),
            ("historical_warnings", n(report.findings.iter().filter(|f| f.severity == Severity::Warning && f.scope() == "historical_journal").count() as u64)),
            ("active_connectivity_probed", JsonValue::Bool(false)),
        ])),
        ("system", object(vec![
            ("os", js(s.os.clone())),
            ("kernel", js(s.kernel.clone())),
            ("uptime_seconds", opt_num(s.uptime_seconds)),
            ("disk_usage_checked", JsonValue::Bool(s.disk_usage_checked)),
            ("root_filesystem_read_only", opt_bool(s.root_filesystem_read_only)),
            ("ipv4_default_route", opt_bool(s.ipv4_default_route)),
            ("ipv6_default_route", opt_bool(s.ipv6_default_route)),
            ("dns_nameservers", opt_num(s.dns_nameservers.map(|v| v as u64))),
            ("dns_stub", JsonValue::Bool(s.dns_stub)),
            ("resolved_active", opt_bool(s.resolved_active)),
            ("failed_units", arr(s.failed_units.iter().cloned().map(js).collect())),
            ("system_units_checked", JsonValue::Bool(s.system_units_checked)),
            ("service_statuses", arr(s.service_statuses.iter().map(|unit| object(vec![
                ("unit", js(unit.unit.clone())),
                ("manager", js(unit.manager.clone())),
                ("transient", opt_bool(unit.transient)),
                ("desktop_entry", JsonValue::Bool(unit.desktop_entry)),
                ("exec_main_status", opt_num(unit.exec_main_status)),
                ("load_state", js(unit.load_state.clone())),
                ("active_state", js(unit.active_state.clone())),
                ("sub_state", js(unit.sub_state.clone())),
                ("result", js(unit.result.clone())),
                ("n_restarts", opt_num(unit.n_restarts)),
            ])).collect())),
            ("failed_user_units", match &s.failed_user_units {
                Some(items) => arr(items.iter().cloned().map(js).collect()),
                None => JsonValue::Null,
            }),
            ("pacman_checked", opt_bool(s.pacman_checked)),
            ("pacman_consistent", opt_bool(s.pacman_consistent)),
            ("dpkg_checked", opt_bool(s.dpkg_checked)),
            ("dpkg_problem_count", match s.dpkg_checked {
                Some(true) => n(s.dpkg_problem_count as u64),
                _ => JsonValue::Null,
            }),
            ("dpkg_problems", arr(s.dpkg_problems.iter().map(|pkg| object(vec![
                ("name", js(pkg.name.clone())),
                ("state", js(pkg.state.clone())),
                ("reinstall_required", JsonValue::Bool(pkg.reinstall_required)),
            ])).collect())),
            ("memory", match &s.memory_health {
                Some(m) => object(vec![
                    ("total_kib", n(m.total_kib)),
                    ("available_kib", n(m.available_kib)),
                    ("swap_total_kib", opt_num(m.swap_total_kib)),
                    ("swap_free_kib", opt_num(m.swap_free_kib)),
                ]),
                None => JsonValue::Null,
            }),
            ("pressure", arr(s.pressure_health.iter().map(|p| object(vec![
                ("resource", js(p.resource.clone())),
                ("some_avg60_percent_x100", opt_num(p.some_avg60_x100.map(u64::from))),
                ("full_avg60_percent_x100", opt_num(p.full_avg60_x100.map(u64::from))),
            ])).collect())),
            ("file_handles", match &s.file_handle_health {
                Some(h) => object(vec![
                    ("allocated", n(h.allocated)), ("unused", n(h.unused)),
                    ("maximum", n(h.maximum)),
                ]),
                None => JsonValue::Null,
            }),
            ("inodes", arr(s.inode_health.iter().map(|i| object(vec![
                ("mountpoint", js(i.mountpoint.clone())),
                ("used_percent", n(i.used_percent as u64)),
            ])).collect())),
            ("md_arrays", arr(s.raid_health.iter().map(|md| object(vec![
                ("name", js(md.name.clone())), ("expected", n(md.expected as u64)),
                ("active", n(md.active as u64)),
            ])).collect())),
            ("interface_drop_deltas", match &s.interface_drop_deltas {
                Some(deltas) => arr(deltas.iter().map(|d| object(vec![
                    ("name", js(d.name.clone())),
                    ("rx_dropped_delta", n(d.rx_dropped_delta)),
                    ("tx_dropped_delta", n(d.tx_dropped_delta)),
                    ("elapsed_ms", n(d.elapsed_ms)),
                ])).collect()),
                None => JsonValue::Null,
            }),
            ("interfaces", arr(s.interfaces.iter().map(|iface| object(vec![
                ("name", js(iface.name.clone())),
                ("operstate", opt_str(&iface.operstate)),
                ("rx_errors", n(iface.rx_errors)),
                ("rx_dropped", n(iface.rx_dropped)),
                ("tx_errors", n(iface.tx_errors)),
                ("tx_dropped", n(iface.tx_dropped)),
            ])).collect())),
            ("disks", arr(s.disks.iter().map(|disk| object(vec![
                ("mountpoint", js(disk.mountpoint.clone())),
                ("used_percent", n(disk.used_percent as u64)),
            ])).collect())),
            ("disk_breakdowns", arr(s.disk_breakdowns.iter().map(|breakdown| object(vec![
                ("mountpoint", js(breakdown.mountpoint.clone())),
                ("top_directories", arr(breakdown.entries.iter().map(|dir| object(vec![
                    ("path", js(dir.path.clone())),
                    ("kib", n(dir.kib)),
                ])).collect())),
            ])).collect())),
        ])),
        ("service_event_windows", arr(report.service_event_windows.iter().map(|group| object(vec![
            ("start", js(utc_timestamp(group.start))),
            ("end", js(utc_timestamp(group.end))),
            ("entry_count", n(group.entry_count as u64)),
            ("boot_relation", js(boot_relation(report, group.start, group.end))),
            ("unattributed_entries", n(group.unattributed_entries as u64)),
            ("units", arr(group.units.iter().cloned().map(js).collect())),
            ("status", js("historical journal message cluster, not independent failed service")),
        ])).collect())),
        ("network_incidents", arr(report.network_incidents.iter().map(|incident| object(vec![
            ("subject", opt_str(&incident.subject)),
            ("start", js(utc_timestamp(incident.start))),
            ("end", js(utc_timestamp(incident.end))),
            ("event_count", n(incident.event_count() as u64)),
            ("summary", js(incident.summary())),
            ("link_events", n(incident.link_events as u64)),
            ("dhcp_events", n(incident.dhcp_events as u64)),
            ("dns_events", n(incident.dns_events as u64)),
            ("unreachable_events", n(incident.unreachable_events as u64)),
            ("status", js("historical; current connectivity not verified")),
        ])).collect())),
        ("findings", arr(report.findings.iter().map(finding_json).collect())),
        ("timeline", arr(report.timeline.iter().map(|event| object(vec![
            ("timestamp", js(utc_timestamp(event.timestamp))),
            ("kind", js(event.kind.as_str())),
            ("label", js(event.kind.label())),
            ("subject", opt_str(&event.subject)),
            ("source", opt_str(&event.source)),
            ("diagnostic_hint", match s.event_contexts.iter().find(|hint|
                hint.timestamp == event.timestamp && hint.kind == event.kind &&
                hint.subject == event.subject
            ) {
                Some(hint) => js(hint.signal.clone()),
                None => JsonValue::Null,
            }),
        ])).collect())),
        ("collector_timings", arr(s.collector_timings.iter().map(|timing| object(vec![
            ("name", js(timing.name)),
            ("elapsed_ms", n(timing.elapsed_ms)),
            ("succeeded", JsonValue::Bool(timing.succeeded)),
        ])).collect())),
        ("collection_notes", arr(s.notes.iter().cloned().map(js).collect())),
        ("privacy", js("No raw journal lines; includes bounded source, unit, and interface identifiers and local mountpoints")),
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
        ("Journal", if report.snapshot.journal_current_boot_only { "current boot only" } else { "all accessible boots" }),
    ];
    out.push_str("<div class=\"meta\">");
    for (label, value) in data {
        write!(out, "<span class=\"chip\"><strong>{}</strong> {}</span>", html_escape(label), html_escape(value)).unwrap();
    }
    out.push_str("</div></header><h2>Findings</h2>");
    for finding in &report.findings {
        let class = finding.severity.as_str();
        write!(out, "<article class=\"card\"><span class=\"tag {class}\">{class}</span><h3>{}</h3><small>{} · confidence: {} · scope: {}</small><p>{}</p>",
            html_escape(&finding.title), html_escape(finding.id), finding.confidence.as_str(), finding.scope(), html_escape(&finding.explanation)).unwrap();
        out.push_str("<strong>Evidence</strong><ul>");
        for item in &finding.evidence { write!(out, "<li>{}</li>", html_escape(item)).unwrap(); }
        out.push_str("</ul><strong>Suggested checks</strong><ul>");
        for item in &finding.checks { write!(out, "<li>{}</li>", html_escape(item)).unwrap(); }
        out.push_str("</ul></article>");
    }
    if report.focus.includes(Focus::Network) {
        out.push_str("<h2>Current local network snapshot</h2><p class=\"muted\">Read-only kernel state; no connectivity probes performed.</p>");
        write!(out, "<p>Default route: IPv4={} · IPv6={}</p>",
            state(report.snapshot.ipv4_default_route), state(report.snapshot.ipv6_default_route)).unwrap();
        for iface in report.snapshot.interfaces.iter().take(20) {
            write!(out, "<p>Interface <code>{}</code>: operstate=<code>{}</code></p>",
                html_escape(&iface.name), html_escape(iface.operstate.as_deref().unwrap_or("unknown"))).unwrap();
        }
        if let Some(deltas) = &report.snapshot.interface_drop_deltas {
            out.push_str("<h3>Interface counter changes during diagnostic</h3><ul>");
            for d in deltas.iter().take(20) {
                write!(out, "<li><code>{}</code>: RX dropped +{}, TX dropped +{} over {} ms</li>",
                    html_escape(&d.name), d.rx_dropped_delta, d.tx_dropped_delta, d.elapsed_ms).unwrap();
            }
            out.push_str("</ul>");
        }
    }
    if report.focus.includes(Focus::System) {
        out.push_str("<h2>Host resource snapshot</h2><p class=\"muted\">Read-only kernel measurements; unsupported sensors are not verified. Host values may differ from container limits.</p>");
        if let Some(m) = &report.snapshot.memory_health {
            write!(out, "<p>Memory available: {} / {} MiB</p>",
                m.available_kib / 1024, m.total_kib / 1024).unwrap();
        } else { out.push_str("<p>Memory: not verified</p>"); }
        for p in &report.snapshot.pressure_health {
            if let Some(some) = p.some_avg60_x100 {
                write!(out, "<p>PSI {} some avg60: {}.{:02}%",
                    html_escape(&p.resource), some / 100, some % 100).unwrap();
                if let Some(full) = p.full_avg60_x100 {
                    write!(out, "; full avg60 {}.{:02}%", full / 100, full % 100).unwrap();
                }
                out.push_str("</p>");
            }
        }
        if let Some(h) = &report.snapshot.file_handle_health {
            write!(out, "<p>Host file handles: {} / {}</p>",
                h.allocated.saturating_sub(h.unused), h.maximum).unwrap();
        }
        if !report.snapshot.inode_health.is_empty() {
            out.push_str("<h3>Inode usage</h3><ul>");
            for inode in report.snapshot.inode_health.iter().take(20) {
                write!(out, "<li><code>{}</code>: {}%</li>",
                    html_escape(&inode.mountpoint), inode.used_percent).unwrap();
            }
            out.push_str("</ul>");
        }
        if !report.snapshot.raid_health.is_empty() {
            out.push_str("<h3>Linux MD arrays</h3><ul>");
            for md in &report.snapshot.raid_health {
                write!(out, "<li><code>{}</code>: {}/{} active</li>",
                    html_escape(&md.name), md.active, md.expected).unwrap();
            }
            out.push_str("</ul>");
        }
        if let Some(checked) = report.snapshot.pacman_checked {
            out.push_str("<h2>pacman local database consistency</h2><p class=\"muted\">Read-only pacman -Dk; does not check installed file integrity or updates.</p><p>");
            out.push_str(if !checked { "Not verified: query unavailable" }
                else if report.snapshot.pacman_consistent == Some(true) { "Local database check passed" }
                else { "Check returned an error; inspect pacman -Dk output" });
            out.push_str("</p>");
        }
        if let Some(checked) = report.snapshot.dpkg_checked {
            out.push_str("<h2>Current dpkg package status</h2><p class=\"muted\">dpkg database only, not pacman; no repairs performed.</p>");
            if checked {
                write!(out, "<p>{} incompletely configured package state(s)</p>",
                    report.snapshot.dpkg_problem_count).unwrap();
                out.push_str("<ul>");
                for pkg in &report.snapshot.dpkg_problems {
                    write!(out, "<li><code>{}</code>: {}{}</li>",
                        html_escape(&pkg.name), html_escape(&pkg.state),
                        if pkg.reinstall_required { " (reinstallation required)" } else { "" }).unwrap();
                }
                out.push_str("</ul>");
            } else {
                out.push_str("<p>Package database query unavailable; status not verified</p>");
            }
        }
    }
    if report.focus.includes(Focus::Services) {
        out.push_str("<h2>Current systemd failed units</h2><p>");
        write!(out, "System manager: {}; User manager: {}",
            if report.snapshot.system_units_checked {
                format!("{} failed", report.snapshot.failed_units.len())
            } else { "not checked".into() },
            match &report.snapshot.failed_user_units {
                Some(units) => format!("{} failed (invoking user)", units.len()),
                None => "not checked".into(),
            }).unwrap();
        out.push_str("</p>");
        if !report.snapshot.service_statuses.is_empty() {
            out.push_str("<h3>Targeted current unit states</h3><p class=\"muted\">Scoped user/system manager state; application readiness is not verified.</p><ul>");
            for unit in &report.snapshot.service_statuses {
                write!(out, "<li><code>{}</code> [manager={}]: load={}, active={}, sub={}, result={}, exit_status={}, transient={}, desktop_launcher={}, NRestarts={}</li>",
                    html_escape(&unit.unit), html_escape(&unit.manager),
                    html_escape(&unit.load_state),
                    html_escape(&unit.active_state), html_escape(&unit.sub_state),
                    html_escape(&unit.result),
                    unit.exec_main_status.map(|v| v.to_string()).unwrap_or_else(|| "unknown".into()),
                    unit.transient.map(|v| v.to_string()).unwrap_or_else(|| "unknown".into()),
                    unit.desktop_entry,
                    unit.n_restarts.map(|n| n.to_string()).unwrap_or_else(|| "unknown".into())).unwrap();
            }
            out.push_str("</ul>");
        }
    }
    if !report.service_event_windows.is_empty() {
        out.push_str("<h2>Historical service message windows</h2><p class=\"muted\">Time clusters, not independent failed services or proof of a current outage.</p>");
        for group in &report.service_event_windows {
            write!(out, "<article class=\"card\"><h3>{} – {}</h3><p>{} classified {}; {} without unit attribution.</p><p>Unit(s): {}</p><p>Boot relation (approximate): {}</p></article>",
                utc_timestamp(group.start), utc_timestamp(group.end),
                group.entry_count, count_word(group.entry_count, "entry", "entries"), group.unattributed_entries,
                html_escape(&if group.units.is_empty() { "unknown".to_string() } else { group.units.join(", ") }),
                boot_relation(report, group.start, group.end)).unwrap();
        }
    }
    if !report.network_incidents.is_empty() {
        out.push_str("<h2>Network incident windows</h2><p class=\"muted\">Historical time clusters, not proven outages. Current connectivity has not been verified.</p>");
        for incident in &report.network_incidents {
            write!(out, "<article class=\"card\"><h3>{}</h3><p>Interface: {}</p><p><time>{} – {}</time></p><p>{} classified {}</p></article>",
                html_escape(&incident.summary()),
                html_escape(incident.subject.as_deref().unwrap_or("not identified")),
                utc_timestamp(incident.start), utc_timestamp(incident.end),
                incident.event_count(), count_word(incident.event_count(), "event", "events")).unwrap();
        }
    }
    if !report.timeline.is_empty() {
        out.push_str("<h2>Classified event timeline</h2><div class=\"timeline\">");
        for event in &report.timeline {
            write!(out, "<div class=\"event\"><time>{}</time>{}{}{}</div>",
                utc_timestamp(event.timestamp), html_escape(event.kind.label()),
                event.subject.as_deref().map(|s| format!(" [{}]", html_escape(s))).unwrap_or_default(),
                event.source.as_deref().map(|s| format!(" via {}", html_escape(s))).unwrap_or_default()).unwrap();
        }
        out.push_str("</div>");
    }
    out.push_str("<h2>Collection coverage</h2><p class=\"muted\">Unverified does not mean healthy.</p><ul>");
    if report.snapshot.journal_truncated {
        write!(out, "<li>Journal: PARTIAL — newest {} entries only; earlier records omitted</li>",
            report.snapshot.journal_entries_examined).unwrap();
    } else {
        write!(out, "<li>Journal: {}</li>", if report.snapshot.journal_checked { "queried" } else { "not verified" }).unwrap();
    }
    if report.focus.includes(Focus::System) {
        write!(out, "<li>Filesystem capacity: {}</li>",
            if report.snapshot.disk_usage_checked { "queried" } else { "not verified" }).unwrap();
        write!(out, "<li>Root filesystem mount: {}</li>",
            match report.snapshot.root_filesystem_read_only {
                Some(true) => "read-only", Some(false) => "read-write", None => "not verified"
            }).unwrap();
    }
    out.push_str("</ul>");
    if !report.snapshot.collector_timings.is_empty() {
        out.push_str("<h2>Collector timings</h2><p class=\"muted\">Actual elapsed time for each local subprocess; no estimated ETA.</p><ul>");
        for timing in &report.snapshot.collector_timings {
            write!(out, "<li><code>{}</code>: {} ms ({})</li>",
                html_escape(timing.name), timing.elapsed_ms,
                if timing.succeeded { "success" } else { "incomplete or failed" }).unwrap();
        }
        out.push_str("</ul>");
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
    fn targeted_systemd_state_is_rendered_and_not_confused_with_app_readiness() {
        let mut snapshot = demo::snapshot();
        snapshot.failed_units.clear();
        snapshot.service_statuses = vec![crate::model::ServiceStatus {
            unit: "example-backup.service".into(),
            manager: "system".into(),
            transient: None,
            desktop_entry: false,
            exec_main_status: None,
            load_state: "loaded".into(),
            active_state: "active".into(),
            sub_state: "running".into(),
            result: "success".into(),
            n_restarts: Some(1),
        }];
        snapshot.events.push(crate::model::Event {
            timestamp: crate::time::now_unix(),
            kind: crate::model::EventKind::ServiceRestartScheduled,
            subject: Some("example-backup.service".into()),
            source: Some("systemd".into()),
        });
        let report = engine::analyze(snapshot, 3600, Focus::All, true);
        assert!(text(&report).contains("active=active sub=running result=success exit_status=unknown transient=unknown desktop_launcher=false NRestarts=1"));
        let j = json(&report);
        assert!(j.contains("\"service_statuses\""));
        assert!(j.contains("\"n_restarts\": 1"));
        let h = html(&report);
        assert!(h.contains("Targeted current unit states"));
        assert!(h.contains("application readiness is not verified"));
        assert!(report.findings.iter().any(|f| f.id == "SERVICE_FAILURE_LOGS"));
    }

    #[test]
    fn sampled_drop_deltas_are_rendered_separately_from_cumulative_totals() {
        let mut snapshot = demo::snapshot();
        snapshot.interface_drop_deltas = Some(vec![crate::model::NetworkDropDelta {
            name: "wlan0".into(), rx_dropped_delta: 12, tx_dropped_delta: 2,
            elapsed_ms: 1500,
        }]);
        let report = engine::analyze(snapshot, 3600, Focus::All, true);
        assert!(text(&report).contains("RX dropped +12, TX dropped +2"));
        assert!(json(&report).contains("\"rx_dropped_delta\": 12"));
        assert!(html(&report).contains("Interface counter changes during diagnostic"));
    }

    #[test]
    fn truncated_journal_is_visible_in_text_json_and_html() {
        let snapshot = crate::model::Snapshot {
            journal_checked: true,
            journal_truncated: true,
            journal_entries_examined: 500,
            ..crate::model::Snapshot::default()
        };
        let report = engine::analyze(snapshot, 86400, Focus::All, false);
        assert!(text(&report).contains("PARTIAL — only newest 500 entries examined"));
        let structured = json(&report);
        assert!(structured.contains("\"journal_truncated\": true"));
        assert!(structured.contains("\"journal_entries_examined\": 500"));
        assert!(html(&report).contains("Journal: PARTIAL"));
    }

    #[test]
    fn coverage_and_incomplete_checks_are_explicit_in_all_formats() {
        let report = engine::analyze(crate::model::Snapshot::default(), 3600, Focus::All, false);
        assert_eq!(report.findings[0].id, "INSUFFICIENT_EVIDENCE");
        assert!(text(&report).contains("COLLECTION COVERAGE"));
        assert!(text(&report).contains("not verified"));
        let j = json(&report);
        assert!(j.contains("\"journal_checked\": false"));
        assert!(j.contains("\"disk_usage_checked\": false"));
        assert!(j.contains("\"root_filesystem_read_only\": null"));
        assert!(html(&report).contains("Collection coverage"));
    }

    #[test]
    fn collector_timings_remain_in_reports_without_raw_output() {
        let report = engine::analyze(demo::snapshot(), 3600, Focus::All, true);
        assert!(text(&report).contains("COLLECTOR TIMINGS"));
        assert!(json(&report).contains("\"collector_timings\""));
        assert!(json(&report).contains("\"elapsed_ms\": 0"));
        assert!(html(&report).contains("Collector timings"));
    }

    #[test]
    fn pacman_check_is_separate_in_text_html_and_json() {
        let mut snapshot = demo::snapshot();
        snapshot.pacman_checked = Some(true);
        snapshot.pacman_consistent = Some(false);
        snapshot.dpkg_checked = None;
        snapshot.dpkg_problems.clear();
        snapshot.dpkg_problem_count = 0;
        let report = engine::analyze(snapshot, 3600, Focus::System, true);
        assert!(report.findings.iter().any(|f| f.id == "PACMAN_DATABASE_CHECK_FAILED"));
        let plain = text(&report);
        assert!(plain.contains("CURRENT PACMAN LOCAL DATABASE CHECK"));
        assert!(!plain.contains("CURRENT DPKG PACKAGE STATUS"));
        let structured = json(&report);
        assert!(structured.contains("\"pacman_checked\": true"));
        assert!(structured.contains("\"pacman_consistent\": false"));
        assert!(html(&report).contains("pacman local database consistency"));
    }

    #[test]
    fn current_boot_scope_is_visible_across_report_formats() {
        let mut report = engine::analyze(demo::snapshot(), 3_600, Focus::All, true);
        report.snapshot.journal_current_boot_only = true;
        assert!(text(&report).contains("current boot only"));
        assert!(json(&report).contains("\"journal_current_boot_only\": true"));
        assert!(html(&report).contains("current boot only"));
    }

    #[test]
    fn render_different_formats() {
        let report = engine::analyze(demo::snapshot(), 3_600, Focus::All, true);
        let plain = text(&report);
        assert!(plain.contains("LINK_BEFORE_DNS"));
        let structured = json(&report);
        assert!(structured.contains("\"schema_version\": 2"));
        assert!(structured.contains("\"demo\": true"));
        assert!(structured.contains("\"scope\": \"historical_journal\""));
        assert!(structured.contains("\"subject\": \"wlan0\""));
        assert!(structured.contains("\"active_connectivity_probed\": false"));
        assert!(structured.contains("\"service_event_windows\""));
        assert!(structured.contains("\"failed_user_units\""));
        assert!(structured.contains("\"dpkg_checked\": true"));
        assert!(structured.contains("\"dpkg_problem_count\": 1"));
        assert!(structured.contains("\"example-kernel-headers\""));
        assert!(structured.contains("\"DPKG_INCOMPLETE_PACKAGES\""));
        assert!(plain.contains("CURRENT DPKG PACKAGE STATUS"));
        assert!(plain.contains("example-kernel-headers: unpacked"));
        assert!(plain.contains("CURRENT SYSTEMD FAILED UNIT SNAPSHOT"));
        let page = html(&report);
        assert!(page.contains("<!doctype html>"));
        assert!(page.contains("DEMO DATA"));
    }

    #[test]
    fn singular_service_event_and_timeline_labels() {
        use crate::model::{Event, EventKind, Snapshot};
        let snapshot = Snapshot {
            system_units_checked: true,
            failed_user_units: Some(vec![]),
            events: vec![
                Event { timestamp: 1, kind: EventKind::ServiceFailure,
                    subject: Some("example.service".into()), source: Some("systemd".into()) }
            ],
            ..Snapshot::default()
        };
        let report = engine::analyze(snapshot, 60, Focus::Services, false);
        let plain = text(&report);
        assert!(plain.contains("1 classified entry; 0 unattributed"));
        assert!(plain.contains("CLASSIFIED EVENT TIMELINE (last 1 event)"));
        assert!(plain.contains("System manager: 0 failed units"));
        assert!(!plain.contains("1 events"));
        let html_page = html(&report);
        assert!(html_page.contains("1 classified entry; 0 without unit attribution."));
    }

    #[test]
    fn requested_current_boot_is_verified_when_journal_was_collected() {
        let mut report = engine::analyze(demo::snapshot(), 3_600, Focus::All, false);
        report.snapshot.journal_current_boot_only = true;
        report.snapshot.journal_checked = true;
        report.snapshot.uptime_seconds = None;
        assert_eq!(boot_relation(&report, 100, 200), "current_boot_filtered");
        report.snapshot.journal_checked = false;
        assert_eq!(boot_relation(&report, 100, 200), "unknown");
    }

    #[test]
    fn service_windows_are_labeled_relative_to_estimated_boot() {
        let mut report = engine::analyze(demo::snapshot(), 3_600, Focus::All, true);
        report.snapshot.uptime_seconds = Some(94);
        let now = report.generated_at;
        assert_eq!(boot_relation(&report, now - 1000, now - 990), "before_current_boot_estimate");
        assert_eq!(boot_relation(&report, now - 30, now - 20), "since_current_boot_estimate");
        assert_eq!(boot_relation(&report, now - 110, now - 70), "spans_boot_estimate");
        report.snapshot.uptime_seconds = None;
        assert_eq!(boot_relation(&report, now - 1000, now - 990), "unknown");
    }

    #[test]
    fn escapes_html_and_json() {
        assert_eq!(html_escape("<script>\"&'"), "&lt;script&gt;&quot;&amp;&#39;");
        let mut s = String::new();
        write_escaped_json(&mut s, "a\n\"b\\c\t");
        assert_eq!(s, "\"a\\n\\\"b\\\\c\\t\"");
    }
}
