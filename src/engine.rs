//! Deterministic diagnostic rules. Each finding separates observation from hypothesis.

use crate::model::{count_word, Confidence, Event, EventKind, Finding, Focus, NetworkIncident, Report, ServiceEventWindow, Severity, Snapshot};
use crate::time::{now_unix, utc_timestamp};

pub fn analyze(snapshot: Snapshot, lookback_seconds: u64, focus: Focus, demo: bool) -> Report {
    let generated_at = now_unix();
    let mut findings = Vec::new();
    let events: Vec<Event> = snapshot.events.iter().filter(|e| focus.includes(e.kind.focus()))
        .cloned().collect();

    if focus.includes(Focus::Network) {
        diagnose_network(&snapshot, &events, &mut findings);
    }
    if focus.includes(Focus::Services) {
        diagnose_services(&snapshot, &events, &mut findings, generated_at);
    }
    if focus.includes(Focus::System) {
        diagnose_system(&snapshot, &events, &mut findings);
    }

    if findings.is_empty() {
        // Do not present a "no anomaly" finding when core collectors were
        // unavailable. Missing optional sensors still remain explicitly unknown.
        let mut unavailable = Vec::new();
        if !snapshot.journal_checked { unavailable.push("journalctl not verified".to_string()); }
        if snapshot.journal_truncated {
            unavailable.push(format!(
                "journal window truncated: only newest {} entries examined",
                snapshot.journal_entries_examined
            ));
        }
        if focus.includes(Focus::System) && !snapshot.disk_usage_checked {
            unavailable.push("filesystem capacity not verified".to_string());
        }
        if focus.includes(Focus::Services) && !snapshot.system_units_checked
            && snapshot.failed_user_units.is_none() {
            unavailable.push("systemd units not verified".to_string());
        }
        if focus.includes(Focus::Network) &&
            snapshot.ipv4_default_route.is_none() && snapshot.ipv6_default_route.is_none() {
            unavailable.push("network routes not verified".to_string());
        }
        if focus.includes(Focus::Network) && snapshot.dns_nameservers.is_none() {
            unavailable.push("DNS configuration not verified".to_string());
        }
        if unavailable.is_empty() {
            findings.push(Finding {
                id: "NO_CLASSIFIED_ANOMALY",
                severity: Severity::Info,
                confidence: Confidence::Limited,
                title: "No classified anomaly found".into(),
                explanation: "No rule matched the available snapshot and selected journal entries. This does not establish that the system is healthy.".into(),
                evidence: vec!["The available observations did not match a known diagnostic rule".into()],
                checks: vec!["Increase --since or --max-events, inspect application-specific logs, and reproduce the failure if safe".into()],
            });
        } else {
            findings.push(Finding {
                id: "INSUFFICIENT_EVIDENCE",
                severity: Severity::Info,
                confidence: Confidence::Limited,
                title: "Core diagnostic collection was incomplete".into(),
                explanation: "WhyDown cannot determine whether a known rule matches when core evidence is unavailable. This is not a positive health check or a confirmed system fault.".into(),
                evidence: unavailable,
                checks: vec!["Review COLLECTION LIMITATIONS and permissions; rerun only the relevant focus after resolving collector access".into()],
            });
        }
    } else if snapshot.journal_truncated {
        // Warnings or other findings remain valid, but the journal may
        // contain additional earlier incidents that were not examined.
        findings.push(Finding {
            id: "JOURNAL_WINDOW_TRUNCATED",
            severity: Severity::Info,
            confidence: Confidence::Limited,
            title: "Journal analysis covered only the newest entries".into(),
            explanation: "The journal window contains more entries than the selected limit. Existing findings are based on the examined tail only, and earlier incidents may be missing.".into(),
            evidence: vec![format!(
                "Newest {} journal entries examined; at least one older entry excluded",
                snapshot.journal_entries_examined
            )],
            checks: vec![
                "Increase --max-events up to 10000 or narrow --since to inspect an earlier time window".into(),
            ],
        });
    }

    let network_incidents = if focus.includes(Focus::Network) {
        let groups = group_network_incidents(&events, 120);
        groups.into_iter().rev().take(30).collect::<Vec<_>>()
            .into_iter().rev().collect()
    } else { Vec::new() };
    let service_event_windows = if focus.includes(Focus::Services) {
        group_service_messages(&events, 5).into_iter().rev().take(30)
            .collect::<Vec<_>>().into_iter().rev().collect()
    } else { Vec::new() };
    let timeline = events.into_iter().rev().take(300).collect::<Vec<_>>()
        .into_iter().rev().collect();

    Report {
        version: env!("CARGO_PKG_VERSION"),
        generated_at,
        lookback_seconds,
        focus,
        demo,
        snapshot,
        findings,
        timeline,
        network_incidents,
        service_event_windows,
    }
}

