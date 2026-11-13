//! Read-only, local collectors. No shell invocation, network probes, or mutations.

use crate::model::{CollectorTiming, DirectoryUsage, DiskBreakdown, DiskUsage, DpkgProblem, Event, EventContext, EventKind, Focus, InterfaceStats, NetworkDropDelta, ServiceStatus, Snapshot};
use crate::progress::{Progress, ProgressMode};
use std::collections::HashSet;
use std::fs;
use std::io::{self, Read};
use std::net::IpAddr;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// Read-only collection without user-facing progress (library API compatibility).
pub fn collect(focus: Focus, since_unix: u64, max_events: usize, disk_details: bool, current_boot: bool) -> Snapshot {
    let silent = Progress::new(ProgressMode::Never, 0);
    collect_with_progress(focus, since_unix, max_events, disk_details, current_boot, &silent)
}

pub fn collect_with_progress(
    focus: Focus, since_unix: u64, max_events: usize,
    disk_details: bool, current_boot: bool, progress: &Progress,
) -> Snapshot {
    collect_with_options(focus, since_unix, max_events, disk_details, current_boot, false, progress)
}

/// Avoid remote and network-backed FUSE mounts by default, which may cause
/// statfs to block or fail. An opt-in scan preserves full coverage.
pub fn collect_with_options(
    focus: Focus, since_unix: u64, max_events: usize,
    disk_details: bool, current_boot: bool,
    include_remote_mounts: bool, progress: &Progress,
) -> Snapshot {
    collect_with_options_and_messages(
        focus, since_unix, max_events, disk_details, current_boot,
        include_remote_mounts, false, progress,
    )
}

