//! Deterministic example inputs for screenshots and smoke tests.

use crate::model::{CollectorTiming, DiskUsage, DpkgProblem, Event, EventKind, InterfaceStats,
    MemoryHealth, PressureHealth, FileHandleHealth, InodeHealth, Snapshot};
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
        interface_drop_deltas: None,
        failed_units: vec!["example-backup.service".into()],
        system_units_checked: true,
        service_statuses: vec![],
        failed_user_units: Some(vec![]),
        dpkg_checked: Some(true),
        dpkg_problem_count: 1,
        pacman_checked: None,
        pacman_consistent: None,
        dpkg_problems: vec![DpkgProblem {
            name: "example-kernel-headers".into(), state: "unpacked".into(),
            reinstall_required: false,
        }],
        memory_health: Some(MemoryHealth {
            total_kib: 4_194_304, available_kib: 2_097_152,
            swap_total_kib: Some(1_048_576), swap_free_kib: Some(786_432),
        }),
        pressure_health: vec![PressureHealth {
            resource: "memory".into(), some_avg60_x100: Some(15),
            full_avg60_x100: Some(0),
        }],
        file_handle_health: Some(FileHandleHealth {
            allocated: 1280, unused: 0, maximum: 524_288,
        }),
        inode_health: vec![InodeHealth {
            mountpoint: "/".into(), used_percent: 18,
        }],
        raid_health: vec![],
        root_filesystem_read_only: Some(false),
        disk_usage_checked: true,
        disks: vec![DiskUsage { mountpoint: "/".into(), used_percent: 64 }],
        disk_breakdowns: vec![],
        events: vec![
            Event { timestamp: now.saturating_sub(240), kind: EventKind::LinkDown, subject: Some("wlan0".into()), source: Some("NetworkManager".into()) },
            Event { timestamp: now.saturating_sub(235), kind: EventKind::DhcpProblem, subject: Some("wlan0".into()), source: Some("NetworkManager".into()) },
            Event { timestamp: now.saturating_sub(225), kind: EventKind::DnsProblem, subject: None, source: Some("systemd-resolved".into()) },
            Event { timestamp: now.saturating_sub(180), kind: EventKind::ServiceFailure, subject: Some("example-backup.service".into()), source: Some("systemd".into()) },
        ],
        event_contexts: vec![],
        journal_checked: true,
        journal_truncated: false,
        journal_entries_examined: 4,
        journal_current_boot_only: false,
        collector_timings: vec![CollectorTiming {
            name: "demo fixture", elapsed_ms: 0, succeeded: true,
        }],
        notes: vec!["DEMO DATA: no local system information was read".into()],
    }
}
