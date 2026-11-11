//! Read-only, local collectors. No shell invocation, network probes, or mutations.

use crate::model::{DiskUsage, Event, EventKind, Focus, InterfaceStats, Snapshot};
use std::collections::HashSet;
use std::fs;
use std::process::Command;

pub fn collect(focus: Focus, since_unix: u64, max_events: usize) -> Snapshot {
    let mut snapshot = Snapshot {
        os: read_os_name().unwrap_or_else(|| "Unknown Linux".into()),
        kernel: read_trimmed("/proc/sys/kernel/osrelease").unwrap_or_else(|| "Unknown".into()),
        uptime_seconds: fs::read_to_string("/proc/uptime")
            .ok()
            .and_then(|v| v.split_whitespace().next()?.parse::<f64>().ok())
            .filter(|v| v.is_finite() && *v >= 0.0)
            .map(|v| v as u64),
        ..Snapshot::default()
    };

    if focus.includes(Focus::Network) {
        match fs::read_to_string("/proc/net/route") {
            Ok(text) => snapshot.ipv4_default_route = Some(parse_ipv4_default_route(&text)),
            Err(_) => snapshot.notes.push("IPv4 routing table unavailable".into()),
        }
        match fs::read_to_string("/proc/net/ipv6_route") {
            Ok(text) => snapshot.ipv6_default_route = Some(parse_ipv6_default_route(&text)),
            Err(_) => snapshot.notes.push("IPv6 routing table unavailable".into()),
        }
        match fs::read_to_string("/etc/resolv.conf") {
            Ok(text) => {
                let (count, stub) = parse_resolv_conf(&text);
                snapshot.dns_nameservers = Some(count);
                snapshot.dns_stub = stub;
            }
            Err(_) => snapshot.notes.push("Cannot inspect /etc/resolv.conf".into()),
        }
        if snapshot.dns_stub {
            snapshot.resolved_active = match run_capture("systemctl", &["is-active", "systemd-resolved.service"]) {
                Ok((_, output)) => Some(output.trim() == "active"),
                Err(_) => {
                    snapshot.notes.push("Cannot determine systemd-resolved state".into());
                    None
                }
            };
        }
        if let Ok(contents) = fs::read_to_string("/proc/net/dev") {
            snapshot.interfaces = parse_interfaces(&contents);
        }
    }

    if focus.includes(Focus::Services) {
        match run_capture("systemctl", &["list-units", "--failed", "--no-legend", "--plain", "--no-pager"]) {
            Ok((true, output)) => snapshot.failed_units = parse_failed_units(&output),
            Ok((false, _)) => snapshot.notes.push("Failed systemd units unavailable (systemd may be absent or inaccessible)".into()),
            Err(_) => snapshot.notes.push("systemctl is not available".into()),
        }
    }

    if focus.includes(Focus::System) {
        match run_capture("df", &["-P"]) {
            Ok((true, output)) => snapshot.disks = parse_df(&output),
            _ => snapshot.notes.push("Filesystem usage unavailable".into()),
        }
    }

    let since_arg = format!("--since=@{since_unix}");
    let count_arg = format!("--lines={max_events}");
    let journal_args = ["--no-pager", "--output=short-unix", since_arg.as_str(), count_arg.as_str()];
    match run_capture("journalctl", &journal_args) {
        Ok((true, output)) => {
            let lines: Vec<&str> = output.lines().collect();
            // With --lines N, a long journal may contain earlier entries not read here.
            if lines.len() >= max_events {
                snapshot.notes.push(format!(
                    "Only the newest {max_events} journal entries were examined; earlier entries in the requested period may be missing"
                ));
            }
            if lines.is_empty() || output.contains("-- No entries --") {
                snapshot.notes.push("No journal entries were returned; logging may be unavailable, inaccessible or simply quiet".into());
            }
            snapshot.events = parse_journal(&output);
        }
        Ok((false, _)) => snapshot.notes.push(
            "Journal collection failed or access is restricted; try adding your user to the systemd-journal group (log out/in afterwards)".into(),
        ),
        Err(_) => snapshot.notes.push("journalctl is unavailable; event analysis skipped".into()),
    }

    snapshot
}

fn read_trimmed(path: &str) -> Option<String> {
    fs::read_to_string(path).ok().map(|value| value.trim().to_string())
}