/// Opt-in original journal evidence: can contain secrets, including in exports.
/// Keep legacy collection entrypoints unchanged for existing library callers.
#[allow(clippy::too_many_arguments)]
pub fn collect_with_options_and_messages(
    focus: Focus, since_unix: u64, max_events: usize,
    disk_details: bool, current_boot: bool,
    include_remote_mounts: bool, include_journal_messages: bool,
    progress: &Progress,
) -> Snapshot {
    let mut snapshot = Snapshot {
        os: read_os_name().unwrap_or_else(|| "Unknown Linux".into()),
        kernel: read_trimmed("/proc/sys/kernel/osrelease").unwrap_or_else(|| "Unknown".into()),
        uptime_seconds: fs::read_to_string("/proc/uptime")
            .ok()
            .and_then(|v| v.split_whitespace().next()?.parse::<f64>().ok())
            .filter(|v| v.is_finite() && *v >= 0.0)
            .map(|v| v as u64),
        journal_current_boot_only: current_boot,
        ..Snapshot::default()
    };

    let network_sample_started = Instant::now();
    if focus.includes(Focus::Network) {
        progress.begin("Inspecting network");
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
            progress.detail("systemctl is-active systemd-resolved (max 20s)");
            snapshot.resolved_active = match capture_timed(&mut snapshot, "systemd-resolved", "systemctl", &["is-active", "systemd-resolved.service"]) {
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
        progress.complete_step();
    }

    if focus.includes(Focus::Services) {
        progress.begin("Checking systemd services");
        progress.detail("systemctl --failed (max 20s)");
        match capture_timed(&mut snapshot, "systemd system units", "systemctl", &["list-units", "--failed", "--no-legend", "--plain", "--no-pager"]) {
            Ok((true, output)) => {
                snapshot.failed_units = parse_failed_units(&output);
                snapshot.system_units_checked = true;
            },
            Ok((false, _)) => snapshot.notes.push("Failed systemd units unavailable (systemd may be absent or inaccessible)".into()),
            Err(_) => snapshot.notes.push("systemctl is not available".into()),
        }
        // User and system units belong to separate systemd managers.
        // A missing user bus means unknown, not zero failed user units.
        progress.detail("systemctl --user --failed (max 20s)");
        match capture_timed(&mut snapshot, "systemd user units", "systemctl", &["--user", "list-units", "--failed", "--no-legend", "--plain", "--no-pager"]) {
            Ok((true, output)) => snapshot.failed_user_units = Some(parse_failed_units(&output)),
            Ok((false, _)) => snapshot.notes.push("User systemd manager unavailable; user units not checked".into()),
            Err(_) => snapshot.notes.push("systemctl --user unavailable; user units not checked".into()),
        }
        progress.complete_step();
    }

    if focus.includes(Focus::System) {
        progress.begin("Checking storage and resource health");
        progress.detail("Reading kernel memory, pressure and RAID snapshots");
        crate::health::collect_kernel(&mut snapshot);
        if let Ok(info) = fs::read_to_string("/proc/self/mountinfo") {
            snapshot.root_filesystem_read_only = parse_root_mount_read_only(&info);
        }
        // pacman -Dk checks local package database consistency without any
        // sync/installation/removal, and only when pacman's DB is present.
        if fs::metadata("/var/lib/pacman/local").is_ok() {
            progress.detail("pacman -Dk (max 20s)");
            snapshot.pacman_checked = Some(false);
            match capture_timed(&mut snapshot, "pacman -Dk", "pacman", &["-Dk"]) {
                Ok((success, _)) => {
                    snapshot.pacman_checked = Some(true);
                    snapshot.pacman_consistent = Some(success);
                }
                Err(error) => snapshot.notes.push(format!(
                    "pacman local database consistency not verified: {error}"
                )),
            }
        }
        // Read the package database via dpkg-query's stable, machine-readable
        // status abbreviation. Do not invoke apt/dpkg configuration commands.
        if fs::metadata("/var/lib/dpkg/status").is_ok() {
            progress.detail("dpkg-query (max 20s)");
            snapshot.dpkg_checked = Some(false);
            match capture_timed(&mut snapshot, "dpkg-query", "dpkg-query",
                &["--show", "--showformat=${binary:Package}\\t${db:Status-Abbrev}\\n"]) {
                Ok((true, output)) => {
                    let problems = parse_dpkg_status(&output);
                    snapshot.dpkg_problem_count = problems.len();
                    snapshot.dpkg_problems = problems.into_iter().take(50).collect();
                    snapshot.dpkg_checked = Some(true);
                }
                Ok((false, _)) => snapshot.notes.push(
                    "dpkg package status query failed; package health not verified".into()),
                Err(_) => snapshot.notes.push(
                    "dpkg status database found but dpkg-query unavailable; package health not verified".into()),
            }
        }
        if !include_remote_mounts {
            if let Ok(info) = fs::read_to_string("/proc/self/mountinfo") {
                if has_remote_mounts(&info) {
                    snapshot.notes.push("Remote or network-backed FUSE filesystem usage excluded by default; use --include-remote-mounts to include (may be slow)".into());
                }
            }
        }
        progress.detail(if include_remote_mounts {
            "df -PT (including remote mounts; max 20s)"
        } else {
            "df -PT --local (excluding remote/FUSE; max 20s)"
        });
        match capture_timed(&mut snapshot, "df disk usage", "df", df_args(false, include_remote_mounts)) {
            Ok((complete, output)) => {
                snapshot.disks = parse_df(&output);
                snapshot.disk_usage_checked = !snapshot.disks.is_empty();
                if !complete {
                    snapshot.notes.push(format!(
                        "Filesystem usage partially collected: df exited nonzero; retained {} valid entries",
                        snapshot.disks.len()
                    ));
                }
                if snapshot.disks.is_empty() {
                    snapshot.notes.push("Filesystem usage unavailable: df returned no valid entries".into());
                }
            }
            Err(error) => snapshot.notes.push(format!(
                "Filesystem usage unavailable: df failed ({error})"
            )),
        }
        // Keep inodes constrained to the same filesystem selection.
        let mounts: Vec<String> = snapshot.disks.iter().map(|d| d.mountpoint.clone()).collect();
        if !mounts.is_empty() {
            progress.detail(if include_remote_mounts {
                "df -Pi (including remote mounts; max 20s)"
            } else {
                "df -Pi --local (excluding remote/FUSE; max 20s)"
            });
            match capture_timed(&mut snapshot, "df inode usage", "df", df_args(true, include_remote_mounts)) {
                Ok((complete, output)) => {
                    snapshot.inode_health = crate::health::parse_inodes(&output, &mounts);
                    if !complete {
                        snapshot.notes.push("Inode usage partially collected: df exited nonzero; existing entries retained".into());
                    }
                }
                Err(error) => snapshot.notes.push(format!("Filesystem inode counts unavailable: df failed ({error})")),
            }
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
                progress.detail("du --max-depth=2 (max 120s; optional disk details)");
                match capture_timed(&mut snapshot, "du disk details", "du", &["-x", "-k", "--max-depth=2", "--", path.as_str()]) {
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
        progress.complete_step();
    }

    progress.begin("Reading journal entries");
    progress.detail("journalctl (max 45s)");
    let since_arg = format!("--since=@{since_unix}");
    // Request one extra entry as a sentinel so we can distinguish an exact
    // N-entry result from a genuinely truncated history. Only N are analyzed.
    let count_arg = format!("--lines={}", max_events.saturating_add(1));
    let mut journal_args = vec!["--no-pager", "--output=short-unix"];
    if current_boot { journal_args.push("--boot=0"); }
    journal_args.push(since_arg.as_str());
    journal_args.push(count_arg.as_str());
    match capture_timed(&mut snapshot, "journalctl", "journalctl", &journal_args) {
        Ok((true, output)) => {
            snapshot.journal_checked = true;
            let (sample, truncated, examined) =
                limit_journal_sample(&output, max_events);
            snapshot.journal_truncated = truncated;
            snapshot.journal_entries_examined = examined;
            if truncated {
                snapshot.notes.push(format!(
                    "Journal window incomplete: only the newest {max_events} entries were analyzed; older entries in the requested period were excluded. Increase --max-events (up to 10000) or narrow --since"
                ));
            }
            if examined == 0 {
                snapshot.notes.push("No journal entries were returned; logging may be unavailable, inaccessible or simply quiet".into());
            }
            (snapshot.events, snapshot.event_contexts) =
                parse_journal_with_context_and_messages(&sample, include_journal_messages);
        }
        Ok((false, _)) => snapshot.notes.push(
            "Journal collection failed or access is restricted; verify journalctl permissions and available boots".into(),
        ),
        Err(error) => snapshot.notes.push(format!(
            "Journal not collected (missing command, timeout, oversized output or I/O error): {error}"
        )),
    }

    // A historical failure is never proof of a currently failed service.
    // Query only whitelisted journal-attributed units, never command-line names.
    // A system unit may not exist in the user's manager and vice versa:
    // probe both bounded managers, record *which manager answered*, and never
    // invent an active state when a transient GUI unit has disappeared.
    if focus.includes(Focus::Services) && snapshot.journal_checked {
        let mut names: Vec<String> = snapshot.events.iter()
            .filter(|e| e.kind == EventKind::ServiceFailure)
            .filter(|e| e.source.as_deref() == Some("systemd"))
            .filter_map(|e| e.subject.as_ref())
            .filter(|name| safe_service_unit_name(name))
            .cloned().collect();
        names.sort();
        names.dedup();
        names.truncate(4);
        for name in names {
            let user_first = snapshot.failed_user_units.as_ref()
                .is_some_and(|units| units.contains(&name))
                || name.starts_with("app-");
            let managers = if user_first { ["user", "system"] }
                else { ["system", "user"] };
            let mut found = None;
            for manager in managers {
                if manager == "user" && snapshot.failed_user_units.is_none() {
                    // The user manager was unavailable during collection.
                    continue;
                }
                progress.detail("systemctl show historical unit in correct manager (max 3s each)");
                let property = "--property=LoadState,ActiveState,SubState,Result,NRestarts,Transient,FragmentPath,ExecMainStatus";
                let args_user = ["--user", "show", "--no-pager", property, "--", name.as_str()];
                let args_system = ["show", "--no-pager", property, "--", name.as_str()];
                let args: &[&str] = if manager == "user" { &args_user } else { &args_system };
                let start = Instant::now();
                let state = run_capture_with_limits(
                    "systemctl", args, Duration::from_secs(3), 8 * 1024);
                let parsed = match state {
                    Ok((true, output)) => parse_service_status(&name, manager, &output),
                    _ => None,
                };
                snapshot.collector_timings.push(CollectorTiming {
                    name: if manager == "user" {
                        "systemd user historical unit status"
                    } else {
                        "systemd system historical unit status"
                    },
                    elapsed_ms: start.elapsed().as_millis().min(u64::MAX as u128) as u64,
                    succeeded: parsed.is_some(),
                });
                if parsed.is_some() {
                    found = parsed;
                    break;
                }
            }
            if let Some(status) = found {
                // The general journal classifier retains only failure lines.
                // An opt-in, scoped unit journal can provide the application's
                // own fatal/error message immediately preceding that failure.
                // Never query unrelated units or retain extra text by default.
                if include_journal_messages {
                    let mut args = vec!["--no-pager", "--output=short-unix"];
                    if current_boot { args.push("--boot=0"); }
                    args.push(since_arg.as_str());
                    args.push("--lines=80");
                    if status.manager == "user" {
                        args.push("--user");
                        args.push("--user-unit");
                    } else {
                        args.push("-u");
                    }
                    args.push(name.as_str());
                    progress.detail("journalctl for identified failed unit (max 3s)");
                    let start = Instant::now();
                    let result = run_capture_with_limits(
                        "journalctl", &args, Duration::from_secs(3), 64 * 1024);
                    let success = matches!(result, Ok((true, _)));
                    if let Ok((true, content)) = result {
                        append_targeted_unit_journal_messages(
                            &mut snapshot.event_contexts, &name, &content,
                        );
                    }
                    snapshot.collector_timings.push(CollectorTiming {
                        name: "systemd targeted unit journal",
                        elapsed_ms: start.elapsed().as_millis().min(u64::MAX as u128) as u64,
                        succeeded: success,
                    });
                    if !success {
                        snapshot.notes.push(format!(
                            "Extra journal context unavailable for {name}; original classified events remain intact"
                        ));
                    }
                }
                snapshot.service_statuses.push(status);
            } else {
                snapshot.notes.push(format!(
                    "Current unit state unavailable for {name} in accessible system/user managers;                      transient units may have disappeared; state not assumed active"));
            }
        }
    }

    // A second passive sample enables in-run counter deltas. Never infer new
    // packet drops from a single cumulative counter and never delay diagnosis.
    if focus.includes(Focus::Network) && !snapshot.interfaces.is_empty() {
        match fs::read_to_string("/proc/net/dev") {
            Ok(raw) => {
                let latest = parse_interfaces(&raw);
                snapshot.interface_drop_deltas = Some(compare_interface_drops(
                    &snapshot.interfaces, &latest,
                    network_sample_started.elapsed().as_millis().min(u64::MAX as u128) as u64,
                ));
            }
            Err(_) => snapshot.notes.push(
                "Second interface counter sample unavailable; packet-drop deltas not verified".into()),
        }
    }
    progress.complete_step();
    snapshot
}

/// Keep only the newest N journal lines. journalctl was asked for N+1,
/// providing an exact sentinel when the requested journal window is larger.
fn limit_journal_sample(content: &str, max_events: usize) -> (String, bool, usize) {
    let lines: Vec<&str> = content.lines().collect();
    let truncated = max_events > 0 && lines.len() > max_events;
    let kept = if truncated {
        &lines[lines.len() - max_events..]
    } else {
        lines.as_slice()
    };
    let examined = if kept.len() == 1 && kept[0].contains("-- No entries --") {
        0
    } else {
        kept.len()
    };
    (kept.join("\n"), truncated, examined)
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

/// Bounded external collection: individual commands are never allowed to
/// block a diagnosis indefinitely or retain unlimited stdout in memory.
/// No shell, privilege escalation, network probes or data mutation.
/// `df` should not stat remote mounts by default. Explicitly exclude the
/// most common network-backed FUSE types, which GNU df may treat as local.
fn df_args(inodes: bool, include_remote: bool) -> &'static [&'static str] {
    match (inodes, include_remote) {
        (false, true) => &["-PT"],
        (true, true) => &["-Pi"],
        (false, false) => &[
            "-PT", "--local", "--exclude-type=fuse.rclone",
            "--exclude-type=fuse.sshfs", "--exclude-type=fuse.s3fs",
            "--exclude-type=fuse.davfs", "--exclude-type=fuse.gvfsd-fuse",
        ],
        (true, false) => &[
            "-Pi", "--local", "--exclude-type=fuse.rclone",
            "--exclude-type=fuse.sshfs", "--exclude-type=fuse.s3fs",
            "--exclude-type=fuse.davfs", "--exclude-type=fuse.gvfsd-fuse",
        ],
    }
}

/// Detect known remote mounts only by parsing mountinfo; never stat them.
/// Other network filesystem types may exist and remain unrecognized.
fn has_remote_mounts(content: &str) -> bool {
    content.lines().any(|line| {
        let Some((_, details)) = line.split_once(" - ") else { return false };
        let Some(kind) = details.split_whitespace().next() else { return false };
        matches!(kind,
            "nfs" | "nfs4" | "cifs" | "smb3" | "9p" | "ceph" |
            "fuse.rclone" | "fuse.sshfs" | "fuse.s3fs" |
            "fuse.davfs" | "fuse.gvfsd-fuse")
    })
}

/// Time every external collector without exposing command output or arguments.
/// Details appear in the report so transient slowdowns can be attributed.
fn capture_timed(
    snapshot: &mut Snapshot, label: &'static str,
    command: &str, args: &[&str],
) -> io::Result<(bool, String)> {
    let start = Instant::now();
    let result = run_capture(command, args);
    let ms = start.elapsed().as_millis().min(u64::MAX as u128) as u64;
    snapshot.collector_timings.push(CollectorTiming {
        name: label,
        elapsed_ms: ms,
        succeeded: matches!(&result, Ok((true, _))),
    });
    result
}

fn run_capture(command: &str, args: &[&str]) -> io::Result<(bool, String)> {
    let seconds = if command == "du" { 120 } else if command == "journalctl" { 45 } else { 20 };
    let max_bytes = if command == "journalctl" { 32 * 1024 * 1024 } else { 16 * 1024 * 1024 };
    run_capture_with_limits(command, args, Duration::from_secs(seconds), max_bytes)
}

fn run_capture_with_limits(
    command: &str, args: &[&str], timeout: Duration, max_bytes: usize,
) -> io::Result<(bool, String)> {
    let mut child = Command::new(command)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;

    let mut stdout = child.stdout.take().ok_or_else(|| {
        io::Error::new(io::ErrorKind::Other, "Collector stdout pipe not available")
    })?;
    // Drain stdout concurrently while the process runs, avoiding OS pipe
    // deadlocks. Keep at most max_bytes; the reader closes the pipe on overflow.
    let reader = thread::spawn(move || -> io::Result<(Vec<u8>, bool)> {
        let mut bytes = Vec::new();
        let mut buffer = [0u8; 8192];
        loop {
            let n = stdout.read(&mut buffer)?;
            if n == 0 { break; }
            let left = max_bytes.saturating_sub(bytes.len());
            if n > left {
                bytes.extend_from_slice(&buffer[..left]);
                return Ok((bytes, true));
            }
            bytes.extend_from_slice(&buffer[..n]);
        }
        Ok((bytes, false))
    });

    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if start.elapsed() >= timeout => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = reader.join();
                return Err(io::Error::new(io::ErrorKind::TimedOut,
                    format!("{command} exceeded collection timeout")));
            }
            Ok(None) => thread::sleep(Duration::from_millis(25)),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = reader.join();
                return Err(error);
            }
        }
    };
    let (bytes, truncated) = reader.join().map_err(|_| {
        io::Error::new(io::ErrorKind::Other, "Collector output reader panicked")
    })??;
    if truncated {
        return Err(io::Error::new(io::ErrorKind::InvalidData,
            format!("{command} output exceeded bounded collection limit")));
    }
    Ok((status.success(), String::from_utf8_lossy(&bytes).into_owned()))
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

