//! Portable, read-only host health sensors from procfs.

use crate::model::{Confidence, FileHandleHealth, Finding, InodeHealth, MemoryHealth, PressureHealth, RaidHealth, Severity, Snapshot};
use std::{collections::HashSet, fs::File, io::{self, Read}};

fn read_small(path: &str) -> io::Result<String> {
    let mut bytes = Vec::new();
    File::open(path)?.take(131_072).read_to_end(&mut bytes)?;
    String::from_utf8(bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

pub fn collect_kernel(s: &mut Snapshot) {
    if let Ok(t) = read_small("/proc/meminfo") { s.memory_health = parse_meminfo(&t); }
    for (resource, path) in [("cpu", "/proc/pressure/cpu"), ("memory", "/proc/pressure/memory"), ("io", "/proc/pressure/io")] {
        if let Ok(t) = read_small(path) {
            if let Some(p) = parse_pressure(resource, &t) { s.pressure_health.push(p); }
        }
    }
    if let Ok(t) = read_small("/proc/sys/fs/file-nr") { s.file_handle_health = parse_file_nr(&t); }
    if let Ok(t) = read_small("/proc/mdstat") { s.raid_health = parse_mdstat(&t); }
}

pub fn parse_meminfo(t: &str) -> Option<MemoryHealth> {
    let (mut total, mut available, mut swap_total, mut swap_free) = (None, None, None, None);
    for line in t.lines() {
        let Some((name, rest)) = line.split_once(':') else { continue };
        let Some(v) = rest.split_whitespace().next().and_then(|s| s.parse::<u64>().ok()) else { continue };
        match name {
            "MemTotal" => total = Some(v), "MemAvailable" => available = Some(v),
            "SwapTotal" => swap_total = Some(v), "SwapFree" => swap_free = Some(v),
            _ => {}
        }
    }
    let total = total.filter(|n| *n > 0)?;
    let available = available.filter(|n| *n <= total)?;
    Some(MemoryHealth { total_kib: total, available_kib: available, swap_total_kib: swap_total, swap_free_kib: swap_free })
}

pub fn parse_pressure(resource: &str, text: &str) -> Option<PressureHealth> {
    if !matches!(resource, "cpu" | "memory" | "io") { return None; }
    let (mut some, mut full) = (None, None);
    for line in text.lines() {
        let mut fields = line.split_whitespace();
        let Some(kind) = fields.next() else { continue };
        if kind != "some" && kind != "full" { continue; }
        for field in fields {
            if let Some(v) = field.strip_prefix("avg60=") {
                if let Ok(p) = v.parse::<f64>() {
                    if p.is_finite() && (0.0..=100.0).contains(&p) {
                        let hundredths = (p * 100.0).round() as u32;
                        if kind == "some" { some = Some(hundredths); } else { full = Some(hundredths); }
                    }
                }
            }
        }
    }
    if some.is_none() && full.is_none() { return None; }
    Some(PressureHealth { resource: resource.to_string(), some_avg60_x100: some, full_avg60_x100: full })
}

pub fn parse_file_nr(text: &str) -> Option<FileHandleHealth> {
    let mut n = text.split_whitespace();
    let allocated = n.next()?.parse().ok()?;
    let unused = n.next()?.parse().ok()?;
    let maximum = n.next()?.parse().ok()?;
    if maximum == 0 || unused > allocated { return None; }
    Some(FileHandleHealth { allocated, unused, maximum })
}

/// GNU df -Pi; only mountpoints accepted by the separate df -PT disk filter.
pub fn parse_inodes(text: &str, mounts: &[String]) -> Vec<InodeHealth> {
    let allowed: HashSet<&str> = mounts.iter().map(String::as_str).collect();
    let mut result = Vec::new();
    for line in text.lines().skip(1) {
        let col: Vec<_> = line.split_whitespace().collect();
        if col.len() < 6 { continue; }
        let path = col[5..].join(" ");
        if !allowed.contains(path.as_str()) { continue; }
        let Ok(total) = col[1].parse::<u64>() else { continue };
        let Ok(used) = col[2].parse::<u64>() else { continue };
        let Ok(pct) = col[4].trim_end_matches('%').parse::<u8>() else { continue };
        if total > 0 && used <= total && pct <= 100 {
            result.push(InodeHealth { mountpoint: path, used_percent: pct });
        }
    }
    result.sort_by(|a,b| a.mountpoint.cmp(&b.mountpoint));
    result.dedup_by(|a,b| a.mountpoint == b.mountpoint);
    result.truncate(128);
    result
}

/// Parse explicit active/expected Linux MD array counts without member paths.
pub fn parse_mdstat(text: &str) -> Vec<RaidHealth> {
    let mut result = Vec::new();
    let mut current: Option<String> = None;
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some((name, detail)) = trimmed.split_once(" : ") {
            current = if name.starts_with("md") && name.len() <= 64 &&
                name.bytes().all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b)) &&
                detail.starts_with("active ") { Some(name.to_string()) } else { None };
            continue;
        }
        if trimmed.is_empty() { current = None; continue; }
        if let Some(ref name) = current {
            for token in trimmed.split_whitespace() {
                let Some(body) = token.strip_prefix('[').and_then(|t| t.strip_suffix(']')) else { continue };
                let Some((exp, act)) = body.split_once('/') else { continue };
                if let (Ok(expected), Ok(active)) = (exp.parse::<u32>(), act.parse::<u32>()) {
                    if expected > 0 && expected <= 256 && active <= expected {
                        result.push(RaidHealth { name: name.clone(), expected, active });
                        current = None;
                        break;
                    }
                }
            }
        }
    }
    result.sort_by(|a,b| a.name.cmp(&b.name));
    result.dedup_by(|a,b| a.name == b.name);
    result.truncate(64);
    result
}