fn read_os_name() -> Option<String> {
    let text = fs::read_to_string("/etc/os-release").ok()?;
    for line in text.lines() {
        if let Some(value) = line.strip_prefix("PRETTY_NAME=") {
            return Some(unquote_os_release(value));
        }
    }
    None
}

fn unquote_os_release(value: &str) -> String {
    let text = value.trim();
    if text.len() >= 2 && text.starts_with('"') && text.ends_with('"') {
        text[1..text.len()-1].replace("\\\"", "\"").replace("\\\\", "\\")
    } else {
        text.to_string()
    }
}

fn run_capture(command: &str, args: &[&str]) -> Result<(bool, String), std::io::Error> {
    // Fixed executables and positional arguments only; never a shell.
    let result = Command::new(command).args(args).output()?;
    Ok((result.status.success(), String::from_utf8_lossy(&result.stdout).into_owned()))
}

pub fn parse_ipv4_default_route(content: &str) -> bool {
    content.lines().skip(1).any(|line| {
        let fields: Vec<_> = line.split_whitespace().collect();
        if fields.len() < 4 || fields[1] != "00000000" || fields[0] == "lo" { return false; }
        u32::from_str_radix(fields[3], 16)
            .map(|flags| flags & 0x1 != 0)
            .unwrap_or(false)
    })
}

pub fn parse_ipv6_default_route(content: &str) -> bool {
    content.lines().any(|line| {
        let fields: Vec<_> = line.split_whitespace().collect();
        fields.len() >= 10 && fields[0] == "0".repeat(32)
            && fields[1] == "00" && fields[9] != "lo"
    })
}

pub fn parse_resolv_conf(content: &str) -> (usize, bool) {
    let servers: Vec<_> = content.lines().filter_map(|line| {
        let line = line.split('#').next()?.trim();
        let mut parts = line.split_whitespace();
        if parts.next()? == "nameserver" { parts.next() } else { None }
    }).collect();
    let stub = servers.iter().any(|s| *s == "127.0.0.53" || *s == "127.0.0.54");
    (servers.len(), stub)
}

pub fn parse_interfaces(content: &str) -> Vec<InterfaceStats> {
    content.lines().skip(2).filter_map(|line| {
        let (name, counters) = line.split_once(':')?;
        let values: Vec<u64> = counters.split_whitespace()
            .map(str::parse::<u64>)
            .collect::<Result<Vec<_>, _>>().ok()?;
        if values.len() < 12 { return None; }
        let name = name.trim();
        if name == "lo" { return None; }
        Some(InterfaceStats {
            name: name.to_string(),
            rx_errors: values[2],
            rx_dropped: values[3],
            tx_errors: values[10],
            tx_dropped: values[11],
        })
    }).collect()
}

pub fn parse_failed_units(content: &str) -> Vec<String> {
    let mut units: Vec<String> = content.lines().filter_map(|line| {
        let candidate = line.split_whitespace()
            .find(|part| part.ends_with(".service") || part.ends_with(".mount") || part.ends_with(".socket") || part.ends_with(".timer"))?;
        Some(candidate.to_string())
    }).collect();
    units.sort();
    units.dedup();
    units.truncate(25);
    units
}

pub fn parse_df(content: &str) -> Vec<DiskUsage> {
    content.lines().skip(1).filter_map(|line| {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 6 { return None; }
        let percent = cols[4].trim_end_matches('%').parse::<u8>().ok()?;
        // The mountpoint is the remaining column(s). Avoid including filesystem identifiers.
        let mountpoint = cols[5..].join(" ");
        Some(DiskUsage { mountpoint, used_percent: percent })
    }).collect()
}

/// Parse systemd journal's `short-unix` output into coarse event types.
/// Raw log lines are deliberately discarded: exported reports contain no log bodies.
pub fn parse_journal(content: &str) -> Vec<Event> {
    let mut seen = HashSet::new();
    let mut events = Vec::new();
    for line in content.lines() {
        let mut parts = line.trim().splitn(2, char::is_whitespace);
        let unix = match parts.next().and_then(|v| v.split('.').next())
            .and_then(|v| v.parse::<u64>().ok()) {
            Some(v) => v,
            None => continue,
        };
        let body = parts.next().unwrap_or("");
        let kind = match classify_log_line(body) { Some(k) => k, None => continue };
        if seen.insert((unix, kind.as_str())) {
            events.push(Event { timestamp: unix, kind });
        }
    }
    events.sort_by_key(|event| event.timestamp);
    events
}