/// Inspect the visible root mount of this mount namespace without
/// resolving paths or touching any filesystem contents. The final match
/// represents the most recent visible entry for the root mountpoint.
/// Intended read-only immutable systems may legitimately return true.
pub fn parse_root_mount_read_only(content: &str) -> Option<bool> {
    let mut result = None;
    for line in content.lines() {
        let Some((left, _)) = line.split_once(" - ") else { continue };
        let columns: Vec<&str> = left.split_whitespace().collect();
        if columns.len() < 6 || columns[4] != "/" { continue; }
        let opts = columns[5].split(',').collect::<Vec<_>>();
        if opts.contains(&"ro") { result = Some(true); }
        else if opts.contains(&"rw") { result = Some(false); }
    }
    result
}

pub fn parse_resolv_conf(content: &str) -> (usize, bool) {
    // An arbitrary token after "nameserver" is not a configured DNS address.
    // IPv6 link-local resolver scopes may include %interface suffix.
    let servers: Vec<&str> = content.lines().filter_map(|line| {
        let line = line.split('#').next()?.trim();
        let mut parts = line.split_whitespace();
        if parts.next()? != "nameserver" { return None; }
        let candidate = parts.next()?;
        let literal = candidate.split('%').next().unwrap_or(candidate);
        if literal.parse::<IpAddr>().is_ok() {
            Some(candidate)
        } else {
            None
        }
    }).collect();
    let stub = servers.iter().any(|s| *s == "127.0.0.53" || *s == "127.0.0.54");
    (servers.len(), stub)
}

