# WhyDown

**Evidence-based, read-only Linux troubleshooting in one command.**

WhyDown inspects common Linux failure signals, groups observations into actionable findings, and reconstructs a short timeline of events. It helps answer **"What failed, and what should I check next?"** without sending your system data to a cloud service.

> [!IMPORTANT]
> **Status: v0.3.2 — early technical release.** WhyDown does **not** prove root causes, repair your system, or replace detailed investigation. Its rule-based journal classification may miss relevant events or produce false positives. Use suggested checks to confirm every hypothesis.

## Highlights

- **Read-only by design:** never restarts services, changes configuration, installs packages, or escalates privileges.
- **Offline:** no outbound network requests, no accounts, no telemetry, no remote AI service.
- **Zero third-party Rust crates:** a minimal, auditable command-line application using only the Rust standard library.
- **Useful initial coverage:** systemd service failures, journal-reported DNS/DHCP/link failures, missing default routes, inactive systemd-resolved stub, filesystem capacity, OOM and storage errors.
- **Time-based correlation and per-interface incident windows:** groups nearby journal events separately for identified network interfaces and flags when link problems precede DNS failures; correlation does not prove causation.
- **Source attribution (v0.3.0):** bounded program, interface or systemd unit identifiers from journal messages; no raw log bodies are retained.
- **Temporal scope:** distinguishes current kernel route/interface snapshots from historical journal observations; live internet connectivity is never probed.
- **Comparable JSON schema v2:** warning counts by scope, network incident subjects and emitting programs for multi-distribution analysis.
- **Service diagnostics (v0.3.1):** separate system/user systemd failed-unit snapshots, plus historical service-message windows that distinguish log entries from independent failures.
- **Audit false-positive filtering (v0.3.2):** sudo/doas/pkexec command audit records are no longer mistaken for failures simply because a grep pattern or command argument contains words such as "Failed to start". Historical service windows now indicate whether they predate the estimated current boot, based on uptime; this is an estimate, not a journal boot ID.
- **Optional disk breakdown:** `--disk-details` uses `du` to list large subdirectories on up to two near-full non-root filesystems. No files are removed.
- **Privacy-aware reports:** stores *classified event types and timestamps*, not raw journal message bodies. Reports can still contain local mountpoints, network interface and systemd unit names.
- **Three output formats:** human-readable terminal output, machine-readable JSON, self-contained HTML.
- **Demo mode:** show a fictional incident without reading any local system information.
- **Linux x86_64 / ARM64:** release archives for GNU/glibc and musl targets. Linux distributions differ; successful CI compilation does not guarantee every distribution has been tested.

## Quick start

### Requirements

- Linux (for actual system diagnostics; demo mode also works elsewhere)
- Rust 1.70 or newer and Cargo (build-time only)
- `journalctl` and `systemctl` for journal/systemd checks (optional; missing commands are reported)
- `df` for filesystem checks (optional)

### Download a prebuilt Linux binary (no Rust required)