fn warning(id: &'static str, title: String, explanation: &str, evidence: String, check: &str) -> Finding {
    Finding { id, title, severity: Severity::Warning, confidence: Confidence::Observed,
        explanation: explanation.into(), evidence: vec![evidence], checks: vec![check.into()] }
}

pub fn assess(s: &Snapshot, out: &mut Vec<Finding>) {
    if let Some(m) = &s.memory_health {
        if (m.available_kib as u128) * 100 < (m.total_kib as u128) * 3 &&
            m.available_kib < 512 * 1024 {
            out.push(warning("LOW_MEMORY_HEADROOM", "Host has critically low available memory".into(),
                "The kernel reports critically low MemAvailable, taking reclaimable cache into account. This is a host snapshot, not a cgroup limit or proof of an OOM incident.",
                format!("MemAvailable {} MiB / MemTotal {} MiB", m.available_kib/1024, m.total_kib/1024),
                "Inspect free -h, cgroup limits and OOM kernel logs"));
        }
    }
    for p in &s.pressure_health {
        let (id, threshold, title, explanation, check) = match p.resource.as_str() {
            "memory" => ("MEMORY_STALL_PRESSURE", 1000, "Sustained severe memory stall pressure",
                "Memory PSI full avg60 indicates a substantial fraction of the past minute with all non-idle tasks stalled; it does not identify a cause.",
                "Inspect /proc/pressure/memory and memory reclaim/cgroup limits"),
            "io" => ("IO_STALL_PRESSURE", 1500, "Sustained severe I/O stall pressure",
                "I/O PSI full avg60 indicates a substantial fraction of the past minute with all non-idle tasks stalled; it does not prove a disk hardware fault.",
                "Inspect /proc/pressure/io and device/kernel logs"),
            _ => continue,
        };
        if let Some(x) = p.full_avg60_x100 {
            if x >= threshold {
                out.push(warning(id, title.into(), explanation,
                    format!("{} PSI full avg60 {}.{:02}% (60s window)", p.resource, x/100, x%100), check));
            }
        }
    }
    if let Some(h) = &s.file_handle_health {
        let used = h.allocated.saturating_sub(h.unused);
        if (used as u128)*100 >= (h.maximum as u128)*95 {
            out.push(warning("FILE_HANDLE_LIMIT_NEAR", "Host file-handle allocation near kernel maximum".into(),
                "Kernel-wide file-nr approaches file-max. This does not imply a process-specific descriptor limit was reached.",
                format!("file-nr used={} maximum={}", used, h.maximum),
                "Inspect /proc/sys/fs/file-nr, process limits and kernel errors"));
        }
    }
    for inode in &s.inode_health {
        if inode.used_percent >= 95 {
            out.push(warning("INODE_NEAR_FULL",
                format!("Inode usage at {} is {}%", inode.mountpoint, inode.used_percent),
                "Free data blocks do not guarantee free inodes for new files.",
                format!("Mount {} inode usage {}%", inode.mountpoint, inode.used_percent),
                "Inspect df -i and filesystem metadata before cleanup"));
        }
    }
    for md in &s.raid_health {
        if md.active < md.expected {
            out.push(warning("MD_RAID_DEGRADED",
                format!("Linux MD array {} has missing active members", md.name),
                "The MD active/expected member count indicates a degraded array; not proof of physical disk failure.",
                format!("Array {}: {}/{} active members", md.name, md.active, md.expected),
                "Inspect /proc/mdstat and mdadm --detail for the affected array; do not force recovery"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn meminfo_headroom_not_memfree() {
        let m = parse_meminfo("MemTotal: 2048000 kB\nMemFree: 5 kB\nMemAvailable: 18000 kB\nSwapTotal: 1000 kB\nSwapFree: 100 kB\n").unwrap();
        assert_eq!(m.available_kib, 18_000);
        assert!(parse_meminfo("MemTotal: 2048 kB\nMemFree: 2 kB").is_none());
    }
    #[test]
    fn psi_avg60_not_transient_avg10() {
        let p = parse_pressure("memory", "some avg10=99.9 avg60=1.35 avg300=0.1 total=1\nfull avg10=99.9 avg60=0.50 avg300=0.1 total=1\n").unwrap();
        assert_eq!(p.some_avg60_x100, Some(135));
        assert_eq!(p.full_avg60_x100, Some(50));
        assert!(parse_pressure("memory", "full avg60=NaN").is_none());
        let mut s = Snapshot::default(); s.pressure_health.push(p);
        let mut v = vec![]; assess(&s, &mut v);
        assert!(!v.iter().any(|f| f.id == "MEMORY_STALL_PRESSURE"));
    }
    #[test]
    fn inode_excludes_immutable_mountpoint() {
        let t = "Filesystem Inodes IUsed IFree IUse% Mounted on\n/dev/sda1 1000 980 20 98% /\n/dev/loop0 20 20 0 100% /snap/x\n";
        assert_eq!(parse_inodes(t, &["/".into()]).len(), 1);
    }
    #[test]
    fn file_nr_validation_and_threshold() {
        assert!(parse_file_nr("9 10 20").is_none());
        let mut s = Snapshot::default(); s.file_handle_health = parse_file_nr("950 0 1000");
        let mut v = vec![]; assess(&s, &mut v);
        assert!(v.iter().any(|f| f.id == "FILE_HANDLE_LIMIT_NEAR"));
    }
    #[test]
    fn mdstat_distinguishes_degraded_and_healthy() {
        let t = "Personalities : [raid1]\nmd0 : active raid1 sda1[0] sdb1[1]\n  100 blocks [2/2] [UU]\n\nmd127 : active raid1 sdc1[0]\n  100 blocks [2/1] [U_]\n\nunused devices: <none>\n";
        let a = parse_mdstat(t); assert_eq!(a.len(), 2);
        let mut s = Snapshot::default(); s.raid_health = a;
        let mut v = vec![]; assess(&s, &mut v);
        assert_eq!(v.iter().filter(|f| f.id == "MD_RAID_DEGRADED").count(), 1);
    }
    #[test]
    fn memory_and_io_sustained_thresholds() {
        let mut s = Snapshot::default();
        s.memory_health = Some(MemoryHealth { total_kib: 2_048_000, available_kib: 20_000, swap_total_kib: None, swap_free_kib: None });
        s.pressure_health.push(PressureHealth { resource: "io".into(), some_avg60_x100: Some(9000), full_avg60_x100: Some(1800) });
        let mut v = vec![]; assess(&s, &mut v);
        assert!(v.iter().any(|f| f.id == "LOW_MEMORY_HEADROOM"));
        assert!(v.iter().any(|f| f.id == "IO_STALL_PRESSURE"));
        assert!(v.iter().all(|f| f.scope() == "current_snapshot"));
    }
}