/// Only compare interfaces seen in both samples with nondecreasing counters.
pub fn compare_interface_drops(
    before: &[InterfaceStats], after: &[InterfaceStats], elapsed_ms: u64,
) -> Vec<NetworkDropDelta> {
    if elapsed_ms == 0 { return Vec::new(); }
    before.iter().filter_map(|old| {
        let new = after.iter().find(|item| item.name == old.name)?;
        Some(NetworkDropDelta {
            name: old.name.clone(),
            rx_dropped_delta: new.rx_dropped.checked_sub(old.rx_dropped)?,
            tx_dropped_delta: new.tx_dropped.checked_sub(old.tx_dropped)?,
            elapsed_ms,
        })
    }).take(64).collect()
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

/// Parse dpkg-query --showformat='${binary:Package}\\t${db:Status-Abbrev}\\n'.
/// The 3-character status contains desired state, actual state, error flag.
/// Ignore fully installed packages and harmless "config-files" leftovers.
pub fn parse_dpkg_status(content: &str) -> Vec<DpkgProblem> {
    let mut problems = Vec::new();
    for line in content.lines() {
        let Some((name, abbrev)) = line.split_once('\t') else { continue };
        if name.is_empty() || name.len() > 128 ||
            !name.bytes().all(|b| b.is_ascii_alphanumeric() || b"+-.:_".contains(&b)) {
            continue;
        }
        let bytes = abbrev.trim_end_matches('\r').as_bytes();
        if bytes.len() != 3 { continue; }
        let reinstall_required = bytes[2] == b'R';
        let state = match bytes[1] {
            b'H' => "half-installed",
            b'U' => "unpacked",
            b'F' => "half-configured",
            b'W' => "triggers-awaited",
            b't' => "triggers-pending",
            _ if reinstall_required => "reinstallation-required",
            _ => continue, // installed, config-files, not-installed
        };
        problems.push(DpkgProblem {
            name: name.to_string(),
            state: state.to_string(),
            reinstall_required,
        });
    }
    problems.sort_by(|a, b| a.name.cmp(&b.name));
    problems.dedup_by(|a, b| a.name == b.name);
    problems
}

fn safe_service_unit_name(unit: &str) -> bool {
    !unit.is_empty() && unit.len() <= 128 && unit.ends_with(".service")
        && !unit.starts_with('-')
        && unit.bytes().all(|b| b.is_ascii_alphanumeric()
            || matches!(b, b'@' | b'.' | b'_' | b'-' | b':'))
}

fn parse_service_status(unit: &str, manager: &str, output: &str) -> Option<ServiceStatus> {
    if !safe_service_unit_name(unit) || !matches!(manager, "system" | "user") {
        return None;
    }
    let mut load = None;
    let mut active = None;
    let mut sub = None;
    let mut result = None;
    let mut n_restarts = None;
    let mut transient = None;
    let mut desktop_entry = false;
    let mut exec_main_status = None;
    for line in output.lines() {
        let Some((key, value)) = line.split_once('=') else { continue };
        let value = value.trim();
        // Never keep arbitrary FragmentPath/ExecStart contents. Retain only
        // the boolean that the unit came from a .desktop launcher.
        if key == "FragmentPath" {
            desktop_entry = value.starts_with("/usr/share/applications/")
                && value.ends_with(".desktop") && value.len() <= 256;
            continue;
        }
        if key == "Transient" {
            transient = match value { "yes" => Some(true), "no" => Some(false), _ => None };
            continue;
        }
        if key == "ExecMainStatus" {
            exec_main_status = value.parse::<u64>().ok().filter(|n| *n <= 255);
            continue;
        }
        if value.len() > 64 || !value.bytes().all(|c| c.is_ascii_alphanumeric()
            || matches!(c, b'-' | b'_')) { continue; }
        match key {
            "LoadState" => load = Some(value.to_owned()),
            "ActiveState" => active = Some(value.to_owned()),
            "SubState" => sub = Some(value.to_owned()),
            "Result" => result = Some(value.to_owned()),
            "NRestarts" => n_restarts = value.parse::<u64>().ok(),
            _ => {},
        }
    }
    let load_state = load?;
    if load_state == "not-found" { return None; }
    Some(ServiceStatus {
        unit: unit.to_owned(), manager: manager.to_owned(),
        transient, desktop_entry, exec_main_status,
        load_state, active_state: active?, sub_state: sub?,
        result: result?, n_restarts,
    })
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
     "wwan", "veth", "bond", "br", "tun", "tap", "wg", "docker"]
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
            // The colon belongs to the original journal token. Checking it
            // after trimming (as before v0.6.0) made this branch unreachable.
            if let Some(candidate) = word.strip_suffix(':') {
                if let Some(name) = identifier(candidate) {
                    if network_interface(&name) { return Some(name); }
                }
            }
        }
    }
    if matches!(kind, EventKind::UsbDisconnect | EventKind::UsbTransportError) {
        // Typical kernel records: "usb 1-5.1: USB disconnect..." or
        // "r8152-cfgselector 1-1.2: USB disconnect...". Device topology
        // identifiers are kept, not serial numbers or command arguments.
        for word in text.split_whitespace() {
            if let Some(candidate) = word.strip_suffix(':') {
                if candidate.len() <= 32 && candidate.starts_with(|c: char| c.is_ascii_digit())
                    && candidate.bytes().all(|c| c.is_ascii_digit() || c == b'.' || c == b'-')
                    && candidate.contains('-') {
                    return Some(candidate.to_string());
                }
            }
        }
    }
    if matches!(kind, EventKind::ServiceFailure | EventKind::ServiceRestartScheduled) {
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

/// Extract only explicit diagnostic codes or canonical status transitions.
///
/// Never retain arbitrary message text: journal records may contain Wi-Fi
/// BSSIDs, hostnames, URLs, client names, credentials and command arguments.
/// Accepted indicators contain only short ASCII state/reason/status codes.
fn diagnostic_signal(body: &str, kind: EventKind) -> Option<String> {
    fn token(s: &str) -> Option<String> {
        let candidate = s.trim_start_matches(['\'', '"', '('])
            .split(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '/' | '=' | '.')))
            .next()?.trim_end_matches(['.', '/']);
        if candidate.is_empty() || candidate.len() > 48 ||
            !candidate.bytes().all(|c| c.is_ascii_alphanumeric() || b"_-./=".contains(&c)) ||
            candidate.contains("..") {
            return None;
        }
        Some(candidate.to_string())
    }
    let lower = body.to_ascii_lowercase();
    let mut parts: Vec<String> = Vec::new();
    for (marker, label) in [
        ("status=", "exit_status"),
        ("code=", "process_code"),
        ("reason=", "reason"),
        ("reason: ", "reason"),
        ("failed with result ", "systemd_result"),
        ("result=", "result"),
        ("errno=", "errno"),
    ] {
        if let Some(pos) = lower.find(marker) {
            let suffix = &body[pos + marker.len()..];
            if let Some(value) = token(suffix) {
                // Whitelist numeric codes and documented status names only.
                // Arbitrary alphanumeric values can be secrets, hostnames or
                // SSIDs; never forward them even under a field called "reason".
                let digits = |v: &str, max: usize| {
                    !v.is_empty() && v.len() <= max && v.bytes().all(|c| c.is_ascii_digit())
                };
                let normalized = match label {
                    "exit_status" if matches!(kind, EventKind::ServiceFailure) => {
                        let (number, suffix) = value.split_once('/').unwrap_or((&value, ""));
                        if !digits(number, 5) { None }
                        else if matches!(suffix.to_ascii_uppercase().as_str(),
                            "FAILURE" | "SUCCESS" | "TERM" | "KILL" | "ABRT"
                            | "SEGV" | "BUS" | "ILL" | "TRAP") {
                            Some(format!("{number}/{suffix}"))
                        } else { Some(number.to_string()) }
                    }
                    "reason" if matches!(kind, EventKind::LinkDown
                            | EventKind::DhcpProblem | EventKind::WifiRoamStart) => {
                        let (number, suffix) = value.split_once('=').unwrap_or((&value, ""));
                        if digits(number, 3) {
                            if matches!(suffix.to_ascii_uppercase().as_str(),
                                "PREV_AUTH_NOT_VALID" | "DISASSOC_DUE_TO_INACTIVITY"
                                | "DEAUTH_LEAVING" | "CLASS2_FRAME_FROM_NONAUTH_STA"
                                | "CLASS3_FRAME_FROM_NONASSOC_STA" | "DISASSOC_AP_BUSY"
                                | "MIC_FAILURE" | "4WAY_HANDSHAKE_TIMEOUT"
                                | "GROUP_KEY_UPDATE_TIMEOUT") {
                                Some(format!("{number}={suffix}"))
                            } else { Some(number.to_string()) }
                        } else if matches!(value.to_ascii_lowercase().as_str(),
                            "carrier-changed" | "ssid-not-found" | "supplicant-disconnect"
                            | "no-secrets" | "dhcp-failed" | "ip-config-unavailable"
                            | "supplicant-timeout" | "connection-removed"
                            | "user-requested" | "deauth" | "disassoc") {
                            Some(value)
                        } else { None }
                    }
                    "systemd_result" | "result"
                        if matches!(kind, EventKind::ServiceFailure |
                            EventKind::ServiceRestartScheduled) &&
                            matches!(value.to_ascii_lowercase().as_str(),
                                "exit-code" | "signal" | "core-dump" | "timeout"
                                | "start-limit-hit" | "resources" | "protocol"
                                | "watchdog" | "oom-kill" | "success") => Some(value),
                    "process_code" if matches!(kind, EventKind::ServiceFailure) &&
                        matches!(value.to_ascii_lowercase().as_str(),
                            "exited" | "killed" | "dumped") => Some(value),
                    "errno" if digits(&value, 4) => Some(value),
                    _ => None,
                };
                if let Some(value) = normalized {
                    parts.push(format!("{label}={value}"));
                }
            }
        }
    }
    // Canonical patterns are useful and contain no journal arguments.
    if matches!(kind, EventKind::LinkDown) &&
        lower.contains("activated -> disconnected") {
        parts.push("transition=activated_to_disconnected".into());
    } else if matches!(kind, EventKind::LinkDown) &&
        lower.contains("ctrl-event-disconnected") {
        parts.push("transition=wpa_disconnected".into());
    }
    if matches!(kind, EventKind::DhcpProblem) &&
        (lower.contains("timed out") || lower.contains("timeout")) {
        parts.push("dhcp_timeout=reported".into());
    }
    if matches!(kind, EventKind::DnsProblem) && lower.contains("servfail") {
        parts.push("dns_reply=SERVFAIL".into());
    }
    if matches!(kind, EventKind::ServiceFailure) &&
        lower.contains("start request repeated too quickly") {
        parts.push("start_limit=hit".into());
    }
    if parts.is_empty() {
        None
    } else {
        // Whitelisted markers only, capped for bounded JSON and clipboard use.
        Some(parts.into_iter().take(4).collect::<Vec<_>>().join("; ").chars().take(160).collect())
    }
}