Download the [latest GitHub Release](https://github.com/hamdyaea/WhyDown/releases) for your CPU architecture and preferred C runtime.

| Architecture | GNU / glibc | musl |
| --- | --- | --- |
| x86_64 | `x86_64-unknown-linux-gnu` | `x86_64-unknown-linux-musl` |
| ARM64 / aarch64 | `aarch64-unknown-linux-gnu` | `aarch64-unknown-linux-musl` |

Example for x86_64 Linux (including Manjaro):

```bash
VERSION=v0.3.2
TARGET=x86_64-unknown-linux-gnu
ARCHIVE="whydown-${VERSION}-${TARGET}.tar.gz"
BASE="https://github.com/hamdyaea/WhyDown/releases/download/${VERSION}"

curl -fL -O "${BASE}/${ARCHIVE}"
curl -fL -O "${BASE}/${ARCHIVE}.sha256"
sha256sum -c "${ARCHIVE}.sha256"
tar -xzf "${ARCHIVE}"
./whydown --version
./whydown demo
```

The SHA-256 checksum helps verify download integrity; it is **not a cryptographic signature**. Archives also contain this README and the GPLv3 license. No root privileges are required to extract or run the demo.

### Build from source

```bash
git clone https://github.com/hamdyaea/WhyDown.git
cd WhyDown
cargo build --release
./target/release/whydown --help
```

To install it in your Cargo binary directory:

```bash
cargo install --path .
whydown diagnose
```

Or, without installing anything:

```bash
cargo run --release -- diagnose --focus network --since 2h
```

### Updating an existing source checkout

```bash
cd ~/WhyDown
git pull --ff-only
cargo build --release
./target/release/whydown --version
```

**One-time migration from v0.1.0/v0.1.1:** builds of those versions generated a local, untracked `Cargo.lock`; v0.2.0 began tracking this file for reproducible release builds. If `git pull` stops with **“untracked working tree file Cargo.lock would be overwritten”**, back up the local file before retrying:

```bash
cd ~/WhyDown
mv Cargo.lock ../WhyDown-Cargo.lock.backup
git pull --ff-only
cargo build --release
./target/release/whydown --version   # expects whydown 0.3.2
```

Do **not** use `git reset --hard`, `git clean -fd`, or `git stash -u` as a generic workaround: they can discard unrelated local work. The `mv` command above preserves your local lockfile. If you have other modifications, inspect `git status` before updating.

### First try: safe demo

```bash
whydown demo
whydown demo --format json --output example.json
whydown demo --format html --output example.html
```

The demo shows **fictional** Wi-Fi disconnect, DHCP failure, DNS error and a failed service. It does not access system logs, network sockets or external services.

## Usage

```text
whydown [diagnose] [OPTIONS]
whydown demo [OPTIONS]

--focus VALUE     all | network | services | system     (default: all)
--since VALUE     e.g. 30m, 24h, 7d                (default: 24h)
--max-events N    newest journal lines read         (default: 500)
--disk-details    inspect directories on near-full filesystems (optional; can take time)
--format VALUE    text | json | html                (default: text)
--output FILE     save to a new file
--overwrite       allow replacing an existing file
-h, --help        display help
-V, --version     display version
```

Examples:

```bash
# Diagnostic overview for the past day
whydown diagnose

# Wi-Fi, DHCP, routes and DNS
whydown diagnose --focus network --since 3h

# Services and related journal messages
whydown diagnose --focus services --since 7d

# Disk usage, OOM and storage errors
whydown diagnose --focus system

# Optional deeper analysis of near-full non-root partitions (read-only; potentially slow)
whydown diagnose --focus system --disk-details

# More journal coverage if the system is very busy
whydown diagnose --max-events 5000 --since 24h

# Integration with jq (if installed)
whydown diagnose --format json | jq '.findings[] | {id, title, confidence}'

# Export a self-contained HTML report
whydown diagnose --format html --output diagnostics.html

# Export schema-v2 JSON for comparisons
whydown diagnose --format json --output report.json
```

**Important:** `--since` limits the time range, but `--max-events` means that only the newest N journal entries in that range are inspected. On busy hosts, increase the limit. This is **not continuous monitoring** and does not guarantee that every relevant event is captured.

### Example terminal output

```text
WhyDown 0.3.2 [DEMO]
====================================================================
Generated : 2026-11-11T20:00:00Z
System    : Demo Linux 1.0 (fictional)
Kernel    : 6.12.0-demo
Focus     : all
Journal   : last 1440 minute(s), newest entries only
Findings  : ...

1. [WARN] Network link disconnections were reported
   ID: LINK_DISCONNECTS | Confidence: observed
   ...

2. [WARN] DHCP failures were reported
   ID: DHCP_FAILURES | Confidence: observed
   ...

3. [WARN] DNS lookup failures were reported
   ID: DNS_FAILURES | Confidence: observed
   ...

4. [INFO] A link interruption preceded a DNS failure
   ID: LINK_BEFORE_DNS | Confidence: moderate
   ...
```

This output is **illustrative**. Timestamps and the exact number/order of findings depend on the run. Use `whydown demo` for the actual output.

## What WhyDown checks

| Area | Data source | Current behavior |
| --- | --- | --- |
| Linux identity | `/etc/os-release`, `/proc/sys/kernel/osrelease` | Identifies the detected OS/kernel |
| IPv4/IPv6 default route | `/proc/net/route`, `/proc/net/ipv6_route` | Detects absent default routes, considering IPv6-only hosts |
| DNS configuration | `/etc/resolv.conf` | Checks whether nameservers are configured |
| systemd-resolved stub | `/etc/resolv.conf`, `systemctl is-active` | Warns if local stub is configured but service appears inactive |
| Link, DHCP and DNS events | `journalctl --output=short-unix` | Classifies matching messages and shows timestamps; generic "link down" messages require network context so storage-controller messages are not treated as network failures |
| Journal correlation | Classified event chronology | Reports a link/DHCP event preceding a DNS error within 120 seconds |
| Network incident windows | Classified network journal events | Groups events separated by at most 120 seconds for the same identified interface; no claim of active outages |
| Current interface state | `/sys/class/net/*/operstate` | Read-only kernel state; no internet connectivity check |
| Event attribution | Bounded `journalctl` identifiers | Optional interface, systemd unit and program names; raw messages discarded |
| Failed services | `systemctl list-units --failed`, `systemctl --user list-units --failed` | Separately checks system and invoking-user managers; unavailable checks are marked unknown, not zero |
| Historical service messages | Classified `journalctl` entries | Groups messages within five seconds into time windows; not a count of independently failed services |
| Service journal events | `journalctl` | Classifies failure-like system messages |
| Disk utilization | `df -PT` | Warns at >=90% for writable-capable filesystem types; excludes full-by-design compressed read-only images (SquashFS, EROFS, ISO 9660, CramFS) |
| Optional disk details | `du -x -k --max-depth=2` | Read-only breakdown for up to two eligible near-full non-root mounts; may be slow or incomplete |
| Memory pressure | `journalctl` | Detects OOM-related messages |
| Storage errors | `journalctl` | Detects common kernel/application storage error strings |
| Interface error counters | `/proc/net/dev` | Reports cumulative RX/TX error counters (not rates) |

### How confident are the diagnoses?

- **Observed:** a specific state or classified message was present in collected evidence. Does not mean the journal classifier understood the context perfectly.
- **Moderate:** a bounded chronological correlation was detected. It may suggest the order of investigation but **does not prove causation**.
- **Limited:** missing or inconclusive evidence prevents a stronger conclusion.

`NO_CLASSIFIED_ANOMALY` does **not** mean a healthy system. It means that no available observation matched the current detection rules.

## Cross-distribution testing (v0.3.0)

WhyDown provides a common report format for comparing Manjaro, Raspberry Pi OS, Debian, CachyOS, Ubuntu Server and Kali Purple. **Manjaro has been tested interactively through v0.3.1; a live v0.3.2 retest and testing on the other five environments remain planned.** See the [validation matrix](docs/TEST_MATRIX.md).

```bash
whydown diagnose --since 24h --max-events 5000 --format json --output report.json
python3 -m json.tool report.json >/dev/null
```

**JSON schema:** `schema_version` remains `2` in v0.3.2 (the breaking transition from v1 occurred in v0.3.0). New optional fields include `system.failed_user_units`, `system.system_units_checked`, and `service_event_windows`. Reports include `summary`, `findings[].scope`, optional `source`/`subject`, kernel interface `operstate`, and per-interface incident subjects. See [JSON report schema v2](docs/REPORT_SCHEMA.md). `operstate=up` does not establish internet reachability.

## Security and privacy

- **Never run as root unless you specifically need privileged journal access.** Ordinary users can perform many checks, but journal access depends on distribution and group permissions.
- There are **no shell commands executed through `sh -c` or `bash -c`**. External tools are called using fixed command names and separate arguments.
- The tool **does not perform network requests** or install packages.
- Exports omit raw journal bodies but may expose bounded emitting-program, systemd-unit and network-interface identifiers, mountpoints and Linux versions. Review reports before sharing.
- `--output` refuses to overwrite a file by default. Use `--overwrite` only when you intend to replace the target.
- A compromised local system can falsify log messages and system state. WhyDown is a diagnostic assistant, not a trust boundary or intrusion detection system.
- The initial journal classifier uses text patterns, not a full systemd journal schema or per-distribution parser. Unexpected language, formatting or application-specific logs may be missed.

See [docs/PRIVACY.md](docs/PRIVACY.md) and [SECURITY.md](SECURITY.md).

## Architecture

```text
src/
├── main.rs        CLI entry point and file output
├── cli.rs         Argument parsing and validation
├── collect.rs     Linux snapshot + read-only journal/systemd/df collectors
├── engine.rs      Deterministic, explainable diagnostic rules
├── model.rs       Findings, confidence, events and system snapshot types
├── render.rs      Text/JSON/standalone-HTML reporting
├── demo.rs        Fictional incident fixture
├── time.rs        UTC timestamp formatting
└── lib.rs         Library entry point
```

The source is deliberately small and dependency-free. Each new diagnostic should carry a **precise evidence requirement**, a **confidence label**, a **verification step**, and a **regression test**.

## Development

```bash
cargo fmt --all
cargo test --all-targets
cargo clippy --all-targets
cargo build --release
```

GitHub Actions runs Rust tests, Clippy, a release build and demo smoke tests. The [release workflow](.github/workflows/release.yml) publishes four Linux archives with SHA-256 checksums when the version in `Cargo.toml` is new. **CI success does not replace real-world distribution testing.**

## Version history and roadmap

- **v0.1.x (released):** initial diagnostics, journal classification, exports, and read-only design; fix read-only-image disk false positives.
- **v0.2.0 (released):** restart-aware network classification, historical incident windows, optional disk breakdown, Linux release archives and checksums.
- **v0.2.1 (released):** stop mistaking the normal NetworkManager DHCP transaction timeout budget for a DHCP failure.
- **v0.2.2 (released):** require network interface or component context for generic "link down" messages, avoiding SATA-storage false positives at boot.
- **v0.3.0 (released):** per-interface incident grouping, bounded event attribution, current-versus-historical output, JSON schema v2.
- **v0.3.1 (released):** system and user systemd failed-unit snapshots; temporal service-message groups without assuming one message equals one service outage.
- **v0.3.2 (released):** filter privilege-wrapper command audit false positives; show approximate previous/current boot relation on historical service windows.
- **v0.4 (planned):** optional terminal UI, broader real-world distribution fixtures, safe evidence bundles and configurable diagnostic rules.
- **Future:** optional local model explanations and broader Linux diagnosis, *never at the expense of verifiable evidence*.

Features not implemented in v0.3.2: continuous monitoring, live network health checks, predictive failures, auto-repair, system-wide event tracing, package history correlation, privileged packet capture, external DNS probing, and cloud dashboards.

## Contributing

Contributions are welcome: reproducible bug reports, sanitized fixtures, new detection rules, documentation and portable build tooling. See [CONTRIBUTING.md](CONTRIBUTING.md).

Please **do not upload unredacted logs** in public issues. Include Linux distribution, kernel, relevant command output with private data removed, and a clear reproduction recipe.

## License

**GNU General Public License v3.0 only (GPL-3.0-only).** See [LICENSE](LICENSE).

Copyright © 2026 Hamdy Abou El Anein and contributors.

WhyDown is provided **without warranty**, including any implied warranty of fitness for a particular purpose. See the license for details.
