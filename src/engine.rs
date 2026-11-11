//! Deterministic diagnostic rules. Each finding separates observation from hypothesis.

use crate::model::{Confidence, Event, EventKind, Finding, Focus, NetworkIncident, Report, Severity, Snapshot};
use crate::time::{now_unix, utc_timestamp};

pub fn analyze(snapshot: Snapshot, lookback_seconds: u64, focus: Focus, demo: bool) -> Report {
    let mut findings = Vec::new();
    let events: Vec<Event> = snapshot.events.iter().filter(|e| focus.includes(e.kind.focus()))
        .cloned().collect();

    if focus.includes(Focus::Network) {
        diagnose_network(&snapshot, &events, &mut findings);
    }
    if focus.includes(Focus::Services) {
        diagnose_services(&snapshot, &events, &mut findings);
    }
    if focus.includes(Focus::System) {
        diagnose_system(&snapshot, &events, &mut findings);
    }

    if findings.is_empty() {
        findings.push(Finding {
            id: "NO_CLASSIFIED_ANOMALY",
            severity: Severity::Info,
            confidence: Confidence::Limited,
            title: "No classified anomaly found".into(),
            explanation: "No rule matched the available snapshot and selected journal entries. This does not establish that the system is healthy.".into(),
            evidence: vec!["The available observations did not match a known diagnostic rule".into()],
            checks: vec!["Increase --since or --max-events, inspect application-specific logs, and reproduce the failure if safe".into()],
        });
    }

    let network_incidents = if focus.includes(Focus::Network) {
        let groups = group_network_incidents(&events, 120);
        groups.into_iter().rev().take(30).collect::<Vec<_>>()
            .into_iter().rev().collect()
    } else { Vec::new() };
    let timeline = events.into_iter().rev().take(60).collect::<Vec<_>>()
        .into_iter().rev().collect();

    Report {
        version: env!("CARGO_PKG_VERSION"),
        generated_at: now_unix(),
        lookback_seconds,
        focus,
        demo,
        snapshot,
        findings,
        timeline,
        network_incidents,
    }
}

/// Group nearby network observations into possible incident windows. These are
/// temporal clusters, not verified causal chains or proof of continued outage.
pub fn group_network_incidents(events: &[Event], max_gap_seconds: u64) -> Vec<NetworkIncident> {
    let mut groups: Vec<NetworkIncident> = Vec::new();
    let mut sorted: Vec<&Event> = events.iter()
        .filter(|event| event.kind.focus() == Focus::Network).collect();
    sorted.sort_by_key(|event| event.timestamp);
    for event in sorted {
        // Group by interface when known; never silently combine wlan0 with eth0.
        let position = groups.iter().rposition(|group| {
            group.subject == event.subject
                && event.timestamp >= group.end
                && event.timestamp - group.end <= max_gap_seconds
        });
        let index = match position {
            Some(index) => index,
            None => {
                groups.push(NetworkIncident {
                    subject: event.subject.clone(),
                    start: event.timestamp, end: event.timestamp,
                    link_events: 0, dhcp_events: 0, dns_events: 0, unreachable_events: 0,
                });
                groups.len() - 1
            }
        };
        if let Some(group) = groups.get_mut(index) {
            group.end = event.timestamp;
            match event.kind {
                EventKind::LinkDown => group.link_events += 1,
                EventKind::DhcpProblem => group.dhcp_events += 1,
                EventKind::DnsProblem => group.dns_events += 1,
                EventKind::NetworkUnreachable => group.unreachable_events += 1,
                _ => {}
            }
        }
    }
    groups.sort_by_key(|group| (group.start, group.end));
    groups
}

fn event_subjects(events: &[Event], kind: EventKind) -> Vec<String> {
    let mut subjects: Vec<String> = events.iter().filter(|e| e.kind == kind)
        .filter_map(|e| e.subject.clone()).collect();
    subjects.sort();
    subjects.dedup();
    subjects.truncate(8);
    subjects
}

fn count(events: &[Event], kind: EventKind) -> usize {
    events.iter().filter(|e| e.kind == kind).count()
}

fn timestamps(events: &[Event], kind: EventKind, limit: usize) -> Vec<String> {
    events.iter().filter(|e| e.kind == kind).take(limit)
        .map(|e| utc_timestamp(e.timestamp)).collect()
}