/// Parse journalctl short-unix output into bounded classified events.
/// The default path never retains original journal messages.
pub fn parse_journal(content: &str) -> Vec<Event> {
    parse_journal_with_context(content).0
}

pub fn parse_journal_with_context(content: &str) -> (Vec<Event>, Vec<EventContext>) {
    parse_journal_with_context_and_messages(content, false)
}

/// Remove the journald hostname, emitter and PID, retaining message only.
fn journal_message_body(body: &str) -> Option<String> {
    let (_, message) = body.split_once(": ")?;
    let message = message.trim();
    if message.is_empty() { return None; }
    // Prevent active terminal escapes and bound each potentially sensitive line.
    let mut excerpt: String = message.chars()
        .filter(|c| !c.is_control())
        .take(240).collect();
    if message.chars().count() > 240 { excerpt.push('…'); }
    if excerpt.is_empty() { None } else { Some(std::mem::take(&mut excerpt)) }
}

/// Add high-signal application messages from an explicitly scoped `journalctl -u`
/// result. A unit association must come from journald's unit filter, *never*
/// from mere temporal proximity in the global journal. These are context,
/// not verified causes. Raw text is retained only in the existing opt-in
/// journal-message path and is constrained by that path's per-event limits.
fn append_targeted_unit_journal_messages(
    contexts: &mut [EventContext], unit: &str, content: &str,
) {
    let mut relevant = Vec::new();
    for line in content.lines() {
        let mut parts = line.trim().splitn(2, char::is_whitespace);
        let Some(timestamp) = parts.next()
            .and_then(|v| v.split('.').next())
            .and_then(|v| v.parse::<u64>().ok()) else { continue; };
        let body = parts.next().unwrap_or("");
        // These extra excerpts should explain what the *application* reported,
        // not repeat the same systemd exit-code/failed-state messages.
        if matches!(event_source(body).as_deref(), None |
            Some("systemd" | "sudo" | "doas" | "pkexec")) {
            continue;
        }
        let Some(message) = journal_message_body(body) else { continue; };
        let lower = message.to_ascii_lowercase();
        if !["fatal", "error", "[err]", "failed", "failure", "unable",
              "denied", "permission", "exiting", "panic", "segfault",
              "crash", "exception", "abort"]
            .iter().any(|needle| lower.contains(needle)) {
            continue;
        }
        relevant.push((timestamp, message));
    }
    for ctx in contexts.iter_mut().filter(|ctx|
        ctx.kind == EventKind::ServiceFailure &&
        ctx.subject.as_deref() == Some(unit)) {
        let mut candidates: Vec<_> = relevant.iter().filter(|(timestamp, _)|
            (*timestamp <= ctx.timestamp &&
             ctx.timestamp.saturating_sub(*timestamp) <= 60) ||
            (*timestamp > ctx.timestamp &&
             timestamp.saturating_sub(ctx.timestamp) <= 10)
        ).collect();
        // Nearest context first; the application error must not be displaced
        // by less useful 'Failed with result' messages already collected.
        candidates.sort_by_key(|(timestamp, _)| ctx.timestamp.abs_diff(*timestamp));
        let mut messages = Vec::new();
        for (_, message) in candidates {
            if !messages.contains(message) {
                messages.push(message.clone());
            }
            if messages.len() == 3 { break; }
        }
        for message in &ctx.journal_messages {
            if messages.len() == 3 { break; }
            if !messages.contains(message) {
                messages.push(message.clone());
            }
        }
        ctx.journal_messages = messages;
    }
}

