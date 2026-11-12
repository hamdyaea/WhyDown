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
            | "IFACE_ERROR_COUNTERS" | "DISK_NEAR_FULL" => "current_snapshot",
            "NO_CLASSIFIED_ANOMALY" => "unverified",
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
        }
    }

    pub fn focus(self) -> Focus {
        match self {
            Self::LinkDown | Self::DhcpProblem | Self::DnsProblem | Self::NetworkUnreachable => {
                Focus::Network
            }
            Self::ServiceFailure => Focus::Services,
            Self::OutOfMemory | Self::StorageError => Focus::System,
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
    pub failed_units: Vec<String>,
    pub system_units_checked: bool,
    /// None when the user's systemd manager is inaccessible.
    pub failed_user_units: Option<Vec<String>>,
    pub disks: Vec<DiskUsage>,
    pub disk_breakdowns: Vec<DiskBreakdown>,
    pub events: Vec<Event>,
    pub notes: Vec<String>,
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
            failed_units: Vec::new(),
            system_units_checked: false,
            failed_user_units: None,
            disks: Vec::new(),
            disk_breakdowns: Vec::new(),
            events: Vec::new(),
            notes: Vec::new(),
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
