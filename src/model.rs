use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    All,
    Network,
    Services,
    System,
}

impl Focus {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "all" => Some(Self::All),
            "network" => Some(Self::Network),
            "services" => Some(Self::Services),
            "system" => Some(Self::System),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Network => "network",
            Self::Services => "services",
            Self::System => "system",
        }
    }

    pub fn includes(self, other: Self) -> bool {
        self == Self::All || self == other
    }
}

impl Finding {
    /// A live system snapshot and a historical journal observation are not interchangeable.
    pub fn scope(&self) -> &'static str {
        match self.id {
            "ROUTE_MISSING" | "IPV4_ROUTE_MISSING" | "DNS_UNCONFIGURED"
            | "DNS_STUB_INACTIVE" | "FAILED_SYSTEMD_UNITS" | "FAILED_USER_SYSTEMD_UNITS"
            | "IFACE_ERROR_COUNTERS" | "DISK_NEAR_FULL" | "DPKG_INCOMPLETE_PACKAGES"
            | "PACMAN_DATABASE_CHECK_FAILED" | "LOW_MEMORY_HEADROOM"
            | "MEMORY_STALL_PRESSURE" | "IO_STALL_PRESSURE"
            | "FILE_HANDLE_LIMIT_NEAR" | "INODE_NEAR_FULL"
            | "MD_RAID_DEGRADED" | "ROOT_FILESYSTEM_READ_ONLY"
            | "CPU_CONTENTION_PRESSURE" | "RECENT_INTERFACE_DROPS"
            | "DISK_CAPACITY_WATCH" => "current_snapshot",
            "NO_CLASSIFIED_ANOMALY" | "INSUFFICIENT_EVIDENCE"
            | "JOURNAL_WINDOW_TRUNCATED" => "unverified",
            _ => "historical_journal",
        }
    }
}

impl fmt::Display for Focus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Warning,
    Info,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Warning => "warning",
            Self::Info => "info",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confidence {
    Observed,
    Moderate,
    Limited,
}