pub fn parse_journal_with_context_and_messages(
    content: &str, include_journal_messages: bool,
) -> (Vec<Event>, Vec<EventContext>) {
    let mut seen = HashSet::new();
    let mut events = Vec::new();
    let mut contexts = Vec::new();
    let mut network_transition_until = 0_u64;
    for line in content.lines() {
        let mut parts = line.trim().splitn(2, char::is_whitespace);
        let unix = match parts.next().and_then(|v| v.split('.').next())
            .and_then(|v| v.parse::<u64>().ok()) {
            Some(v) => v,
            None => continue,
        };
        let body = parts.next().unwrap_or("");
        let source = event_source(body);
        let lower = body.to_ascii_lowercase();
        // sudo/doas/pkexec audit records frequently contain the full invoked
        // command. Words in --grep patterns or arguments are not observed
        // service, network, memory or storage failures.
        let privilege_wrapper = matches!(source.as_deref(), Some("sudo" | "doas" | "pkexec"));
        if privilege_wrapper && (lower.contains("command=") || lower.contains("cmd=")) {
            continue;
        }
        if lower.contains("stopping network manager")
            || lower.contains("starting network manager")
            || (lower.contains("networkmanager[") && (
                lower.contains("caught sigterm") || lower.contains("is starting...")
            )) {
            network_transition_until = unix.saturating_add(60);
        }
        let kind = match classify_log_line(body) { Some(k) => k, None => continue };
        // A privilege wrapper reporting a command error is not systemd
        // declaring a service unit failure. Keep genuine systemd entries.
        if privilege_wrapper && kind == EventKind::ServiceFailure { continue; }
        if matches!(kind, EventKind::LinkDown | EventKind::DhcpProblem)
            && unix <= network_transition_until {
            continue;
        }
        let subject = event_subject(body, kind);
        // Preserve indicators and at most three original messages per incident
        // and second. Default reports never include the original text.
        let signal = diagnostic_signal(body, kind);
        let message = if include_journal_messages {
            journal_message_body(body)
        } else { None };
        if signal.is_some() || message.is_some() {
            if let Some(context) = contexts.iter_mut().find(|ctx: &&mut EventContext|
                ctx.timestamp == unix && ctx.kind == kind && ctx.subject == subject) {
                if let Some(signal) = signal {
                    for marker in signal.split("; ") {
                        if !context.signal.split("; ").any(|old| old == marker) {
                            let joined = if context.signal.is_empty() {
                                marker.to_string()
                            } else { format!("{}; {}", context.signal, marker) };
                            if joined.len() <= 160 { context.signal = joined; }
                        }
                    }
                }
                if let Some(message) = message {
                    if context.journal_messages.len() < 3
                        && !context.journal_messages.contains(&message) {
                        context.journal_messages.push(message);
                    }
                }
            } else {
                contexts.push(EventContext {
                    timestamp: unix, kind, subject: subject.clone(),
                    signal: signal.unwrap_or_default(),
                    journal_messages: message.into_iter().collect(),
                });
            }
        }
        if seen.insert((unix, kind.as_str(), subject.clone())) {
            events.push(Event {
                timestamp: unix, kind, subject, source,
            });
        }
    }
    events.sort_by(|a,b| a.timestamp.cmp(&b.timestamp)
        .then(a.kind.as_str().cmp(b.kind.as_str()))
        .then(a.subject.cmp(&b.subject)));
    if include_journal_messages {
        let retained: HashSet<_> = events.iter().rev().take(300)
            .map(|e| (e.timestamp, e.kind.as_str(), e.subject.clone())).collect();
        for context in &mut contexts {
            if !retained.contains(&(context.timestamp, context.kind.as_str(), context.subject.clone())) {
                context.journal_messages.clear();
            }
        }
    }
    (events, contexts)
}

