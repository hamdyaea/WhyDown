//! Read-only, local collectors. No shell invocation, network probes, or mutations.

use crate::model::{DirectoryUsage, DiskBreakdown, DiskUsage, Event, EventKind, Focus, InterfaceStats, Snapshot};
use std::collections::HashSet;
use std::fs;
use std::process::Command;

pub fn collect(focus: Focus, since_unix: u64, max_events: usize, disk_details: bool) -> Snapshot {
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
        match run_capture("df", &["-PT"]) {
            Ok((true, output)) => snapshot.disks = parse_df(&output),
            _ => snapshot.notes.push("Filesystem usage unavailable".into()),
        }
        if disk_details {
            // Optional: GNU du recursively walks the filesystem, possibly slowly.
            // Avoid scanning the root filesystem and cap to two distinct mounts.
            let paths: Vec<String> = snapshot.disks.iter()
                .filter(|d| d.used_percent >= 90 && d.mountpoint != "/")
                .map(|d| d.mountpoint.clone()).take(2).collect();
            if paths.is_empty() {
                snapshot.notes.push("Disk details: no eligible near-full non-root mountpoint to inspect".into());
            }
            for path in paths {
                match run_capture("du", &["-x", "-k", "--max-depth=2", "--", path.as_str()]) {
                    Ok((ok, output)) => {
                        let entries = parse_du(&output, &path);
                        if !ok {
                            snapshot.notes.push(format!("Directory sizes for {path} may be incomplete due to permissions or changed files"));
                        }
                        if !entries.is_empty() {
                            snapshot.disk_breakdowns.push(DiskBreakdown { mountpoint: path, entries });
                        } else {
                            snapshot.notes.push(format!("No directory-size details available for {path}"));
                        }
                    }
                    Err(_) => snapshot.notes.push("GNU du unavailable; disk breakdown skipped".into()),
                }
            }
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
        let operstate = if name.bytes().all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b)) {
            fs::read_to_string(format!("/sys/class/net/{name}/operstate"))
                .ok().map(|state| state.trim().to_string())
                .filter(|state| matches!(state.as_str(), "up" | "down" | "unknown" | "dormant" | "lowerlayerdown" | "notpresent" | "testing"))
        } else { None };
        Some(InterfaceStats {
            name: name.to_string(),
            operstate,
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

/// Parse GNU `df -PT` output and ignore compressed/immutable image filesystems.
/// SquashFS, EROFS and similar filesystems are normally 100% used by design:
/// their capacity is the image size, not available writable disk space.
/// Keep real writable volumes (including overlay/tmpfs) eligible for warnings.
pub fn parse_df(content: &str) -> Vec<DiskUsage> {
    content.lines().skip(1).filter_map(|line| {
        let cols: Vec<&str> = line.split_whitespace().collect();
        // `df -PT`: Filesystem Type 1024-blocks Used Available Capacity Mounted on
        if cols.len() < 7 { return None; }
        let fs_type = cols[1];
        if matches!(fs_type, "squashfs" | "erofs" | "iso9660" | "cramfs") {
            return None;
        }
        let percent = cols[5].trim_end_matches('%').parse::<u8>().ok()?;
        let mountpoint = cols[6..].join(" ");
        Some(DiskUsage { mountpoint, used_percent: percent })
    }).collect()
}

/// Parse GNU du -x -k --max-depth=2 output (KiB), largest subdirectories first.
/// This is not a byte-accurate inventory: hard links, snapshots, permissions,
/// concurrent writes and filesystem metadata can all affect the result.
pub fn parse_du(output: &str, mountpoint: &str) -> Vec<DirectoryUsage> {
    let prefix = format!("{}/", mountpoint.trim_end_matches('/'));
    let mut entries: Vec<DirectoryUsage> = output.lines().filter_map(|line| {
        let (kib, path) = line.split_once('\t')?;
        let kib = kib.parse::<u64>().ok()?;
        if !path.starts_with(&prefix) || path.contains("/../") { return None; }
        let relative = &path[prefix.len()..];
        if relative.is_empty() || relative.starts_with('.') && relative == "." { return None; }
        if relative.split('/').count() > 2 { return None; }
        Some(DirectoryUsage { path: path.to_string(), kib })
    }).collect();
    entries.sort_by(|a,b| b.kib.cmp(&a.kib).then(a.path.cmp(&b.path)));
    entries.truncate(8);
    entries
}

/// Extract only short, validated identifiers, never raw message bodies.
fn identifier(token: &str) -> Option<String> {
    let trimmed = token.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '-' && c != '.' && c != '@');
    if trimmed.is_empty() || trimmed.len() > 64
        || !trimmed.bytes().all(|b| b.is_ascii_alphanumeric() || b"_.-@".contains(&b)) {
        return None;
    }
    Some(trimmed.to_string())
}

fn event_source(body: &str) -> Option<String> {
    // journalctl short-unix: <time> <hostname> <program>[pid]: <message>.
    // Ignore hostname; take only the emitting program identifier.
    let label = body.split_whitespace().nth(1)?.trim_end_matches(':');
    let program = label.split('[').next().unwrap_or(label);
    identifier(program)
}

fn network_interface(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    ["eth", "enp", "eno", "ens", "enx", "wlan", "wlp", "wlx",
     "wwan", "w w", "veth", "bond", "br", "tun", "tap", "wg", "docker"]
        .iter().any(|prefix| n.starts_with(prefix)) && !n.contains("..")
}

fn event_subject(body: &str, kind: EventKind) -> Option<String> {
    let text = body.to_ascii_lowercase();
    if kind.focus() == Focus::Network {
        for marker in ["device (", "dhcp4 (", "dhcp6 (", "interface ("] {
            if let Some(start) = text.find(marker) {
                let rest = &text[start + marker.len()..];
                if let Some((candidate, _)) = rest.split_once(')') {
                    if let Some(value) = identifier(candidate) {
                        if network_interface(&value) { return Some(value); }
                    }
                }
            }
        }
        // Kernel messages such as "e1000e ... enp3s0: NIC Link is Down".
        for word in text.split_whitespace() {
            let candidate = word.trim_end_matches(':');
            if let Some(name) = identifier(candidate) {
                if network_interface(&name) && candidate.ends_with(':') {
                    return Some(name);
                }
            }
        }
    }
    if kind == EventKind::ServiceFailure {
        for word in text.split_whitespace() {
            if let Some(name) = identifier(word) {
                if [".service", ".socket", ".mount", ".timer", ".target", ".path"]
                    .iter().any(|suffix| name.ends_with(suffix)) {
                    return Some(name);
                }
            }
        }
    }
    None
}

/// Parse journalctl short-unix output into bounded classified events.
/// Only source/subject identifiers and UTC seconds survive; raw messages are discarded.
pub fn parse_journal(content: &str) -> Vec<Event> {
    let mut seen = HashSet::new();
    let mut events = Vec::new();
    let mut network_transition_until = 0_u64;
    for line in content.lines() {
        let mut parts = line.trim().splitn(2, char::is_whitespace);
        let unix = match parts.next().and_then(|v| v.split('.').next())
            .and_then(|v| v.parse::<u64>().ok()) {
            Some(v) => v,
            None => continue,
        };
        let body = parts.next().unwrap_or("");
        let lower = body.to_ascii_lowercase();
        if lower.contains("stopping network manager")
            || lower.contains("starting network manager")
            || (lower.contains("networkmanager[") && (
                lower.contains("caught sigterm") || lower.contains("is starting...")
            )) {
            network_transition_until = unix.saturating_add(60);
        }
        let kind = match classify_log_line(body) { Some(k) => k, None => continue };
        if matches!(kind, EventKind::LinkDown | EventKind::DhcpProblem)
            && unix <= network_transition_until {
            continue;
        }
        let subject = event_subject(body, kind);
        if seen.insert((unix, kind.as_str(), subject.clone())) {
            events.push(Event {
                timestamp: unix, kind, subject, source: event_source(body),
            });
        }
    }
    events.sort_by(|a,b| a.timestamp.cmp(&b.timestamp)
        .then(a.kind.as_str().cmp(b.kind.as_str()))
        .then(a.subject.cmp(&b.subject)));
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

    // These are expected when NetworkManager stops, starts or reconfigures.
    if t.contains("canceled dhcp transaction") || t.contains("state changed no lease")
        || t.contains("unmanaged-quitting") || t.contains("reason 'unmanaged'")
        || t.contains("unavailable -> disconnected") || t.contains("disconnected -> unmanaged")
        || t.contains("disconnected -> prepare") || t.contains("activated -> deactivating")
        || t.contains("exiting (success)") || t.contains("deactivated successfully")
        || t.contains("starting network manager") || t.contains("stopping network manager")
        // NetworkManager logs its configured DHCP transaction deadline using
        // the word "timeout". This is a normal lease acquisition, not a timeout.
        || (t.contains("dhcp") && (
            t.contains("activation: beginning transaction")
            || t.contains("state changed new lease")
        )) {
        return None;
    }

    let nm_link_change = t.contains("device (")
        && (t.contains("activated -> disconnected") || t.contains("activated -> unavailable"));
    // "link down" is also emitted by storage controllers (e.g. "ata3: SATA
    // link down" on an unused SATA port). Require network/interface context
    // for generic "link down" text; otherwise do not report a network fault.
    let generic_link_down = t.contains("link is down") || t.contains("link down");
    let network_context = [
        "networkmanager", "systemd-networkd", "wpa_supplicant",
        "nic link", "network interface", "network link", "ethernet",
        "wi-fi", "wifi", "netdev", "wlan", "wlp", "wlx",
        "eth0", "eth1", "enp", "eno", "ens", "enx",
        "veth", "bond", "br0:", "br-", "bridge",
        "r8169", "e1000e", "igc ", "ixgbe", "virtio_net",
    ].iter().any(|marker| t.contains(marker));
    let storage_link = t.contains("sata link");
    let explicit_link_loss = t.contains("deauthenticat") || t.contains("lost carrier")
        || t.contains("carrier lost") || t.contains("association lost")
        || t.contains("ctrl-event-disconnected")
        || (generic_link_down && network_context && !storage_link);
    if nm_link_change || explicit_link_loss {
        return Some(EventKind::LinkDown);
    }
    let dhcp_error = t.contains("dhcp") && ["failed", "timeout", "timed out", "expired", "unable", "error"].iter().any(|p| t.contains(p));
    if dhcp_error || t.contains("lease lost") || t.contains("failed to renew lease") {
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
    fn attributed_network_and_service_events() {
        let journal = "1700000010.100 host NetworkManager[42]: device (wlan0): state change: activated -> disconnected (reason 'supplicant-disconnect')\n\
1700000010.200 host NetworkManager[42]: device (enp3s0): state change: activated -> disconnected (reason 'carrier')\n\
1700000011.100 host systemd[1]: example-backup.service: Failed with result 'exit-code'.\n";
        let events = parse_journal(journal);
        assert_eq!(events.len(), 3, "events in same second but different interfaces must survive");
        assert_eq!(events[0].subject.as_deref(), Some("enp3s0"));
        assert_eq!(events[1].subject.as_deref(), Some("wlan0"));
        assert_eq!(events[1].source.as_deref(), Some("NetworkManager"));
        assert_eq!(events[2].subject.as_deref(), Some("example-backup.service"));
        assert_eq!(events[2].source.as_deref(), Some("systemd"));
        assert!(events.iter().all(|e| !e.source.as_deref().unwrap_or("").contains("host")));
    }

    #[test]
    fn unknown_subject_is_none_and_sensitive_data_is_not_retained() {
        let event = parse_journal("1700000000.1 host app[91]: Temporary failure in name resolution for secret.corp.example\n");
        assert_eq!(event.len(), 1);
        assert_eq!(event[0].kind, EventKind::DnsProblem);
        assert_eq!(event[0].subject, None);
        assert_eq!(event[0].source.as_deref(), Some("app"));
    }

    #[test]
    fn journal_classification_and_order() {
        let journal = "1700000005.123 host systemd[1]: Failed to start Example Service.\n1700000002.002 host NetworkManager[42]: device (wlan0): state change: activated -> disconnected (reason 'supplicant-disconnect')\n1700000003.002 host systemd-resolved[3]: DNS server unreachable\n";
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
    fn reboot_networkmanager_events_are_not_link_or_dhcp_incidents() {
        let fixture = include_str!("../tests/fixtures/networkmanager-reboot.log");
        let events = parse_journal(fixture);
        assert!(!events.iter().any(|e| matches!(e.kind, EventKind::LinkDown | EventKind::DhcpProblem)), "{events:?}");
    }

    #[test]
    fn genuine_link_and_dhcp_errors_still_detected() {
        let log = "1700000000.1 host NetworkManager[42]: device (wlan0): state change: activated -> disconnected (reason 'supplicant-disconnect')\n1700000090.1 host NetworkManager[42]: dhcp4 (wlan0): request timed out\n";
        let events = parse_journal(log);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].kind, EventKind::LinkDown);
        assert_eq!(events[1].kind, EventKind::DhcpProblem);
    }

    #[test]
    fn dhcp_initial_timeout_budget_is_not_failure() {
        // 2026-11-11 Manjaro / NetworkManager startup: "timeout in 45 seconds"
        // describes a deadline and not a failure; a new DHCPv6 lease follows.
        let startup = "1794436944.6337 host NetworkManager[1046]: <info> dhcp6 (wlan0): activation: beginning transaction (timeout in 45 seconds)\n\
1794436944.6540 host NetworkManager[1046]: <info> dhcp6 (wlan0): state changed new lease\n";
        assert!(parse_journal(startup).is_empty());
        assert_eq!(classify_log_line(
            "NetworkManager[10]: dhcp4 (eth0): activation: beginning transaction (timeout in 45 seconds)"
        ), None);
    }

    #[test]
    fn storage_link_down_at_boot_is_not_a_network_failure() {
        // Captured on Manjaro on 2026-11-11 at 23:39:22 CET.
        let sata = "1794436762.0 host kernel: ata3: SATA link down (SStatus 4 SControl 300)";
        assert_eq!(classify_log_line(sata), None);
        assert!(parse_journal(&format!("{sata}\n")).is_empty());
        for line in [
            "host kernel: ata1: SATA link is down",
            "host kernel: Bluetooth: controller link down",
            "host kernel: scsi disk: link down",
        ] {
            assert_eq!(classify_log_line(line), None, "{line}");
        }
    }

    #[test]
    fn ethernet_link_down_still_generates_network_incidents() {
        for line in [
            "host kernel: e1000e 0000:00:1f.6 eth0: NIC Link is Down",
            "host kernel: igc 0000:02:00.0 enp2s0: Link is Down",
            "host systemd-networkd[345]: eth0: Link DOWN",
            "host NetworkManager[123]: device (enp2s0): link down",
        ] {
            assert_eq!(classify_log_line(line), Some(EventKind::LinkDown), "{line}");
        }
    }

    #[test]
    fn genuine_dhcp_timeout_remains_classified() {
        let failure = "1794437000.1 host NetworkManager[1046]: dhcp6 (wlan0): request timed out";
        let events = parse_journal(failure);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, EventKind::DhcpProblem);
    }

    #[test]
    fn parse_du_sorts_and_limits_subdirectories() {
        let output = "1024\t/var/log\n10000\t/var\n8192\t/var/cache\n4000\t/var/lib/manjaro-tools\n5000\t/var/lib\n";
        let entries = parse_du(output, "/var");
        assert_eq!(entries.len(), 4);
        assert_eq!(entries[0].path, "/var/cache");
        assert_eq!(entries[1].path, "/var/lib");
        assert_eq!(entries[2].path, "/var/lib/manjaro-tools");
    }

    #[test]
    fn parse_failed_unit_list() {
        let content = "● sshd.service loaded failed failed OpenSSH server\n   nginx.service loaded failed failed NGINX\n";
        assert_eq!(parse_failed_units(content), vec!["nginx.service", "sshd.service"]);
    }

    #[test]
    fn df_works() {
        let df = "Filesystem Type 1024-blocks Used Available Capacity Mounted on\n/dev/sda1 ext4 100 95 5 95% /\n";
        let disks = parse_df(df);
        assert_eq!(disks.len(), 1);
        assert_eq!(disks[0].used_percent, 95);
        assert_eq!(disks[0].mountpoint, "/");
    }

    #[test]
    fn ignore_full_read_only_images_but_keep_real_disks() {
        // Reproduces a real Manjaro report with Snap mounts at 100%.
        let df = "Filesystem Type 1024-blocks Used Available Capacity Mounted on\n\
/dev/nvme0n1p2 ext4 10000 9000 1000 90% /var\n\
/dev/loop0 squashfs 128 128 0 100% /var/lib/snapd/snap/bare/5\n\
/dev/loop1 squashfs 512 512 0 100% /var/lib/snapd/snap/core22/2955\n\
/dev/loop2 erofs 512 512 0 100% /some/immutable/image\n\
/dev/sr0 iso9660 1000 1000 0 100% /mnt/cdrom\n\
/dev/loop3 cramfs 1024 1024 0 100% /mnt/compressed\n\
overlay overlay 10000 9100 900 91% /containers\n\
tmpfs tmpfs 1000 920 80 92% /run\n";
        let disks = parse_df(df);
        assert_eq!(disks.len(), 3);
        assert_eq!(disks[0].mountpoint, "/var");
        assert_eq!(disks[0].used_percent, 90);
        assert_eq!(disks[1].mountpoint, "/containers");
        assert_eq!(disks[2].mountpoint, "/run");
    }

    #[test]
    fn untyped_df_output_is_not_misinterpreted() {
        let df = "Filesystem 1024-blocks Used Available Capacity Mounted on\n/dev/sda1 100 95 5 95% /\n";
        assert!(parse_df(df).is_empty());
    }
}