/// Group nearby network observations into possible incident windows. These are
/// temporal clusters, not verified causal chains or proof of continued outage.
pub fn group_network_incidents(events: &[Event], max_gap_seconds: u64) -> Vec<NetworkIncident> {
    let mut groups: Vec<NetworkIncident> = Vec::new();
    let mut sorted: Vec<&Event> = events.iter()
        .filter(|event| matches!(event.kind,
            EventKind::LinkDown | EventKind::DhcpProblem | EventKind::DnsProblem
            | EventKind::NetworkUnreachable)).collect();
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

/// A grouping of nearby service messages is not an independently failed unit.
pub fn group_service_messages(events: &[Event], gap_seconds: u64) -> Vec<ServiceEventWindow> {
    let mut selected: Vec<&Event> = events.iter()
        .filter(|e| e.kind == EventKind::ServiceFailure).collect();
    selected.sort_by_key(|e| e.timestamp);
    let mut groups: Vec<ServiceEventWindow> = Vec::new();
    for event in selected {
        let append = groups.last().is_some_and(|g| {
            event.timestamp >= g.end && event.timestamp - g.end <= gap_seconds
        });
        if !append {
            groups.push(ServiceEventWindow {
                start: event.timestamp, end: event.timestamp,
                entry_count: 0, unattributed_entries: 0, units: Vec::new(),
            });
        }
        if let Some(group) = groups.last_mut() {
            group.end = event.timestamp;
            group.entry_count += 1;
            if let Some(unit) = &event.subject {
                if !group.units.contains(unit) { group.units.push(unit.clone()); }
            } else {
                group.unattributed_entries += 1;
            }
        }
    }
    for group in &mut groups { group.units.sort(); }
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
            title: "No valid nameserver address in /etc/resolv.conf".into(),
            explanation: "Conventional libc DNS lookups may fail or use a resolver mechanism not represented by this file. Some applications use a separate DNS configuration.".into(),
            evidence: vec!["No valid IP literal was found in an uncommented `nameserver` directive".into()],
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
            title: if interfaces.len() == 1 { format!("Network link interruption on {}", interfaces[0]) } else { "Network link disconnections were reported".into() },
            explanation: "The selected journal contains messages consistent with link loss or Wi-Fi disconnection. The physical cause cannot be established from these messages alone.".into(),
            evidence: {
                let mut items = vec![
                    format!("{links} classified disconnection event(s) across subject/second pairs"),
                    format!("First observed: {}", timestamps(events, EventKind::LinkDown, 1).join(", ")),
                ];
                if !interfaces.is_empty() { items.push(format!("Interface(s): {}", interfaces.join(", "))); }
                if let Some((before, after, iface)) = nearby_matching_subjects(
                    events, EventKind::LinkDown, EventKind::WifiAssociated, 120
                ) {
                    if after == before {
                        // Journal timestamps are retained at whole-second resolution.
                        // An association in the same second cannot prove which came first.
                        items.push(format!(
                            "Wi-Fi association and link event on {iface} logged in the same second at {}; ordering unknown; Internet access not verified",
                            utc_timestamp(after)
                        ));
                    } else {
                        items.push(format!(
                            "Subsequent Wi-Fi association on {iface}: {} ({} seconds after link event); Internet access not verified",
                            utc_timestamp(after), after - before
                        ));
                    }
                }
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
            title: { let interfaces = event_subjects(events, EventKind::DhcpProblem); if interfaces.len() == 1 { format!("DHCP problem on {}", interfaces[0]) } else { "DHCP failures were reported".into() } },
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

    if let Some((start, end, iface)) = nearby_matching_subjects(
        events, EventKind::WifiRoamStart, EventKind::WifiAssociated, 30
    ) {
        findings.push(Finding {
            id: "WIFI_LOCAL_ROAM_OBSERVED",
            severity: Severity::Info,
            confidence: Confidence::Observed,
            title: "Wi-Fi local-choice handoff and nearby association observed".into(),
            explanation: "The driver reported deauthentication by local choice and a nearby association on the same interface. Their order cannot be established if both were logged in the same second. This may be normal access-point roaming; it does not prove Internet connectivity.".into(),
            evidence: vec![if end == start {
                format!(
                    "{iface}: local-choice deauthentication and association logged in the same second at {}; order not verified",
                    utc_timestamp(start)
                )
            } else {
                format!(
                    "{iface}: local-choice deauthentication at {} and association {} seconds later",
                    utc_timestamp(start), end - start
                )
            }],
            checks: vec!["Inspect NetworkManager roaming decisions and access-point logs only if disruptions are observed".into()],
        });
    }
    if let Some(deltas) = &snapshot.interface_drop_deltas {
        for delta in deltas.iter().filter(|d| d.elapsed_ms >= 100
            && d.rx_dropped_delta.saturating_add(d.tx_dropped_delta) >= 10) {
            findings.push(Finding {
                id: "RECENT_INTERFACE_DROPS",
                severity: Severity::Info,
                confidence: Confidence::Observed,
                title: format!("Packet-drop counters increased on {}", delta.name),
                explanation: "Two kernel counter samples taken during this run show an increase. Interface drops do not necessarily mean end-to-end packet loss.".into(),
                evidence: vec![format!(
                    "{}: +{} RX drops, +{} TX drops over {} ms",
                    delta.name, delta.rx_dropped_delta, delta.tx_dropped_delta, delta.elapsed_ms
                )],
                checks: vec![format!("Compare repeated ip -s link show dev {} samples and driver logs", delta.name)],
            });
        }
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

/// Pair observations only on the same explicitly identified interface.
fn nearby_matching_subjects(
    events: &[Event], before: EventKind, after: EventKind, max_gap: u64,
) -> Option<(u64, u64, String)> {
    events.iter().filter(|e| e.kind == before).find_map(|first| {
        let iface = first.subject.as_ref()?;
        let later = events.iter().filter(|e|
            e.kind == after && e.subject.as_ref() == Some(iface)
                && e.timestamp >= first.timestamp
                && e.timestamp - first.timestamp <= max_gap
        ).min_by_key(|e| e.timestamp)?;
        Some((first.timestamp, later.timestamp, iface.clone()))
    })
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

fn diagnose_services(snapshot: &Snapshot, events: &[Event], findings: &mut Vec<Finding>, generated_at: u64) {
    if !snapshot.failed_units.is_empty() {
        findings.push(Finding {
            id: "FAILED_SYSTEMD_UNITS",
            severity: Severity::Warning,
            confidence: Confidence::Observed,
            title: format!("{} currently failed systemd {}", snapshot.failed_units.len(), count_word(snapshot.failed_units.len(), "unit", "units")),
            explanation: "systemd currently reports failed units. Their relation to the reported incident is not established.".into(),
            evidence: snapshot.failed_units.iter().take(10).cloned().collect(),
            checks: vec!["Inspect `systemctl --failed` and `journalctl -u <unit>`".into()],
        });
    }
    if let Some(user_failed) = &snapshot.failed_user_units {
        if !user_failed.is_empty() {
            findings.push(Finding {
                id: "FAILED_USER_SYSTEMD_UNITS",
                severity: Severity::Warning,
                confidence: Confidence::Observed,
                title: format!("{} currently failed user systemd {}", user_failed.len(), count_word(user_failed.len(), "unit", "units")),
                explanation: "The invoking user's systemd manager reports currently failed units; separate from system units.".into(),
                evidence: user_failed.iter().take(10).cloned().collect(),
                checks: vec!["Inspect `systemctl --user --failed` and the user journal".into()],
            });
        }
    }
    let failures = count(events, EventKind::ServiceFailure);
    if failures > 0 {
        let distinct_seconds: std::collections::HashSet<u64> = events.iter()
            .filter(|e| e.kind == EventKind::ServiceFailure)
            .map(|e| e.timestamp).collect();
        let windows = group_service_messages(events, 5);
        let unknown = events.iter().filter(|e| {
            e.kind == EventKind::ServiceFailure && e.subject.is_none()
        }).count();
        let matched_restarts: Vec<(String, u64)> = events.iter()
            .filter(|e| e.kind == EventKind::ServiceFailure)
            .filter_map(|failure| {
                let unit = failure.subject.as_ref()?;
                let restart = events.iter()
                    .filter(|e| e.kind == EventKind::ServiceRestartScheduled
                        && e.subject.as_ref() == Some(unit)
                        && e.timestamp >= failure.timestamp
                        && e.timestamp - failure.timestamp <= 120)
                    .min_by_key(|e| e.timestamp)?;
                Some((unit.clone(), restart.timestamp))
            })
            .take(8).collect();
        findings.push(Finding {
            id: "SERVICE_FAILURE_LOGS",
            severity: Severity::Warning,
            confidence: Confidence::Observed,
            title: { let units = event_subjects(events, EventKind::ServiceFailure); if units.len() == 1 { format!("Historical service failure: {}", units[0]) } else if units.len() > 1 { format!("Historical service failures: {} identified units", units.len()) } else { "Historical service-failure messages appeared in the journal".into() } },
            explanation: "Classified journal entries are NOT a count of independently failed services. Multiple units and unrelated generic messages may occur at the same time. Current system/user failed-unit states are checked separately when available.".into(),
            evidence: {
                let mut evidence = vec![
                    format!("{failures} classified {} across {} distinct {} in {} time {}",
                        count_word(failures, "entry", "entries"),
                        distinct_seconds.len(), count_word(distinct_seconds.len(), "second", "seconds"),
                        windows.len(), count_word(windows.len(), "window", "windows")),
                    format!("{unknown} {} without an identified unit (possibly related or unrelated)",
                        count_word(unknown, "entry", "entries")),
                ];
                let names = event_subjects(events, EventKind::ServiceFailure);
                if !names.is_empty() { evidence.push(format!("Identified unit(s): {}", names.join(", "))); }
                for (unit, when) in &matched_restarts {
                    evidence.push(format!("{}: restart scheduled at {}; this alone does not confirm recovery",
                        unit, utc_timestamp(*when)));
                    if let Some(state) = snapshot.service_statuses.iter().find(|s| &s.unit == unit) {
                        evidence.push(format!(
                            "{}: current system-manager state is {}/{} (result={}, restarts={}); this is not application readiness",
                            unit, state.active_state, state.sub_state, state.result,
                            state.n_restarts.map(|n| n.to_string()).unwrap_or_else(|| "unknown".into())
                        ));
                    }
                }
                if snapshot.system_units_checked && snapshot.failed_units.is_empty() {
                    evidence.push("Current systemd snapshot has zero failed system units; past failures may have recovered".into());
                }
                if let Some(uptime) = snapshot.uptime_seconds {
                    let estimated_boot = generated_at.saturating_sub(uptime);
                    let before_boot = events.iter().filter(|e|
                        e.kind == EventKind::ServiceFailure && e.timestamp < estimated_boot).count();
                    if before_boot > 0 {
                        evidence.push(format!(
                            "{before_boot} classified {} {} the estimated current boot (based on uptime; system clock changes can affect this)",
                            count_word(before_boot, "entry", "entries"),
                            count_word(before_boot, "predates", "predate")
                        ));
                    }
                }
                evidence
            },
            checks: vec![
                "Compare current `systemctl --failed` and `systemctl --user --failed` state".into(),
                "Check surrounding journal lines: 15/TERM can occur during shutdown; status=1/FAILURE is a real process exit failure".into(),
                "Inspect automatic restart policies and later recovery messages".into(),
            ],
        });
    }
    if let Some((unit, when)) = events.iter()
        .filter(|e| e.kind == EventKind::ServiceRestartScheduled)
        .find_map(|restart| {
            let unit = restart.subject.as_ref()?;
            if snapshot.system_units_checked && !snapshot.failed_units.contains(unit) {
                Some((unit.clone(), restart.timestamp))
            } else { None }
        }) {
        let state = snapshot.service_statuses.iter().find(|s| s.unit == unit);
        let mut evidence = vec![format!(
            "{}: restart scheduled at {}; absent from current failed-unit list",
            unit, utc_timestamp(when)
        )];
        if let Some(state) = state {
            evidence.push(format!(
                "Current systemctl show: load={}, active={}, sub={}, result={}, NRestarts={}",
                state.load_state, state.active_state, state.sub_state, state.result,
                state.n_restarts.map(|n| n.to_string()).unwrap_or_else(|| "unknown".into())
            ));
        }
        findings.push(Finding {
            id: "SERVICE_RESTART_OBSERVED",
            severity: Severity::Info,
            confidence: if state.is_some() { Confidence::Observed } else { Confidence::Limited },
            title: if state.is_some_and(|s| s.active_state == "active") {
                "Service restart scheduled; unit currently active in systemd"
            } else {
                "Service restart scheduled; current unit readiness not verified"
            }.into(),
            explanation: "Systemd scheduled a restart. A separate read-only system-manager snapshot may confirm the unit is currently active, but cannot prove the application is ready, or that it ran continuously after the failure.".into(),
            evidence,
            checks: vec![format!("Review surrounding journal and service/application health for {}", unit)],
        });
    }
}

fn usb_root_port(path: &str) -> &str {
    path.split('.').next().unwrap_or(path)
}

fn diagnose_system(snapshot: &Snapshot, events: &[Event], findings: &mut Vec<Finding>) {
    crate::health::assess(snapshot, findings);
    if snapshot.root_filesystem_read_only == Some(true) {
        findings.push(Finding {
            id: "ROOT_FILESYSTEM_READ_ONLY",
            severity: Severity::Info,
            confidence: Confidence::Observed,
            title: "Root filesystem is mounted read-only in this mount namespace".into(),
            explanation: "The root mount has the ro option in /proc/self/mountinfo. This can be an intentional immutable/live/container configuration, or a response to errors. It is not itself proof of an unexpected filesystem failure.".into(),
            evidence: vec!["Mountpoint / has the ro flag in the active mount namespace".into()],
            checks: vec![
                "Inspect `findmnt -no SOURCE,FSTYPE,OPTIONS /` and the intended system image design".into(),
                "Review kernel filesystem errors before attempting any remount; do not automatically change mount flags".into(),
            ],
        });
    }
    if snapshot.pacman_checked == Some(true) && snapshot.pacman_consistent == Some(false) {
        findings.push(Finding {
            id: "PACMAN_DATABASE_CHECK_FAILED",
            severity: Severity::Warning,
            confidence: Confidence::Limited,
            title: "pacman local database consistency check returned an error".into(),
            explanation: "The read-only pacman -Dk check returned nonzero. This can indicate inconsistent package metadata or dependencies, but also a command/access error. It does not prove database corruption and does not verify all installed file contents.".into(),
            evidence: vec!["pacman -Dk returned nonzero; no raw pacman output retained".into()],
            checks: vec![
                "Run pacman -Dk interactively to inspect detailed messages".into(),
                "Inspect package database access and pacman.log before considering changes".into(),
                "For installed-file presence checks use pacman -Qk separately (can take time)".into(),
            ],
        });
    }
    if snapshot.dpkg_checked == Some(true) && snapshot.dpkg_problem_count > 0 {
        let count = snapshot.dpkg_problem_count;
        let mut evidence = vec![format!(
            "{count} {} not fully configured according to dpkg-query",
            count_word(count, "package is", "packages are")
        )];
        for pkg in snapshot.dpkg_problems.iter().take(15) {
            evidence.push(format!("{}: {}{}",
                pkg.name, pkg.state,
                if pkg.reinstall_required { " (reinstallation required)" } else { "" }));
        }
        if count > 15 {
            evidence.push(format!("{} additional affected packages not listed; use dpkg --audit", count - 15));
        }
        findings.push(Finding {
            id: "DPKG_INCOMPLETE_PACKAGES",
            severity: Severity::Warning,
            confidence: Confidence::Observed,
            title: format!("{count} {} in an incomplete dpkg state",
                count_word(count, "package", "packages")),
            explanation: "The local dpkg status database contains unpacked, half-configured, half-installed or trigger-pending packages (or a reinstallation-required error). This is a current package-management issue, independent of systemd failed units. It does not identify why a package configuration failed.".into(),
            evidence,
            checks: vec![
                "Inspect `dpkg --audit` for detailed, localized diagnostics".into(),
                "Preview (do not execute) a possible fix with `apt-get -s -f install`".into(),
                "Review relevant dpkg/apt logs and kernel package post-install scripts before changing packages".into(),
            ],
        });
    }
    for disk in &snapshot.disks {
        if (85..90).contains(&disk.used_percent) {
            findings.push(Finding {
                id: "DISK_CAPACITY_WATCH",
                severity: Severity::Info,
                confidence: Confidence::Observed,
                title: format!("Filesystem {} is {}% utilized", disk.mountpoint, disk.used_percent),
                explanation: "Capacity passed an advisory threshold but is below the 90% warning threshold. Growth and free bytes matter more than one percentage.".into(),
                evidence: vec![format!("Local df snapshot: {} at {}%", disk.mountpoint, disk.used_percent)],
                checks: vec!["Monitor filesystem growth and remaining free bytes before considering cleanup".into()],
            });
        }
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
    // Group only paths under the SAME USB root-port topology branch.
    // A burst spanning different root ports can represent unrelated actions.
    // A subtree detachment can also represent ONE hub loss, not many faults.
    let usb: Vec<&Event> = events.iter().filter(|e| e.kind == EventKind::UsbDisconnect).collect();
    let mut usb_burst_reported = false;
    for start in usb.iter().take(64) {
        let Some(start_path) = start.subject.as_deref() else { continue };
        let root_port = usb_root_port(start_path);
        let affected: std::collections::HashSet<String> = usb.iter()
            .filter(|e| e.timestamp >= start.timestamp && e.timestamp - start.timestamp <= 30)
            .filter_map(|e| e.subject.as_deref())
            .filter(|path| usb_root_port(path) == root_port)
            .map(str::to_owned).collect();
        if affected.len() >= 4 {
            let mut evidence = vec![format!(
                "{} distinct USB topology paths on root-port branch {} detached within 30 seconds beginning {}",
                affected.len(), root_port, utc_timestamp(start.timestamp)
            )];
            if events.iter().any(|e| e.kind == EventKind::UsbTransportError
                && e.timestamp.abs_diff(start.timestamp) <= 30) {
                evidence.push("A nearby Realtek r8152 USB transport error was observed".into());
            }
            usb_burst_reported = true;
            findings.push(Finding {
                id: "USB_DISCONNECT_BURST",
                severity: Severity::Warning,
                confidence: Confidence::Observed,
                title: "Multiple USB topology paths detached in one short interval".into(),
                explanation: "Several USB topology paths under one root-port branch detached close together. This may be a single hub/dock detachment or power event; it does not prove independent device faults, physical cause or causation of later network events.".into(),
                evidence,
                checks: vec![
                    "Check whether a dock/hub was intentionally unplugged, powered off or reset".into(),
                    "Inspect kernel USB, xHCI and r8152 messages and physical power/cables if unexpected".into(),
                ],
            });
            break;
        }
    }
    if !usb_burst_reported && count(events, EventKind::UsbTransportError) > 0 {
        findings.push(Finding {
            id: "USB_TRANSPORT_STATUS",
            severity: Severity::Info,
            confidence: Confidence::Observed,
            title: "USB network adapter reported a transport-status error".into(),
            explanation: "The r8152 USB adapter reported status -71 while submitting an interrupt transfer. This can occur during a disconnect or reset and is not proof of a defective adapter.".into(),
            evidence: vec![format!(
                "{} classified r8152 transport status observation(s)",
                count(events, EventKind::UsbTransportError)
            )],
            checks: vec!["Review surrounding USB hub/dock power and kernel logs if the adapter remains unavailable".into()],
        });
    }

    for (kind, id, title, explain, check) in [
        (EventKind::OutOfMemory, "OOM_EVENTS", "Out-of-memory events reported", "The journal contains messages consistent with memory exhaustion or process termination by an OOM killer.", "Review memory pressure and kernel logs around the event"),
        (EventKind::StorageError, "STORAGE_ERRORS", "Storage errors reported", "The journal contains storage-related error messages; their severity and affected device are not determined by this classifier.", "Inspect kernel storage logs, available disk space and hardware health"),
        (EventKind::FileHandleExhaustion, "FD_EXHAUSTION_LOGS", "File-descriptor exhaustion messages reported", "The selected journal contains explicit too-many-open-files or file-table-overflow messages. They may relate to one process's per-process limit rather than the kernel's global file-max.", "Inspect process limits, /proc/sys/fs/file-nr, service logs and ulimit -n"),
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
    fn wifi_usb_service_and_capacity_findings_are_contextual() {
        let ev = |t, kind, subject: Option<&str>| Event {
            timestamp: t, kind, subject: subject.map(str::to_owned),
            source: Some("kernel".into()),
        };
        let mut snapshot = Snapshot {
            system_units_checked: true,
            failed_user_units: Some(vec![]),
            disks: vec![
                crate::model::DiskUsage { mountpoint: "/home".into(), used_percent: 88 },
                crate::model::DiskUsage { mountpoint: "/opt".into(), used_percent: 86 },
            ],
            events: vec![
                ev(100, EventKind::UsbDisconnect, Some("1-1")),
                ev(100, EventKind::UsbDisconnect, Some("1-1.1")),
                ev(101, EventKind::UsbDisconnect, Some("1-1.2")),
                ev(101, EventKind::UsbDisconnect, Some("1-1.3")),
                ev(110, EventKind::LinkDown, Some("wlan0")),
                ev(128, EventKind::WifiAssociated, Some("wlan0")),
                ev(180, EventKind::WifiRoamStart, Some("wlan0")),
                ev(180, EventKind::WifiAssociated, Some("wlan0")),
                ev(50, EventKind::ServiceFailure, Some("lightdm.service")),
                ev(50, EventKind::ServiceRestartScheduled, Some("lightdm.service")),
            ],
            interface_drop_deltas: Some(vec![crate::model::NetworkDropDelta {
                name: "wlan0".into(), rx_dropped_delta: 12, tx_dropped_delta: 1,
                elapsed_ms: 1300,
            }]),
            ..Snapshot::default()
        };
        snapshot.events.sort_by_key(|e| e.timestamp);
        let report = analyze(snapshot, 3600, Focus::All, false);
        for id in [
            "USB_DISCONNECT_BURST", "DISK_CAPACITY_WATCH",
            "WIFI_LOCAL_ROAM_OBSERVED", "SERVICE_RESTART_OBSERVED",
            "RECENT_INTERFACE_DROPS",
        ] {
            assert!(report.findings.iter().any(|f| f.id == id), "missing {id}");
        }
        let link = report.findings.iter().find(|f| f.id == "LINK_DISCONNECTS").unwrap();
        assert!(link.evidence.iter().any(|x| x.contains("18 seconds")));
        let service = report.findings.iter().find(|f| f.id == "SERVICE_FAILURE_LOGS").unwrap();
        assert!(service.evidence.iter().any(|x| x.contains("restart scheduled")));
        assert_eq!(report.network_incidents.len(), 1,
            "positive reassociation alone must not create an incident window");
    }

    #[test]
    fn active_unit_snapshot_enriches_restart_without_erasing_failure() {
        let events = vec![
            Event {timestamp: 100, kind: EventKind::ServiceFailure,
                subject: Some("lightdm.service".into()), source: Some("systemd".into())},
            Event {timestamp: 100, kind: EventKind::ServiceRestartScheduled,
                subject: Some("lightdm.service".into()), source: Some("systemd".into())},
        ];
        let report = analyze(Snapshot {
            system_units_checked: true,
            failed_units: vec![],
            events,
            service_statuses: vec![crate::model::ServiceStatus {
                unit: "lightdm.service".into(),
                manager: "system".into(),
                transient: None,
                desktop_entry: false,
                exec_main_status: None,
                load_state: "loaded".into(),
                active_state: "active".into(),
                sub_state: "running".into(),
                result: "success".into(),
                n_restarts: Some(1),
            }],
            ..Snapshot::default()
        }, 3600, Focus::Services, false);
        assert!(report.findings.iter().any(|f| f.id == "SERVICE_FAILURE_LOGS"));
        let restart = report.findings.iter().find(|f|
            f.id == "SERVICE_RESTART_OBSERVED").unwrap();
        assert_eq!(restart.confidence, Confidence::Observed);
        assert!(restart.evidence.iter().any(|text|
            text.contains("active=active") && text.contains("NRestarts=1")));
        assert!(restart.explanation.contains("cannot prove the application is ready"));
    }

    #[test]
    fn wifi_association_must_match_the_same_interface() {
        let events = vec![
            Event {timestamp: 100, kind: EventKind::LinkDown,
                subject: Some("wlan0".into()), source: None},
            Event {timestamp: 112, kind: EventKind::WifiAssociated,
                subject: Some("wlan1".into()), source: None},
        ];
        assert!(nearby_matching_subjects(&events,
            EventKind::LinkDown, EventKind::WifiAssociated, 30).is_none());
    }

    #[test]
    fn same_second_wifi_events_do_not_claim_order_or_recovery() {
        // Real journal export retains seconds, not subsecond timing. Even if the
        // source association precedes deauthentication, their timestamps tie.
        let events = [
            EventKind::WifiAssociated,
            EventKind::WifiRoamStart,
            EventKind::LinkDown,
        ].into_iter().map(|kind| Event {
            timestamp: 100,
            kind,
            subject: Some("wlan0".into()),
            source: Some("kernel".into()),
        }).collect();
        let report = analyze(Snapshot { events, ..Snapshot::default() },
            3600, Focus::Network, false);
        let roam = report.findings.iter().find(|f|
            f.id == "WIFI_LOCAL_ROAM_OBSERVED").unwrap();
        assert!(roam.evidence.iter().any(|line| line.contains("order not verified")));
        assert!(!roam.evidence.iter().any(|line| line.contains("later")));
        let link = report.findings.iter().find(|f| f.id == "LINK_DISCONNECTS").unwrap();
        assert!(link.evidence.iter().any(|line| line.contains("ordering unknown")));
        assert!(!link.evidence.iter().any(|line| line.contains("Subsequent")));
    }

    #[test]
    fn usb_burst_requires_four_unique_ports_in_one_window() {
        let e = |timestamp, port: &str| Event {
            timestamp,
            kind: EventKind::UsbDisconnect,
            subject: Some(port.into()),
            source: Some("kernel".into()),
        };
        let report = analyze(Snapshot {
            events: vec![e(100,"1-1"),e(101,"1-1.1"),e(102,"1-1.2")],
            ..Snapshot::default()
        }, 3600, Focus::System, false);
        assert!(!report.findings.iter().any(|f| f.id == "USB_DISCONNECT_BURST"));
        let distant = analyze(Snapshot {
            events: vec![e(100,"1-1"),e(101,"1-1.1"),e(102,"1-1.2"),e(200,"1-1.3")],
            ..Snapshot::default()
        }, 3600, Focus::System, false);
        assert!(!distant.findings.iter().any(|f| f.id == "USB_DISCONNECT_BURST"));
    }

    #[test]
    fn usb_burst_does_not_mix_distinct_root_port_branches() {
        let e = |port: &str| Event {
            timestamp: 100,
            kind: EventKind::UsbDisconnect,
            subject: Some(port.to_owned()),
            source: Some("kernel".into()),
        };
        let mixed = analyze(Snapshot {
            events: vec![e("1-5"), e("1-5.1"), e("1-1"), e("1-1.1")],
            ..Snapshot::default()
        }, 3600, Focus::System, false);
        assert!(!mixed.findings.iter().any(|f| f.id == "USB_DISCONNECT_BURST"),
            "events on separate USB root ports must not count as one burst");

        let cascade = analyze(Snapshot {
            events: vec![
                e("1-5"), e("1-5.1"), e("1-1"), e("1-1.1"),
                e("1-1.1.1"), e("1-1.1.2"),
            ],
            ..Snapshot::default()
        }, 3600, Focus::System, false);
        let finding = cascade.findings.iter().find(|f|
            f.id == "USB_DISCONNECT_BURST").unwrap();
        assert!(finding.evidence.iter().any(|line|
            line.contains("4 distinct USB topology paths on root-port branch 1-1")));
        assert!(!finding.evidence.iter().any(|line| line.contains("6 distinct")));
    }

    #[test]
    fn isolated_r8152_transport_status_is_info_not_hardware_fault() {
        let report = analyze(Snapshot {
            events: vec![Event {timestamp: 100,kind: EventKind::UsbTransportError,
                subject: Some("1-1.2".into()),source: Some("kernel".into())}],
            ..Snapshot::default()
        },3600,Focus::System,false);
        let finding=report.findings.iter().find(|f|f.id=="USB_TRANSPORT_STATUS").unwrap();
        assert_eq!(finding.severity,Severity::Info);
    }

    #[test]
    fn disk_threshold_information_only_until_90_percent() {
        for percent in [84, 85, 89, 90] {
            let snapshot = Snapshot {
                disks: vec![crate::model::DiskUsage {
                    mountpoint: "/test".into(), used_percent: percent
                }], ..Snapshot::default()
            };
            let report = analyze(snapshot, 3600, Focus::System, false);
            let info = report.findings.iter().any(|f| f.id == "DISK_CAPACITY_WATCH");
            let warning = report.findings.iter().any(|f| f.id == "DISK_NEAR_FULL");
            assert_eq!(info, (85..90).contains(&percent));
            assert_eq!(warning, percent >= 90);
        }
    }

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
    fn service_log_groups_are_not_independent_failed_units() {
        let snapshot = Snapshot {
            system_units_checked: true,
            failed_user_units: Some(vec![]),
            events: vec![
                Event { timestamp: 100, kind: EventKind::ServiceFailure, subject: None, source: Some("systemd".into()) },
                Event { timestamp: 100, kind: EventKind::ServiceFailure, subject: Some("rclone-onedrive.service".into()), source: Some("systemd".into()) },
                Event { timestamp: 500, kind: EventKind::ServiceFailure, subject: None, source: Some("systemd".into()) },
                Event { timestamp: 500, kind: EventKind::ServiceFailure, subject: Some("proton-backup.service".into()), source: Some("systemd".into()) },
            ],
            ..Snapshot::default()
        };
        let report = analyze(snapshot, 3_600, Focus::Services, false);
        assert_eq!(report.service_event_windows.len(), 2);
        assert_eq!(report.service_event_windows[0].entry_count, 2);
        assert_eq!(report.service_event_windows[0].unattributed_entries, 1);
        assert_eq!(report.service_event_windows[1].units, vec!["proton-backup.service"]);
        let finding = report.findings.iter().find(|f| f.id == "SERVICE_FAILURE_LOGS").unwrap();
        assert!(finding.explanation.contains("NOT a count"));
    }

    #[test]
    fn singular_historical_message_evidence_is_grammatical() {
        let snapshot = Snapshot {
            uptime_seconds: Some(50),
            events: vec![
                Event { timestamp: 1, kind: EventKind::ServiceFailure,
                    subject: Some("sample.service".into()), source: Some("systemd".into()) },
            ],
            ..Snapshot::default()
        };
        let report = analyze(snapshot, 3_600, Focus::Services, false);
        let finding = report.findings.iter().find(|f| f.id == "SERVICE_FAILURE_LOGS").unwrap();
        assert!(finding.evidence.iter().any(|s| s.contains("1 classified entry across 1 distinct second in 1 time window")));
        assert!(finding.evidence.iter().any(|s| s.contains("1 classified entry predates the estimated current boot")));
    }

    #[test]
    fn current_user_failed_units_have_current_snapshot_scope() {
        let snapshot = Snapshot {
            failed_user_units: Some(vec!["broken-user.service".into()]),
            system_units_checked: true,
            ..Snapshot::default()
        };
        let report = analyze(snapshot, 60, Focus::Services, false);
        assert_eq!(report.findings.iter()
            .find(|f| f.id == "FAILED_USER_SYSTEMD_UNITS").unwrap().scope(), "current_snapshot");
    }

    #[test]
    fn pacman_failure_is_a_limited_current_warning() {
        let report = analyze(Snapshot {
            pacman_checked: Some(true), pacman_consistent: Some(false),
            ..Snapshot::default()
        }, 3600, Focus::System, false);
        let finding = report.findings.iter()
            .find(|f| f.id == "PACMAN_DATABASE_CHECK_FAILED").unwrap();
        assert_eq!(finding.scope(), "current_snapshot");
        assert_eq!(finding.severity, Severity::Warning);
        assert_eq!(finding.confidence, Confidence::Limited);
    }

    #[test]
    fn pacman_success_or_unavailable_is_not_a_failure() {
        for (checked, consistent) in [
            (None, None), (Some(false), None), (Some(true), Some(true))
        ] {
            let report = analyze(Snapshot {
                pacman_checked: checked, pacman_consistent: consistent,
                ..Snapshot::default()
            }, 3600, Focus::System, false);
            assert!(!report.findings.iter()
                .any(|f| f.id == "PACMAN_DATABASE_CHECK_FAILED"));
        }
    }

    #[test]
    fn dpkg_incomplete_kernel_packages_are_current_warning() {
        use crate::model::DpkgProblem;
        let snapshot = Snapshot {
            dpkg_checked: Some(true),
            dpkg_problem_count: 4,
            dpkg_problems: vec![
                DpkgProblem { name: "linux-image-6.12.116+deb13-amd64".into(),
                    state: "half-configured".into(), reinstall_required: false },
                DpkgProblem { name: "linux-headers-amd64".into(),
                    state: "unpacked".into(), reinstall_required: false },
            ],
            ..Snapshot::default()
        };
        let report = analyze(snapshot, 3600, Focus::System, false);
        let finding = report.findings.iter().find(|f| f.id == "DPKG_INCOMPLETE_PACKAGES").unwrap();
        assert_eq!(finding.scope(), "current_snapshot");
        assert!(finding.title.contains("4 packages"));
        assert!(finding.evidence.iter().any(|s| s.contains("half-configured")));
        assert!(!report.findings.iter().any(|f| f.id == "NO_CLASSIFIED_ANOMALY"));
    }

    #[test]
    fn healthy_or_unavailable_dpkg_does_not_generate_warning() {
        for status in [None, Some(false), Some(true)] {
            let report = analyze(Snapshot { dpkg_checked: status, ..Snapshot::default() },
                3600, Focus::System, false);
            assert!(!report.findings.iter().any(|f| f.id == "DPKG_INCOMPLETE_PACKAGES"));
        }
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
    fn truncated_journal_without_matching_rules_is_incomplete_evidence() {
        let report = analyze(Snapshot {
            journal_checked: true,
            journal_truncated: true,
            journal_entries_examined: 500,
            disk_usage_checked: true,
            ..Snapshot::default()
        }, 86400, Focus::System, false);
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].id, "INSUFFICIENT_EVIDENCE");
        assert!(report.findings[0].evidence.iter().any(|item| item.contains("500")));
    }

    #[test]
    fn truncated_journal_with_a_real_warning_preserves_warning_and_notes_partial_scope() {
        let report = analyze(Snapshot {
            journal_checked: true,
            journal_truncated: true,
            journal_entries_examined: 500,
            events: vec![Event {
                timestamp: 1_700_000_000,
                kind: EventKind::FileHandleExhaustion,
                source: Some("app".into()), subject: None,
            }],
            ..Snapshot::default()
        }, 86400, Focus::System, false);
        assert!(report.findings.iter().any(|f| f.id == "FD_EXHAUSTION_LOGS"));
        let limited = report.findings.iter()
            .find(|f| f.id == "JOURNAL_WINDOW_TRUNCATED").unwrap();
        assert_eq!(limited.scope(), "unverified");
        assert_eq!(limited.severity, Severity::Info);
    }

    #[test]
    fn no_anomaly_requires_core_evidence() {
        let healthy = Snapshot {
            journal_checked: true,
            disk_usage_checked: true,
            system_units_checked: true,
            failed_user_units: Some(vec![]),
            ipv4_default_route: Some(true),
            dns_nameservers: Some(1),
            ..Snapshot::default()
        };
        let report = analyze(healthy, 60, Focus::All, true);
        assert_eq!(report.findings[0].id, "NO_CLASSIFIED_ANOMALY");

        let incomplete = analyze(Snapshot::default(), 60, Focus::All, true);
        assert_eq!(incomplete.findings[0].id, "INSUFFICIENT_EVIDENCE");
        assert_eq!(incomplete.findings[0].scope(), "unverified");
        assert!(incomplete.findings[0].evidence.iter()
            .any(|x| x.contains("journalctl not verified")));
    }

    #[test]
    fn root_read_only_is_not_automatically_a_failure() {
        let report = analyze(Snapshot {
            root_filesystem_read_only: Some(true), ..Snapshot::default()
        }, 3600, Focus::System, false);
        let root = report.findings.iter()
            .find(|f| f.id == "ROOT_FILESYSTEM_READ_ONLY").unwrap();
        assert_eq!(root.severity, Severity::Info);
        assert_eq!(root.scope(), "current_snapshot");
    }

    #[test]
    fn historical_fd_exhaustion_classified_independently_of_current_file_nr() {
        let report = analyze(Snapshot {
            events: vec![Event {
                timestamp: 1_700_000_000,
                kind: EventKind::FileHandleExhaustion,
                source: Some("app".into()), subject: None,
            }],
            ..Snapshot::default()
        }, 3600, Focus::System, false);
        assert!(report.findings.iter().any(|f| f.id == "FD_EXHAUSTION_LOGS"));
    }
}
