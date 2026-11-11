# WhyDown privacy and data-flow notes

## Core privacy properties

WhyDown v0.1.0 has no telemetry, auto-update, remote API, cloud dependency, external DNS test or outbound network functionality. It reads local OS, networking and service data and may call local `journalctl`, `systemctl` and `df` commands in separate processes without invoking a shell. No background daemon is installed.

The raw text of journal lines is discarded after classification. The reports retain only coarse event types and timestamps, and optional names of local systemd units, network interfaces and filesystem mountpoints. These remaining identifiers **may be sensitive**, especially on shared or business computers.

## Collected data

- OS release and kernel version
- Uptime, if available
- Current existence of IPv4/IPv6 default routes
- Number of configured DNS servers and local DNS-stub status
- Interface names and cumulative error/drop counters
- Failed systemd unit names, when available
- Filesystem mountpoint paths and usage percent
- Time-stamped types of matching journal events (not their message bodies)
- Collection warnings and explanatory findings

## Reports

- stdout reporting is never persisted by WhyDown itself.
- `--output` writes to the specified local file only.
- Avoid uploading unreviewed HTML/JSON reports to public issues or third-party systems.
- Other tools and the user's shell may record output; WhyDown cannot guarantee privacy outside its own process.

## Permissions

Run without root by default. Depending on your system, journal access may require membership in `systemd-journal`, `adm`, or similar groups. Granting those permissions gives broader access to logs than WhyDown alone, so apply least privilege.

## Limits

WhyDown does not protect against tampered local logs, malicious binaries shadowing `journalctl` or `systemctl` in PATH, or compromised operating systems. It is a diagnostic tool and **not a sandbox, IDS, antivirus or verified forensic recorder**.


## Event attribution (v0.3.0)

JSON schema v2 includes optional `subject` (bounded interface/systemd-unit identifier) and `source` (journal program name). Only short ASCII identifiers are retained. Raw journal message bodies and hostnames are not included in event attribution.

These identifiers, mountpoints and systemd units may still expose system information. Review exports before sharing. A current interface `operstate` or default route is not proof of internet connectivity; historic journal incidents do not establish the current state.
