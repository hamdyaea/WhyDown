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
}

#[derive(Debug, Clone)]
pub struct InterfaceStats {
    pub name: String,
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
    pub disks: Vec<DiskUsage>,
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
            disks: Vec::new(),
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
}
