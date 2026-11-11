# WhyDown

**Evidence-based, read-only Linux troubleshooting in one command.**

WhyDown inspects common Linux failure signals, groups observations into actionable findings, and reconstructs a short timeline of events. It helps answer **"What failed, and what should I check next?"** without sending your system data to a cloud service.

> [!IMPORTANT]
> **Status: v0.1.0 — early technical MVP.** WhyDown does **not** prove root causes, repair your system, or replace detailed investigation. Its rule-based journal classification may miss relevant events or produce false positives. Use suggested checks to confirm every hypothesis.

## Highlights

- **Read-only by design:** never restarts services, changes configuration, installs packages, or escalates privileges.
- **Offline:** no outbound network requests, no accounts, no telemetry, no remote AI service.
- **Zero third-party Rust crates:** a minimal, auditable command-line application using only the Rust standard library.
- **Useful initial coverage:** systemd service failures, journal-reported DNS/DHCP/link failures, missing default routes, inactive systemd-resolved stub, filesystem capacity, OOM and storage errors.
- **Time-based correlation:** flags when a network link problem was logged shortly before a DNS failure, explicitly distinguishing correlation from causation.
- **Privacy-aware reports:** stores *classified event types and timestamps*, not raw journal message bodies. Reports can still contain local mountpoints, network interface and systemd unit names.
- **Three output formats:** human-readable terminal output, machine-readable JSON, self-contained HTML.
- **Demo mode:** show a fictional incident without reading any local system information.
- **Linux x86_64 / ARM64:** build from source on Debian, Ubuntu, Fedora, Arch/Manjaro and Raspberry Pi OS. Other distributions may work but are not yet verified.

## Quick start

### Requirements

- Linux (for actual system diagnostics; demo mode also works elsewhere)
- Rust 1.70 or newer and Cargo (build-time only)
- `journalctl` and `systemctl` for journal/systemd checks (optional; missing commands are reported)
- `df` for filesystem checks (optional)

### Build and install

```bash
git clone https://github.com/hamdyaea/WhyDown.git
cd WhyDown
cargo build --release
./target/release/whydown --help
```

**The repository URL above is the proposed publication destination, not a claim that it already exists.** Until you publish it, build from the supplied source archive.

To install it in your Cargo binary directory:

```bash
cargo install --path .
whydown diagnose
```

Or, without installing anything:

```bash
cargo run --release -- diagnose --focus network --since 2h
```

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

# More journal coverage if the system is very busy
whydown diagnose --max-events 5000 --since 24h

# Integration with jq (if installed)
whydown diagnose --format json | jq '.findings[] | {id, title, confidence}'

# Export a self-contained HTML report
whydown diagnose --format html --output diagnostics.html
```

**Important:** `--since` limits the time range, but `--max-events` means that only the newest N journal entries in that range are inspected. On busy hosts, increase the limit. This is **not continuous monitoring** and does not guarantee that every relevant event is captured.

### Example terminal output

```text
WhyDown 0.1.0 [DEMO]
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

## What the MVP checks

| Area | Data source | Current behavior |
| --- | --- | --- |
| Linux identity | `/etc/os-release`, `/proc/sys/kernel/osrelease` | Identifies the detected OS/kernel |
| IPv4/IPv6 default route | `/proc/net/route`, `/proc/net/ipv6_route` | Detects absent default routes, considering IPv6-only hosts |
| DNS configuration | `/etc/resolv.conf` | Checks whether nameservers are configured |
| systemd-resolved stub | `/etc/resolv.conf`, `systemctl is-active` | Warns if local stub is configured but service appears inactive |
| Link, DHCP and DNS events | `journalctl --output=short-unix` | Classifies matching messages and shows timestamps |
| Journal correlation | Classified event chronology | Reports a link/DHCP event preceding a DNS error within 120 seconds |
| Failed services | `systemctl list-units --failed` | Lists failed service/unit identifiers |
| Service journal events | `journalctl` | Classifies failure-like system messages |
| Disk utilization | `df -P` | Warns at >=90% reported usage |
| Memory pressure | `journalctl` | Detects OOM-related messages |
| Storage errors | `journalctl` | Detects common kernel/application storage error strings |
| Interface error counters | `/proc/net/dev` | Reports cumulative RX/TX error counters (not rates) |

### How confident are the diagnoses?

- **Observed:** a specific state or classified message was present in collected evidence. Does not mean the journal classifier understood the context perfectly.
- **Moderate:** a bounded chronological correlation was detected. It may suggest the order of investigation but **does not prove causation**.
- **Limited:** missing or inconclusive evidence prevents a stronger conclusion.

`NO_CLASSIFIED_ANOMALY` does **not** mean a healthy system. It means that no available observation matched the current detection rules.

## Security and privacy

- **Never run as root unless you specifically need privileged journal access.** Ordinary users can perform many checks, but journal access depends on distribution and group permissions.
- There are **no shell commands executed through `sh -c` or `bash -c`**. External tools are called using fixed command names and separate arguments.
- The tool **does not perform network requests** or install packages.
- Exports intentionally omit raw journal bodies. They can still expose service names, interfaces, local mountpoints and Linux version. Review reports before sharing.
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

The GitHub Actions workflow runs tests and a release build on Ubuntu Linux. This project is a starting point; **the maintainer must verify the behavior on each claimed distribution before publishing platform support as tested**.

## Planned milestones

- **v0.1.x:** integration test fixtures for multiple distros, better journal limitations/reporting, packaging and reproducible builds.
- **v0.2:** configurable diagnostic rule library, richer service attribution, networking timeline, optional sampling to detect changing interface counters.
- **v0.3:** curses-style TUI, safe evidence bundle export and pluggable distribution-specific collectors.
- **Future:** optional local model explanations and broader Linux diagnosis, *never at the expense of verifiable evidence*.

Features not implemented in v0.1: continuous monitoring, predictive failures, auto-repair, system-wide event tracing, package history correlation, privileged packet capture, external DNS probing, cloud dashboards.

## Contributing

Contributions are welcome: reproducible bug reports, sanitized fixtures, new detection rules, documentation and portable build tooling. See [CONTRIBUTING.md](CONTRIBUTING.md).

Please **do not upload unredacted logs** in public issues. Include Linux distribution, kernel, relevant command output with private data removed, and a clear reproduction recipe.

## License

**GNU General Public License v3.0 only (GPL-3.0-only).** See [LICENSE](LICENSE).

Copyright © 2026 Hamdy Abou El Anein and contributors.

WhyDown is provided **without warranty**, including any implied warranty of fitness for a particular purpose. See the license for details.
