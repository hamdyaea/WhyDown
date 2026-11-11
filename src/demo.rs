//! Deterministic example inputs for screenshots and smoke tests.

use crate::model::{DiskUsage, Event, EventKind, InterfaceStats, Snapshot};
use crate::time::now_unix;

pub fn snapshot() -> Snapshot {
    let now = now_unix();
    Snapshot {
        os: "Demo Linux 1.0 (fictional)".into(),
        kernel: "6.12.0-demo".into(),
        uptime_seconds: Some(72_000),
        ipv4_default_route: Some(true),
        ipv6_default_route: Some(true),
        dns_nameservers: Some(1),
        dns_stub: true,
        resolved_active: Some(true),
        interfaces: vec![InterfaceStats {
            name: "wlan0".into(), operstate: Some("up".into()), rx_errors: 0, rx_dropped: 1, tx_errors: 0, tx_dropped: 0,
        }],
        failed_units: vec!["example-backup.service".into()],
        disks: vec![DiskUsage { mountpoint: "/".into(), used_percent: 64 }],
        disk_breakdowns: vec![],
        events: vec![
            Event { timestamp: now.saturating_sub(240), kind: EventKind::LinkDown, subject: Some("wlan0".into()), source: Some("NetworkManager".into()) },
            Event { timestamp: now.saturating_sub(235), kind: EventKind::DhcpProblem, subject: Some("wlan0".into()), source: Some("NetworkManager".into()) },
            Event { timestamp: now.saturating_sub(225), kind: EventKind::DnsProblem, subject: Some("wlan0".into()), source: Some("NetworkManager".into()) },
            Event { timestamp: now.saturating_sub(180), kind: EventKind::ServiceFailure, subject: Some("wlan0".into()), source: Some("NetworkManager".into()) },
        ],
        notes: vec!["DEMO DATA: no local system information was read".into()],
    }
}