fn diagnose_network(snapshot: &Snapshot, events: &[Event], findings: &mut Vec<Finding>) {
    if snapshot.ipv4_default_route == Some(false) && snapshot.ipv6_default_route == Some(false) {
        findings.push(Finding {
            id: "ROUTE_MISSING",
            severity: Severity::Warning,
            confidence: Confidence::Observed,
            title: "No default IPv4 or IPv6 route is currently present".into(),
            explanation: "The routing tables expose no active default route. Internet destinations may still be reachable via specific routes, tunnels, or separate network namespaces.".into(),
            evidence: vec!["Neither /proc/net/route nor /proc/net/ipv6_route showed a default route".into()],
            checks: vec!["Inspect `ip route` and `ip -6 route`".into(), "Check the intended gateway and network interface state".into()],
        });
    } else if snapshot.ipv4_default_route == Some(false) && snapshot.ipv6_default_route != Some(true) {
        findings.push(Finding {
            id: "IPV4_ROUTE_MISSING",
            severity: Severity::Info,
            confidence: Confidence::Observed,
            title: "No default IPv4 route is currently present".into(),
            explanation: "This may be normal on an IPv6-only host or an isolated machine.".into(),
            evidence: vec!["The IPv4 routing table has no active default route".into()],
            checks: vec!["Inspect `ip -4 route` to confirm the expected network design".into()],
        });
    }

    if snapshot.dns_nameservers == Some(0) {
        findings.push(Finding {
            id: "DNS_UNCONFIGURED",
            severity: Severity::Warning,
            confidence: Confidence::Observed,
            title: "No nameserver entry in /etc/resolv.conf".into(),
            explanation: "Conventional libc DNS lookups may fail or use a resolver mechanism not represented by this file. Some applications use a separate DNS configuration.".into(),
            evidence: vec!["No uncommented `nameserver` directive was found".into()],
            checks: vec!["Inspect `resolvectl status` or your network manager's DNS configuration".into()],
        });
    }

    if snapshot.dns_stub && snapshot.resolved_active == Some(false) {
        findings.push(Finding {
            id: "DNS_STUB_INACTIVE",
            severity: Severity::Warning,
            confidence: Confidence::Observed,
            title: "Local systemd-resolved DNS stub appears inactive".into(),
            explanation: "/etc/resolv.conf points to the local systemd-resolved stub, but its systemd service is not active. DNS resolution may be affected.".into(),
            evidence: vec!["Stub address configured; systemd-resolved.service was not active".into()],
            checks: vec!["Inspect `systemctl status systemd-resolved.service`".into(), "Inspect `resolvectl status` and `/etc/resolv.conf`".into()],
        });
    }

    let links = count(events, EventKind::LinkDown);
    let dhcp = count(events, EventKind::DhcpProblem);
    let dns = count(events, EventKind::DnsProblem);
    let unreachable = count(events, EventKind::NetworkUnreachable);

    if links > 0 {
        let interfaces = event_subjects(events, EventKind::LinkDown);
        findings.push(Finding {
            id: "LINK_DISCONNECTS",
            severity: Severity::Warning,
            confidence: Confidence::Observed,
            title: "Network link disconnections were reported".into(),
            explanation: "The selected journal contains messages consistent with link loss or Wi-Fi disconnection. The physical cause cannot be established from these messages alone.".into(),
            evidence: {
                let mut items = vec![
                    format!("{links} classified disconnection event(s) across subject/second pairs"),
                    format!("First observed: {}", timestamps(events, EventKind::LinkDown, 1).join(", ")),
                ];
                if !interfaces.is_empty() { items.push(format!("Interface(s): {}", interfaces.join(", "))); }
                items
            },
            checks: vec!["Inspect access point/router logs and interface carrier or Wi-Fi signal".into(), "Check NetworkManager or systemd-networkd logs around the timestamps".into()],
        });
    }

    if dhcp > 0 {
        findings.push(Finding {
            id: "DHCP_FAILURES",
            severity: Severity::Warning,
            confidence: Confidence::Observed,
            title: "DHCP failures were reported".into(),
            explanation: "DHCP-related failures appear in the journal. This could reflect a router, link, lease or client issue; further checks are required.".into(),
            evidence: {
                let mut items = vec![format!("{dhcp} classified DHCP event(s)")];
                let subjects = event_subjects(events, EventKind::DhcpProblem);
                if !subjects.is_empty() { items.push(format!("Interface(s): {}", subjects.join(", "))); }
                items
            },
            checks: vec!["Inspect DHCP lease status and client logs".into(), "Check DHCP server/router health and whether the link stayed up".into()],
        });
    }

    if dns > 0 {
        findings.push(Finding {
            id: "DNS_FAILURES",
            severity: Severity::Warning,
            confidence: Confidence::Observed,
            title: "DNS lookup failures were reported".into(),
            explanation: "Name resolution failures occurred in the selected journal. DNS errors can be a downstream symptom of a broader connection outage.".into(),
            evidence: vec![format!("{dns} distinct second(s) with classified DNS errors")],
            checks: vec!["Compare resolution of a known hostname with reachability of its DNS server".into(), "Inspect `resolvectl status` and routing during the incident".into()],
        });
    }

    if unreachable > 0 {
        findings.push(Finding {
            id: "NETWORK_UNREACHABLE",
            severity: Severity::Warning,
            confidence: Confidence::Observed,
            title: "Network unreachable errors were reported".into(),
            explanation: "One or more processes reported network reachability failures in the journal.".into(),
            evidence: vec![format!("{unreachable} distinct second(s) with unreachable messages")],
            checks: vec!["Review interface connectivity and the gateway at the recorded time".into()],
        });
    }

    if let Some((first, later)) = adjacent_events(events, EventKind::LinkDown, EventKind::DnsProblem, 120) {
        findings.push(Finding {
            id: "LINK_BEFORE_DNS",
            severity: Severity::Info,
            confidence: Confidence::Moderate,
            title: "A link interruption preceded a DNS failure".into(),
            explanation: "The ordering suggests that at least one DNS failure could be a symptom of network disconnection, rather than an independent DNS-server issue. This is correlation, not proof of causation.".into(),
            evidence: vec![format!("Link event: {}", utc_timestamp(first)), format!("DNS event: {} ({} s later)", utc_timestamp(later), later - first)],
            checks: vec!["Investigate link stability and DHCP before changing DNS servers".into()],
        });
    } else if let Some((first, later)) = adjacent_events(events, EventKind::DhcpProblem, EventKind::DnsProblem, 120) {
        findings.push(Finding {
            id: "DHCP_BEFORE_DNS",
            severity: Severity::Info,
            confidence: Confidence::Moderate,
            title: "A DHCP problem preceded a DNS failure".into(),
            explanation: "A DHCP problem occurred shortly before a DNS error. The DNS symptom may reflect lost network configuration; correlation does not prove a causal relationship.".into(),
            evidence: vec![format!("DHCP event: {}", utc_timestamp(first)), format!("DNS event: {} ({} s later)", utc_timestamp(later), later - first)],
            checks: vec!["Review DHCP lease transitions and DNS configuration at both timestamps".into()],
        });
    }

    for iface in &snapshot.interfaces {
        if iface.rx_errors > 0 || iface.tx_errors > 0 {
            findings.push(Finding {
                id: "IFACE_ERROR_COUNTERS",
                severity: Severity::Info,
                confidence: Confidence::Observed,
                title: format!("Nonzero error counters on interface {}", iface.name),
                explanation: "These are cumulative kernel counters, not necessarily errors that happened during the requested time window. A second sample is required to detect new errors.".into(),
                evidence: vec![format!("RX errors: {}; TX errors: {}", iface.rx_errors, iface.tx_errors)],
                checks: vec![format!("Sample `ip -s link show dev {}` again and compare error counts", iface.name)],
            });
        }
    }
}