pub fn classify_log_line(text: &str) -> Option<EventKind> {
    let t = text.to_ascii_lowercase();

    if ["out of memory", "oom-killer", "memory cgroup out of memory", "killed process"].iter().any(|p| t.contains(p)) {
        return Some(EventKind::OutOfMemory);
    }
    if ["no space left on device", "read-only file system", "input/output error", "i/o error"].iter().any(|p| t.contains(p)) {
        return Some(EventKind::StorageError);
    }
    if t.contains("deauthenticat") || t.contains("lost carrier") || t.contains("carrier lost")
        || t.contains("link is down") || t.contains("link down")
        || t.contains("association lost")
        || (t.contains("disconnected") && (
            t.contains("networkmanager") || t.contains("wpa_supplicant") || t.contains("wlan")
            || t.contains("wi-fi") || t.contains("wifi")
        ) && !t.contains("disconnected -> activated") && !t.contains("disconnected -> connecting")) {
        return Some(EventKind::LinkDown);
    }
    if t.contains("dhcp") && ["failed", "timeout", "timed out", "no lease", "expired", "unable", "error"].iter().any(|p| t.contains(p))
        || t.contains("lease lost") || t.contains("failed to renew lease") {
        return Some(EventKind::DhcpProblem);
    }
    if t.contains("temporary failure in name resolution") || t.contains("name or service not known")
        || t.contains("servfail") || (t.contains("dns") && ["fail", "timeout", "unreachable", "error"].iter().any(|p| t.contains(p))) {
        return Some(EventKind::DnsProblem);
    }
    if t.contains("network is unreachable") || t.contains("no route to host") {
        return Some(EventKind::NetworkUnreachable);
    }
    if ["failed to start", "start request repeated too quickly", "entered failed state", "failed with result", "main process exited"].iter().any(|p| t.contains(p)) {
        return Some(EventKind::ServiceFailure);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipv4_route_works() {
        let head = "Iface Destination Gateway Flags RefCnt Use Metric Mask MTU Window IRTT\n";
        assert!(!parse_ipv4_default_route(&format!("{head}eth0 00000000 00000000 0000 0 0 0 00000000 0 0 0")));
        assert!(parse_ipv4_default_route(&format!("{head}eth0 00000000 0102A8C0 0003 0 0 0 00000000 0 0 0")));
    }

    #[test]
    fn ipv6_default_route_excludes_loopback_placeholder() {
        let base = "00000000000000000000000000000000 00 00000000000000000000000000000000 00 00000000000000000000000000000000 00000000 00000001 00000000 00200200";
        assert!(!parse_ipv6_default_route(&format!("{base} lo\n")));
        assert!(parse_ipv6_default_route(&format!("{base} eth0\n")));
    }

    #[test]
    fn nameservers() {
        assert_eq!(parse_resolv_conf("# comment\nnameserver 127.0.0.53\nnameserver 1.1.1.1\n"), (2, true));
        assert_eq!(parse_resolv_conf("search example.org\n"), (0, false));
    }

    #[test]
    fn journal_classification_and_order() {
        let journal = "1700000005.123 host systemd[1]: Failed to start Example Service.\n1700000002.002 host NetworkManager[42]: device wlan0 disconnected\n1700000003.002 host systemd-resolved[3]: DNS server unreachable\n";
        let events = parse_journal(journal);
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].kind, EventKind::LinkDown);
        assert_eq!(events[1].kind, EventKind::DnsProblem);
        assert_eq!(events[2].kind, EventKind::ServiceFailure);
    }

    #[test]
    fn do_not_misread_reconnection() {
        assert_eq!(classify_log_line("NetworkManager: state change: disconnected -> activated"), None);
    }

    #[test]
    fn parse_failed_unit_list() {
        let content = "● sshd.service loaded failed failed OpenSSH server\n   nginx.service loaded failed failed NGINX\n";
        assert_eq!(parse_failed_units(content), vec!["nginx.service", "sshd.service"]);
    }

    #[test]
    fn df_works() {
        let df = "Filesystem 1024-blocks Used Available Capacity Mounted on\n/dev/sda1 100 95 5 95% /\n";
        assert_eq!(parse_df(df)[0].used_percent, 95);
    }
}