pub fn classify_log_line(text: &str) -> Option<EventKind> {
    let t = text.to_ascii_lowercase();

    // Expected handoffs must not be classified as hard link failures. A
    // successful association is evidence of reconnect, not Internet access.
    if t.contains("deauthenticating from") && t.contains("by local choice")
        && ["wlan", "wlp", "wlx"].iter().any(|marker| t.contains(marker)) {
        return Some(EventKind::WifiRoamStart);
    }
    if t.contains(": associated")
        && ["wlan", "wlp", "wlx"].iter().any(|marker| t.contains(marker)) {
        return Some(EventKind::WifiAssociated);
    }
    if t.contains("scheduled restart job") && t.contains(".service:") {
        return Some(EventKind::ServiceRestartScheduled);
    }
    if t.contains("usb disconnect, device number") {
        return Some(EventKind::UsbDisconnect);
    }
    if t.contains("stop submitting intr, status -71") && t.contains("r8152") {
        return Some(EventKind::UsbTransportError);
    }

    if ["too many open files", "file table overflow", "emfile", "enfile"]
        .iter().any(|p| t.contains(p)) {
        return Some(EventKind::FileHandleExhaustion);
    }

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
    fn diagnostic_indicators_are_specific_but_never_include_raw_log_secrets() {
        let logs = concat!(
            "1794499327.0 machine kernel: wlan0: deauthenticated from aa:bb:cc:dd:ee:ff (Reason: 2=PREV_AUTH_NOT_VALID) secret=keep-me-private\n",
            "1794499328.0 machine systemd[1]: sshd.service: Main process exited, code=exited, status=1/FAILURE token=topsecret\n",
            "1794499329.0 machine NetworkManager[8]: device (wlan0): state change: activated -> disconnected (reason 'carrier-changed')\n",
            "1794499330.0 machine systemd-resolved[44]: DNS query SERVFAIL for account.internal.example\n",
        );
        let (events, contexts) = parse_journal_with_context(logs);
        assert_eq!(events.len(), 4);
        assert!(contexts.iter().any(|d| d.kind == EventKind::LinkDown
            && d.subject.as_deref() == Some("wlan0") && d.signal.contains("reason=2=PREV_AUTH_NOT_VALID")));
        assert!(contexts.iter().any(|d| d.kind == EventKind::ServiceFailure
            && d.subject.as_deref() == Some("sshd.service") && d.signal.contains("exit_status=1/FAILURE")));
        assert!(contexts.iter().any(|d| d.kind == EventKind::DnsProblem
            && d.signal.contains("dns_reply=SERVFAIL")));
        assert!(contexts.iter().all(|d| d.signal.len() <= 160
            && !d.signal.contains("private") && !d.signal.contains("secret")
            && !d.signal.contains("aa:bb") && !d.signal.contains("account.internal")));
        assert_eq!(parse_journal(logs).len(), events.len());
    }

    #[test]
    fn targeted_unit_journal_retains_application_fatal_message_not_unrelated_lines() {
        let general = concat!(
            "1794499328.0 kali systemd[1]: nessusd.service: Main process exited, code=exited, status=1/FAILURE\n",
            "1794499328.1 kali systemd[1]: nessusd.service: Failed with result 'exit-code'.\n",
            "1794499328.2 kali systemd[1]: vboxadd-service.service: Failed with result 'exit-code'.\n"
        );
        let (_, mut contexts) = parse_journal_with_context_and_messages(general, true);
        let unit_journal = concat!(
            "1794499300.0 kali nessus-service[764]: nessus-service [764][INFO] : Nessus started\n",
            "1794499328.0 kali nessus-service[764]: nessus-service [764][ERR] : Exiting: nessusd had too many fatal errors.\n",
            "1794499328.1 kali systemd[1]: nessusd.service: Failed with result 'exit-code'.\n",
            "1794499100.0 kali nessus-service[764]: too old fatal error\n",
            "1794499350.0 kali nessus-service[764]: future fatal error\n"
        );
        append_targeted_unit_journal_messages(&mut contexts, "nessusd.service", unit_journal);
        let nessus = contexts.iter().find(|ctx|
            ctx.subject.as_deref() == Some("nessusd.service")).unwrap();
        assert!(nessus.journal_messages[0].contains("too many fatal errors."));
        assert!(nessus.journal_messages.iter().any(|s| s.contains("Main process exited")));
        assert!(!nessus.journal_messages.iter().any(|s| s.contains("too old")));
        assert!(!nessus.journal_messages.iter().any(|s| s.contains("future fatal")));
        assert_eq!(nessus.journal_messages.len(), 3);
        let other = contexts.iter().find(|ctx|
            ctx.subject.as_deref() == Some("vboxadd-service.service")).unwrap();
        assert_eq!(other.journal_messages.len(), 1);
        let (_, default_contexts) = parse_journal_with_context(general);
        assert!(default_contexts.iter().all(|ctx| ctx.journal_messages.is_empty()));
    }

    #[test]
    fn original_journal_messages_are_opt_in_bounded_and_correlated() {
        let logs = concat!(
            "1794499328.0 machine systemd[1]: app-demo.service: Main process exited, code=exited, status=24 token=LOCAL_SECRET\n",
            "1794499328.2 machine systemd[1]: app-demo.service: Failed with result 'exit-code'.\n",
            "1794499328.4 machine systemd[1]: app-demo.service: Failed with result 'exit-code'.\n",
            "1794499329.0 machine systemd[1]: other.service: Main process exited, code=exited, status=1/FAILURE\n"
        );
        let (_, private) = parse_journal_with_context(logs);
        assert!(private.iter().all(|ctx| ctx.journal_messages.is_empty()));
        let (events, detailed) = parse_journal_with_context_and_messages(logs, true);
        assert_eq!(events.len(), 2);
        let app = detailed.iter().find(|ctx| ctx.subject.as_deref() == Some("app-demo.service")).unwrap();
        assert_eq!(app.journal_messages.len(), 2);
        assert!(app.journal_messages[0].contains("token=LOCAL_SECRET"));
        assert!(app.signal.contains("exit_status=24"));
        assert!(app.signal.contains("systemd_result=exit-code"));
        assert!(detailed.iter().any(|ctx| ctx.subject.as_deref() == Some("other.service")
            && ctx.journal_messages.len() == 1));
    }

    #[test]
    fn unknown_reason_text_is_not_forwarded_as_diagnostic_code() {
        assert_eq!(diagnostic_signal(
            "wlan0: deauthenticated (Reason: 2=SUPERSENSITIVE_TOKEN)",
            EventKind::LinkDown).as_deref(), Some("reason=2"));
        assert_eq!(diagnostic_signal(
            "sshd.service: failed with result 'my-login-secret'",
            EventKind::ServiceFailure), None);
        assert_eq!(diagnostic_signal(
            "sshd.service: main process exited status=1/ACCESS_TOKEN",
            EventKind::ServiceFailure).as_deref(), Some("exit_status=1"));
    }

    #[test]
    fn real_manjaro_journal_classifies_wifi_recovery_usb_and_restart() {
        let logs = concat!(
            "1794499282.0 host kernel: usb 1-5.1: USB disconnect, device number 7\n",
            "1794499303.0 host kernel: usb 1-1: USB disconnect, device number 2\n",
            "1794499303.1 host kernel: usb 1-1.1: USB disconnect, device number 9\n",
            "1794499303.2 host kernel: usb 1-1.2: USB disconnect, device number 4\n",
            "1794499327.0 host kernel: wlan0: deauthenticated from aa:bb:cc:dd:ee:ff (Reason: 2=PREV_AUTH_NOT_VALID)\n",
            "1794499345.0 host kernel: wlan0: associated\n",
            "1794499399.0 host kernel: wlan0: deauthenticating from aa:bb:cc:dd:ee:ff by local choice (Reason: 3=DEAUTH_LEAVING)\n",
            "1794499399.1 host kernel: wlan0: associated\n",
            "1794496601.0 host systemd[1]: lightdm.service: Main process exited, code=exited, status=1/FAILURE\n",
            "1794496601.1 host systemd[1]: lightdm.service: Scheduled restart job, restart counter is at 1.\n",
            "1794499346.0 host kernel: [UFW BLOCK] IN=wlan0 OUT= SRC=192.168.0.20 DST=224.0.0.252 PROTO=UDP DPT=5355\n"
        );
        let events = parse_journal(logs);
        assert_eq!(events.iter().filter(|e| e.kind == EventKind::UsbDisconnect).count(), 4);
        assert_eq!(events.iter().filter(|e| e.kind == EventKind::LinkDown).count(), 1);
        assert_eq!(events.iter().filter(|e| e.kind == EventKind::WifiRoamStart).count(), 1);
        assert_eq!(events.iter().filter(|e| e.kind == EventKind::WifiAssociated).count(), 2);
        assert_eq!(events.iter().filter(|e| e.kind == EventKind::ServiceRestartScheduled).count(), 1);
        assert!(!events.iter().any(|e| e.source.as_deref() == Some("ufw")));
        assert!(events.iter().any(|e| e.kind == EventKind::UsbDisconnect
            && e.subject.as_deref() == Some("1-5.1")));
    }

    #[test]
    fn network_drop_delta_requires_two_valid_monotonic_samples() {
        let before = vec![InterfaceStats {name:"wlan0".into(),operstate:Some("up".into()),
            rx_errors:0,rx_dropped:100,tx_errors:0,tx_dropped:5}];
        let after = vec![InterfaceStats {name:"wlan0".into(),operstate:Some("up".into()),
            rx_errors:0,rx_dropped:112,tx_errors:0,tx_dropped:7}];
        let delta = compare_interface_drops(&before, &after, 2000);
        assert_eq!(delta[0].rx_dropped_delta, 12);
        assert_eq!(delta[0].tx_dropped_delta, 2);
        assert!(compare_interface_drops(&before, &after, 0).is_empty());
        assert!(compare_interface_drops(&after, &before, 2000).is_empty(),
            "reset/decrease is not new packet loss");
    }

    #[test]
    fn journal_sampling_uses_extra_entry_to_detect_actual_truncation() {
        let logs = concat!(
            "1700000000.1 host systemd[1]: example.service: Failed with result 'exit-code'.\n",
            "1700000001.1 host kernel: wlan0: deauthenticated from aa:bb:cc:dd:ee:ff\n",
            "1700000002.1 host app[1]: recovered normally\n",
        );
        let (limited, truncated, count) = limit_journal_sample(logs, 2);
        assert!(truncated);
        assert_eq!(count, 2);
        assert!(!limited.contains("example.service"));
        assert!(limited.contains("wlan0:"));
        assert!(limited.contains("recovered normally"));

        let (exact, truncated, count) = limit_journal_sample(logs, 3);
        assert!(!truncated, "exactly N entries must not be called truncated");
        assert_eq!(count, 3);
        assert_eq!(exact.lines().count(), 3);

        let (none, truncated, count) = limit_journal_sample("-- No entries --\n", 5);
        assert!(!truncated);
        assert_eq!(count, 0);
        assert_eq!(none, "-- No entries --");
    }

    #[test]
    fn local_df_default_skips_remote_mounts_but_full_scan_is_opt_in() {
        assert!(df_args(false, false).contains(&"--local"));
        assert!(df_args(true, false).contains(&"--exclude-type=fuse.rclone"));
        assert_eq!(df_args(false, true), &["-PT"]);
        assert_eq!(df_args(true, true), &["-Pi"]);
        let mounts = "10 1 0:20 / /mnt/onedrive rw - fuse.rclone rclone rw\n\
11 1 0:21 / /home rw - ext4 /dev/sda1 rw\n";
        assert!(has_remote_mounts(mounts));
        assert!(!has_remote_mounts("11 1 0:21 / /home rw - ext4 /dev/sda1 rw\n"));
    }

    #[test]
    fn command_timings_are_recorded_without_raw_output() {
        let mut s = Snapshot::default();
        let result = capture_timed(&mut s, "synthetic test", "printf", &["OK"]).unwrap();
        assert!(result.0);
        assert_eq!(result.1, "OK");
        assert_eq!(s.collector_timings.len(), 1);
        assert_eq!(s.collector_timings[0].name, "synthetic test");
        assert!(s.collector_timings[0].succeeded);
    }

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
    fn network_kernel_interface_is_attributed_to_actual_device() {
        let journal = concat!(
            "1700000010.100 host kernel: wlan0: deauthenticated from 11:22:33:44:55:66\n",
            "1700000011.100 host kernel: e1000e 0000:00:19.0 enp3s0: NIC Link is Down\n"
        );
        let events = parse_journal(journal);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].subject.as_deref(), Some("wlan0"));
        assert_eq!(events[1].subject.as_deref(), Some("enp3s0"));
    }

    #[test]
    fn read_only_root_mount_detected_without_touching_mounts() {
        let rw = "20 1 8:2 / / rw,relatime - ext4 /dev/sda2 rw\n";
        let ro = "20 1 8:2 / / ro,relatime - squashfs /dev/loop0 ro\n";
        assert_eq!(parse_root_mount_read_only(rw), Some(false));
        assert_eq!(parse_root_mount_read_only(ro), Some(true));
        assert_eq!(parse_root_mount_read_only("bad data"), None);
    }

    #[test]
    fn fd_errors_are_classified_without_retaining_raw_log_messages() {
        let events = parse_journal(
            "1700000020.000 host app[55]: open: Too many open files for /secret/config\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, EventKind::FileHandleExhaustion);
        assert_eq!(events[0].subject, None);
        assert!(!format!("{events:?}").contains("/secret/config"));
    }

    #[test]
    fn nameservers() {
        assert_eq!(parse_resolv_conf("# comment\nnameserver 127.0.0.53\nnameserver 1.1.1.1\n"), (2, true));
        assert_eq!(parse_resolv_conf("search example.org\n"), (0, false));
        assert_eq!(parse_resolv_conf("nameserver not-an-ip\n"), (0, false));
        assert_eq!(parse_resolv_conf("nameserver 2001:4860:4860::8888\n"), (1, false));
        assert_eq!(parse_resolv_conf("nameserver fe80::1%wlan0\n"), (1, false));
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
    fn sudo_audit_search_terms_do_not_become_service_or_network_failures() {
        // Reproduces the semantics of an audit record generated when a user
        // searches for failures. Arguments are NOT evidence of such failures.
        let audit = "1794441081.5 host sudo[1234]: admin : TTY=pts/1 ; PWD=/tmp ; USER=root ; COMMAND=/usr/bin/journalctl --grep=Failed to start|Failed with result|network is unreachable\n";
        assert!(parse_journal(audit).is_empty());
        let quoted = "1794441082.5 host sudo[1234]: failed to start was passed as command text\n";
        assert!(parse_journal(quoted).is_empty());
    }

    #[test]
    fn real_systemd_unit_failures_remain_visible_after_audit_filter() {
        let log = "1794441083.5 host systemd[100]: rclone-onedrive.service: Failed with result 'exit-code'.\n";
        let events = parse_journal(log);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, EventKind::ServiceFailure);
        assert_eq!(events[0].subject.as_deref(), Some("rclone-onedrive.service"));
    }

    #[test]
    fn dpkg_audit_states_from_debian13_kernel_failure() {
        let text = "linux-headers-6.12.116+deb13-amd64\tiU \n\
linux-headers-amd64\tiU \n\
linux-image-amd64\tiU \n\
linux-image-6.12.116+deb13-amd64\tiF \n\
bash\tii \n\
residual-config\trc \n\
held-installed\thi \n\
half-installed\tiH \n\
need-reinstall\tiHR\n\
trigger-unit\tit \n\
trigger-await\tiW \n";
        let found = parse_dpkg_status(text);
        assert_eq!(found.len(), 8, "only non-installed / broken states should be retained");
        assert!(found.iter().any(|p| p.name == "linux-image-6.12.116+deb13-amd64" && p.state == "half-configured"));
        assert!(found.iter().any(|p| p.name == "linux-headers-amd64" && p.state == "unpacked"));
        assert!(found.iter().any(|p| p.name == "need-reinstall" && p.reinstall_required));
        assert!(!found.iter().any(|p| p.name == "residual-config" || p.name == "bash" || p.name == "held-installed"));
    }

    #[test]
    fn dpkg_query_ignores_invalid_or_inapplicable_states() {
        let text = "pkg\tii \nremoved\trc \nnotinstalled\tun \nunknown\t?? \nnot-a-record\n";
        assert!(parse_dpkg_status(text).is_empty());
    }

    #[test]
    fn collector_limits_subprocess_runtime_and_output() {
        // External commands are used solely as synthetic test fixtures.
        let timeout = run_capture_with_limits("sleep", &["2"],
            Duration::from_millis(70), 1024).unwrap_err();
        assert_eq!(timeout.kind(), io::ErrorKind::TimedOut);
        let output = run_capture_with_limits("printf", &["123456789"],
            Duration::from_secs(2), 4).unwrap_err();
        assert_eq!(output.kind(), io::ErrorKind::InvalidData);
        let success = run_capture_with_limits("printf", &["OK"],
            Duration::from_secs(2), 10).unwrap();
        assert!(success.0);
        assert_eq!(success.1, "OK");
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
    fn bounded_service_status_parser_accepts_live_lightdm_state() {
        let text = "LoadState=loaded\nActiveState=active\nSubState=running\nResult=success\nNRestarts=1\n";
        let status = parse_service_status("lightdm.service", "system", text).unwrap();
        assert_eq!(status.active_state, "active");
        assert_eq!(status.sub_state, "running");
        assert_eq!(status.result, "success");
        assert_eq!(status.n_restarts, Some(1));
        assert!(parse_service_status("missing.service", "user", "LoadState=not-found\nActiveState=inactive\nSubState=dead\nResult=success\n").is_none());
        assert!(!safe_service_unit_name("--help.service"));
        assert!(!safe_service_unit_name("../../x.service"));
        assert!(!safe_service_unit_name("test.service\nInjected=true"));
        assert!(safe_service_unit_name("systemd-user@1000.service"));
    }

    #[test]
    fn desktop_transient_metadata_and_exit_code_are_scoped_and_private() {
        let raw = "LoadState=loaded\nActiveState=failed\nSubState=failed\nResult=exit-code\nNRestarts=0\nTransient=yes\nFragmentPath=/usr/share/applications/opera.desktop\nExecMainStatus=24\nExecStart=/usr/bin/opera /home/user/private/document.html\n";
        let state = parse_service_status("app-opera@abc.service", "user", raw).unwrap();
        assert_eq!(state.manager, "user");
        assert_eq!(state.transient, Some(true));
        assert!(state.desktop_entry);
        assert_eq!(state.exec_main_status, Some(24));
        assert!(!format!("{state:?}").contains("private"));
        assert!(parse_service_status("app-opera@abc.service", "system",
            "LoadState=not-found\nActiveState=inactive\nSubState=dead\nResult=success\n").is_none());
        let malicious = raw.replace("/usr/share/applications/opera.desktop",
            "/home/user/private.desktop");
        assert!(!parse_service_status("app-opera@abc.service", "user", &malicious)
            .unwrap().desktop_entry);
    }

    #[test]
    fn multiple_systemd_lines_in_same_second_keep_all_safe_diagnostic_codes() {
        let raw = concat!(
            "1794507530.100 host systemd[1265]: app-opera@abc.service: Main process exited, code=exited, status=24/n/a user-secret=ABC\n",
            "1794507530.200 host systemd[1265]: app-opera@abc.service: Failed with result 'exit-code'.\n",
        );
        let (events, contexts) = parse_journal_with_context(raw);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].subject.as_deref(), Some("app-opera@abc.service"));
        let detail = &contexts[0].signal;
        assert!(detail.contains("exit_status=24"), "{detail}");
        assert!(detail.contains("systemd_result=exit-code"), "{detail}");
        assert!(!detail.contains("secret"));
        assert!(!detail.contains("ABC"));
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