fn adjacent_events(events: &[Event], before: EventKind, after: EventKind, max_delta: u64) -> Option<(u64, u64)> {
    // The most recent preceding qualifying event must be in the bounded time window.
    let mut previous = None;
    for event in events {
        if event.kind == before { previous = Some(event.timestamp); }
        if event.kind == after {
            if let Some(timestamp) = previous {
                if event.timestamp >= timestamp && event.timestamp - timestamp <= max_delta {
                    return Some((timestamp, event.timestamp));
                }
            }
        }
    }
    None
}

fn diagnose_services(snapshot: &Snapshot, events: &[Event], findings: &mut Vec<Finding>) {
    if !snapshot.failed_units.is_empty() {
        findings.push(Finding {
            id: "FAILED_SYSTEMD_UNITS",
            severity: Severity::Warning,
            confidence: Confidence::Observed,
            title: format!("{} currently failed systemd unit(s)", snapshot.failed_units.len()),
            explanation: "systemd currently reports failed units. Their relation to the reported incident is not established.".into(),
            evidence: snapshot.failed_units.iter().take(10).cloned().collect(),
            checks: vec!["Inspect `systemctl --failed` and `journalctl -u <unit>`".into()],
        });
    }
    let failures = count(events, EventKind::ServiceFailure);
    if failures > 0 {
        findings.push(Finding {
            id: "SERVICE_FAILURE_LOGS",
            severity: Severity::Warning,
            confidence: Confidence::Observed,
            title: "Service failures appeared in the journal".into(),
            explanation: "Service errors were observed, but the message classification alone does not identify which service or the underlying cause.".into(),
            evidence: {
                let mut items = vec![format!("{failures} classified service-failure event(s)")];
                let subjects = event_subjects(events, EventKind::ServiceFailure);
                if !subjects.is_empty() { items.push(format!("Unit(s): {}", subjects.join(", "))); }
                items
            },
            checks: vec!["Inspect the relevant journal entries and failed unit details".into()],
        });
    }
}