impl Confidence {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Observed => "observed",
            Self::Moderate => "moderate",
            Self::Limited => "limited",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Finding {
    pub id: &'static str,
    pub severity: Severity,
    pub confidence: Confidence,
    pub title: String,
    pub explanation: String,
    pub evidence: Vec<String>,
    pub checks: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    LinkDown,
    DhcpProblem,
    DnsProblem,
    NetworkUnreachable,
    ServiceFailure,
    OutOfMemory,
    StorageError,
    FileHandleExhaustion,
    WifiAssociated,
    WifiRoamStart,
    UsbDisconnect,
    UsbTransportError,
    ServiceRestartScheduled,
}

impl EventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LinkDown => "link_disconnected",
            Self::DhcpProblem => "dhcp_problem",
            Self::DnsProblem => "dns_failure",
            Self::NetworkUnreachable => "network_unreachable",
            Self::ServiceFailure => "service_failure",
            Self::OutOfMemory => "out_of_memory",
            Self::StorageError => "storage_error",
            Self::FileHandleExhaustion => "file_handle_exhaustion",
            Self::WifiAssociated => "wifi_associated",
            Self::WifiRoamStart => "wifi_roam_start",
            Self::UsbDisconnect => "usb_disconnect",
            Self::UsbTransportError => "usb_transport_error",
            Self::ServiceRestartScheduled => "service_restart_scheduled",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::LinkDown => "Network link disconnected",
            Self::DhcpProblem => "DHCP problem reported",
            Self::DnsProblem => "DNS resolution failure reported",
            Self::NetworkUnreachable => "Network unreachable reported",
            Self::ServiceFailure => "Service failure reported",
            Self::OutOfMemory => "Out-of-memory event reported",
            Self::StorageError => "Storage error reported",
            Self::FileHandleExhaustion => "File descriptor exhaustion reported",
            Self::WifiAssociated => "Wi-Fi association observed",
            Self::WifiRoamStart => "Local-choice Wi-Fi handoff observed",
            Self::UsbDisconnect => "USB peripheral disconnected",
            Self::UsbTransportError => "USB transport error reported",
            Self::ServiceRestartScheduled => "Service restart scheduled",
        }
    }

    pub fn focus(self) -> Focus {
        match self {
            Self::LinkDown | Self::DhcpProblem | Self::DnsProblem
            | Self::NetworkUnreachable | Self::WifiAssociated
            | Self::WifiRoamStart => {
                Focus::Network
            }
            Self::ServiceFailure | Self::ServiceRestartScheduled => Focus::Services,
            Self::OutOfMemory | Self::StorageError | Self::FileHandleExhaustion
            | Self::UsbDisconnect | Self::UsbTransportError => Focus::System,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Event {
    pub timestamp: u64,
    pub kind: EventKind,
    /// Bounded identifier extracted from the journal, not the message body.
    pub subject: Option<String>,
    /// Name of the emitting program (e.g. NetworkManager, kernel).
    pub source: Option<String>,
}

#[derive(Debug, Clone)]
pub struct InterfaceStats {
    pub name: String,
    /// Current kernel operstate, a local snapshot (not a connectivity probe).
    pub operstate: Option<String>,
    pub rx_errors: u64,
    pub rx_dropped: u64,
    pub tx_errors: u64,
    pub tx_dropped: u64,
}

/// Two monotonic samples from /proc/net/dev during the same diagnosis.
#[derive(Debug, Clone)]
pub struct NetworkDropDelta {
    pub name: String,
    pub rx_dropped_delta: u64,
    pub tx_dropped_delta: u64,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone)]
pub struct DiskUsage {
    pub mountpoint: String,
    pub used_percent: u8,
}

#[derive(Debug, Clone)]
pub struct DirectoryUsage {
    pub path: String,
    pub kib: u64,
}

#[derive(Debug, Clone)]
pub struct DiskBreakdown {
    pub mountpoint: String,
    pub entries: Vec<DirectoryUsage>,
}

/// A bounded cluster of observed network-related journal events, not a proven outage.
#[derive(Debug, Clone)]
pub struct NetworkIncident {
    /// Network interface, when the journal explicitly supplied one.
    /// Unknown subjects form separate windows from identified interfaces.
    pub subject: Option<String>,
    pub start: u64,
    pub end: u64,
    pub link_events: usize,
    pub dhcp_events: usize,
    pub dns_events: usize,
    pub unreachable_events: usize,
}

impl NetworkIncident {
    pub fn event_count(&self) -> usize {
        self.link_events + self.dhcp_events + self.dns_events + self.unreachable_events
    }

    pub fn summary(&self) -> String {
        let mut names = Vec::new();
        if self.link_events > 0 { names.push("link"); }
        if self.dhcp_events > 0 { names.push("DHCP"); }
        if self.dns_events > 0 { names.push("DNS"); }
        if self.unreachable_events > 0 { names.push("unreachable"); }
        names.join(" / ")
    }
}

/// Temporal cluster of historical service log observations, not a count
/// of independently failed services or a statement of current service health.
#[derive(Debug, Clone)]
pub struct ServiceEventWindow {
    pub start: u64,
    pub end: u64,
    pub entry_count: usize,
    pub unattributed_entries: usize,
    pub units: Vec<String>,
}

/// Read-only state for a journal-attributed service in the verified manager.
/// Active does not imply application readiness or prove past recovery.

#[derive(Debug, Clone)]
pub struct ServiceStatus {
    pub unit: String,
    /// "system" or "user"; based on the manager queried, never inferred from a name.
    pub manager: String,
    /// Whether this is a systemd transient unit.
    pub transient: Option<bool>,
    /// True only if the installed FragmentPath is a desktop launcher.
    /// No paths or command arguments are retained.
    pub desktop_entry: bool,
    /// Numeric exit status from systemctl show, not a diagnosis of its cause.
    pub exec_main_status: Option<u64>,
    pub load_state: String,
    pub active_state: String,
    pub sub_state: String,
    pub result: String,
    pub n_restarts: Option<u64>,
}

/// One package that is not fully configured in the local dpkg status database.
#[derive(Debug, Clone)]
pub struct DpkgProblem {
    pub name: String,
    pub state: String,
    pub reinstall_required: bool,
}

/// Linux-wide resource snapshot from /proc/meminfo (host, not cgroup limit).
#[derive(Debug, Clone)]
pub struct MemoryHealth {
    pub total_kib: u64,
    pub available_kib: u64,
    pub swap_total_kib: Option<u64>,
    pub swap_free_kib: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct PressureHealth {
    pub resource: String,
    /// 60-second PSI stall average, in hundredths of a percent.
    pub some_avg60_x100: Option<u32>,
    pub full_avg60_x100: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct FileHandleHealth {
    pub allocated: u64,
    pub unused: u64,
    pub maximum: u64,
}

#[derive(Debug, Clone)]
pub struct InodeHealth {
    pub mountpoint: String,
    pub used_percent: u8,
}

#[derive(Debug, Clone)]
pub struct RaidHealth {
    pub name: String,
    pub expected: u32,
    pub active: u32,
}

/// Bounded per-command timing; does not contain command outputs or user paths.
#[derive(Debug, Clone)]
pub struct CollectorTiming {
    pub name: &'static str,
    pub elapsed_ms: u64,
    pub succeeded: bool,
}

/// Optional, strictly whitelisted fault/status codes extracted from classified
/// journal lines. No original messages, hostnames, addresses or arguments.
#[derive(Debug, Clone)]
pub struct EventContext {
    pub timestamp: u64,
    pub kind: EventKind,
    pub subject: Option<String>,
    pub signal: String,
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub os: String,
    pub kernel: String,
    pub uptime_seconds: Option<u64>,
    pub ipv4_default_route: Option<bool>,
    pub ipv6_default_route: Option<bool>,
    pub dns_nameservers: Option<usize>,
    pub dns_stub: bool,
    pub resolved_active: Option<bool>,
    pub interfaces: Vec<InterfaceStats>,
    /// None when a second usable /proc/net/dev sample was unavailable.
    pub interface_drop_deltas: Option<Vec<NetworkDropDelta>>,
    pub failed_units: Vec<String>,
    pub system_units_checked: bool,
    /// At most four bounded system/user-manager snapshots for historical failures.
    /// Empty means none collected, not that every historical failure recovered.
    pub service_statuses: Vec<ServiceStatus>,
    /// None when the user's systemd manager is inaccessible.
    pub failed_user_units: Option<Vec<String>>,
    /// None: dpkg status database absent (not a dpkg-managed system).
    /// Some(false): database present, but query failed; Some(true): queried.
    pub dpkg_checked: Option<bool>,
    /// Total number before the display cap.
    pub dpkg_problem_count: usize,
    pub dpkg_problems: Vec<DpkgProblem>,
    /// None: pacman local database absent; false: command unavailable; true: completed.
    pub pacman_checked: Option<bool>,
    /// None: not checked; true: pacman -Dk succeeded; false: nonzero exit.
    pub pacman_consistent: Option<bool>,
    /// Host-level Linux resource probes. None means unavailable, not healthy.
    pub memory_health: Option<MemoryHealth>,
    pub pressure_health: Vec<PressureHealth>,
    pub file_handle_health: Option<FileHandleHealth>,
    pub inode_health: Vec<InodeHealth>,
    pub raid_health: Vec<RaidHealth>,
    /// Root (/) mount options from /proc/self/mountinfo; None if unverified.
    pub root_filesystem_read_only: Option<bool>,
    /// True only when the disk capacity collector returned valid entries.
    pub disk_usage_checked: bool,
    pub disks: Vec<DiskUsage>,
    pub disk_breakdowns: Vec<DiskBreakdown>,
    pub events: Vec<Event>,
    /// Safe, bounded details for those classified events only.
    pub event_contexts: Vec<EventContext>,
    /// True only when journalctl completed successfully; empty logs may still be valid.
    pub journal_checked: bool,
    /// True if at least one older entry was omitted after collecting limit + 1.
    pub journal_truncated: bool,
    /// Number of journal entries actually examined (not classified-event count).
    pub journal_entries_examined: usize,
    /// True when journalctl was invoked with --boot=0.
    pub journal_current_boot_only: bool,
    pub notes: Vec<String>,
    /// Measured local collector durations (not estimated).
    pub collector_timings: Vec<CollectorTiming>,
}

impl Default for Snapshot {
    fn default() -> Self {
        Self {
            os: "Unknown Linux".to_string(),
            kernel: "Unknown".to_string(),
            uptime_seconds: None,
            ipv4_default_route: None,
            ipv6_default_route: None,
            dns_nameservers: None,
            dns_stub: false,
            resolved_active: None,
            interfaces: Vec::new(),
            interface_drop_deltas: None,
            failed_units: Vec::new(),
            system_units_checked: false,
            service_statuses: Vec::new(),
            failed_user_units: None,
            dpkg_checked: None,
            dpkg_problem_count: 0,
            dpkg_problems: Vec::new(),
            pacman_checked: None,
            pacman_consistent: None,
            memory_health: None,
            pressure_health: Vec::new(),
            file_handle_health: None,
            inode_health: Vec::new(),
            raid_health: Vec::new(),
            root_filesystem_read_only: None,
            disk_usage_checked: false,
            disks: Vec::new(),
            disk_breakdowns: Vec::new(),
            events: Vec::new(),
            event_contexts: Vec::new(),
            journal_checked: false,
            journal_truncated: false,
            journal_entries_examined: 0,
            journal_current_boot_only: false,
            notes: Vec::new(),
            collector_timings: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Report {
    pub version: &'static str,
    pub generated_at: u64,
    pub lookback_seconds: u64,
    pub focus: Focus,
    pub demo: bool,
    pub snapshot: Snapshot,
    pub findings: Vec<Finding>,
    pub timeline: Vec<Event>,
    pub network_incidents: Vec<NetworkIncident>,
    pub service_event_windows: Vec<ServiceEventWindow>,
}

/// Return grammatical singular/plural labels without altering numeric data.
pub fn count_word(count: usize, singular: &'static str, plural: &'static str) -> &'static str {
    if count == 1 { singular } else { plural }
}

#[cfg(test)]
mod count_word_tests {
    use super::count_word;
    #[test]
    fn singular_and_plural_counts() {
        assert_eq!(count_word(0, "entry", "entries"), "entries");
        assert_eq!(count_word(1, "entry", "entries"), "entry");
        assert_eq!(count_word(2, "entry", "entries"), "entries");
    }
}