fn diagnose_system(snapshot: &Snapshot, events: &[Event], findings: &mut Vec<Finding>) {
    for disk in &snapshot.disks {
        if disk.used_percent >= 90 {
            let mut evidence = vec![format!("`df -PT` reports {}% used", disk.used_percent)];
            let mut checks = vec!["Inspect `df -hT` and the largest directories before deleting anything".into()];
            if let Some(details) = snapshot.disk_breakdowns.iter().find(|d| d.mountpoint == disk.mountpoint) {
                for item in details.entries.iter().take(6) {
                    evidence.push(format!("Directory {}: {:.1} GiB (recursive du estimate, nested sizes overlap)",
                        item.path, item.kib as f64 / 1_048_576.0));
                }
                if details.entries.iter().any(|d| d.path.starts_with("/var/cache")) {
                    checks.push("Review package caches; on Arch/Manjaro preview pacman cache candidates with `paccache -d` (if installed)".into());
                }
                if details.entries.iter().any(|d| d.path.contains("/manjaro-tools")) {
                    checks.push("Inspect old manjaro-tools build/output directories before manually removing anything".into());
                }
                if details.entries.iter().any(|d| d.path.contains("/docker")) {
                    checks.push("Inspect Docker space with `docker system df`; avoid removing live volumes".into());
                }
                if details.entries.iter().any(|d| d.path.starts_with("/var/log")) {
                    checks.push("Check journal retention with `journalctl --disk-usage`".into());
                }
            } else {
                checks.push("For an optional read-only directory breakdown, rerun with `--disk-details` (can take time)".into());
            }
            findings.push(Finding {
                id: "DISK_NEAR_FULL",
                severity: Severity::Warning,
                confidence: Confidence::Observed,
                title: format!("Filesystem usage is {}% at {}", disk.used_percent, disk.mountpoint),
                explanation: "This filesystem is at or above 90% capacity. High utilization can affect services, but it is not proof of an incident. Optional du breakdowns may differ from df due to metadata, snapshots, sparse files, permissions and concurrent writes.".into(),
                evidence,
                checks,
            });
        }
    }
    for (kind, id, title, explain, check) in [
        (EventKind::OutOfMemory, "OOM_EVENTS", "Out-of-memory events reported", "The journal contains messages consistent with memory exhaustion or process termination by an OOM killer.", "Review memory pressure and kernel logs around the event"),
        (EventKind::StorageError, "STORAGE_ERRORS", "Storage errors reported", "The journal contains storage-related error messages; their severity and affected device are not determined by this classifier.", "Inspect kernel storage logs, available disk space and hardware health"),
    ] {
        let n = count(events, kind);
        if n > 0 {
            findings.push(Finding {
                id,
                severity: Severity::Warning,
                confidence: Confidence::Observed,
                title: title.into(),
                explanation: explain.into(),
                evidence: vec![format!("{n} distinct second(s) classified as {}", kind.as_str())],
                checks: vec![check.into()],
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correlate_link_before_dns() {
        let events = vec![
            Event { timestamp: 100, kind: EventKind::LinkDown, subject: None, source: None },
            Event { timestamp: 130, kind: EventKind::DnsProblem, subject: None, source: None },
        ];
        assert_eq!(adjacent_events(&events, EventKind::LinkDown, EventKind::DnsProblem, 120), Some((100, 130)));
        assert_eq!(adjacent_events(&events, EventKind::DnsProblem, EventKind::LinkDown, 120), None);
    }

    #[test]
    fn no_false_causality() {
        let events = vec![
            Event { timestamp: 100, kind: EventKind::LinkDown, subject: None, source: None },
            Event { timestamp: 500, kind: EventKind::DnsProblem, subject: None, source: None },
        ];
        assert_eq!(adjacent_events(&events, EventKind::LinkDown, EventKind::DnsProblem, 120), None);
    }

    #[test]
    fn network_incident_windows_separate_interfaces() {
        let events = vec![
            Event { timestamp: 100, kind: EventKind::LinkDown, subject: Some("wlan0".into()), source: None },
            Event { timestamp: 105, kind: EventKind::LinkDown, subject: Some("enp3s0".into()), source: None },
            Event { timestamp: 130, kind: EventKind::DhcpProblem, subject: Some("wlan0".into()), source: None },
        ];
        let groups = group_network_incidents(&events, 120);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].subject.as_deref(), Some("wlan0"));
        assert_eq!(groups[0].event_count(), 2);
        assert_eq!(groups[1].subject.as_deref(), Some("enp3s0"));
        assert_eq!(groups[1].event_count(), 1);
    }

    #[test]
    fn network_incidents_group_temporally_and_remain_historical() {
        let events = vec![
            Event { timestamp: 100, kind: EventKind::LinkDown, subject: None, source: None },
            Event { timestamp: 120, kind: EventKind::DhcpProblem, subject: None, source: None },
            Event { timestamp: 150, kind: EventKind::DnsProblem, subject: None, source: None },
            Event { timestamp: 400, kind: EventKind::DhcpProblem, subject: None, source: None },
            Event { timestamp: 405, kind: EventKind::ServiceFailure, subject: None, source: None },
        ];
        let groups = group_network_incidents(&events, 120);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].event_count(), 3);
        assert_eq!(groups[0].summary(), "link / DHCP / DNS");
        assert_eq!(groups[0].start, 100);
        assert_eq!(groups[0].end, 150);
        assert_eq!(groups[1].event_count(), 1);
        assert_eq!(groups[1].start, 400);
    }

    #[test]
    fn disk_breakdown_keeps_real_warning_and_explains_space() {
        use crate::model::{DirectoryUsage, DiskBreakdown, DiskUsage};
        let snapshot = Snapshot {
            disks: vec![DiskUsage { mountpoint: "/var".into(), used_percent: 90 }],
            disk_breakdowns: vec![DiskBreakdown {
                mountpoint: "/var".into(),
                entries: vec![DirectoryUsage {path: "/var/cache".into(), kib: 26 * 1_048_576}],
            }],
            ..Snapshot::default()
        };
        let report = analyze(snapshot, 3600, Focus::System, false);
        let warning = report.findings.iter().find(|f| f.id == "DISK_NEAR_FULL").unwrap();
        assert!(warning.evidence.iter().any(|s| s.contains("26.0 GiB")));
        assert!(warning.checks.iter().any(|s| s.contains("paccache -d")));
    }

    #[test]
    fn ipv6_only_is_not_marked_broken() {
        let snapshot = Snapshot {
            ipv4_default_route: Some(false),
            ipv6_default_route: Some(true),
            dns_nameservers: Some(1),
            ..Snapshot::default()
        };
        let report = analyze(snapshot, 60, Focus::Network, true);
        assert!(!report.findings.iter().any(|f| f.id == "ROUTE_MISSING" || f.id == "IPV4_ROUTE_MISSING"));
    }

    #[test]
    fn explicit_no_anomaly_message() {
        let report = analyze(Snapshot::default(), 60, Focus::All, true);
        assert_eq!(report.findings[0].id, "NO_CLASSIFIED_ANOMALY");
    }
}
