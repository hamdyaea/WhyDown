# WhyDown

**Evidence-based, read-only Linux troubleshooting in one command.**

WhyDown inspects common Linux failure signals, groups observations into actionable findings, and reconstructs a short timeline of events. It helps answer **"What failed, and what should I check next?"** without sending your system data to a cloud service.

> [!IMPORTANT]
> **Status: v0.10.0 — early technical release.** WhyDown does **not** prove root causes, repair your system, or replace detailed investigation. Its rule-based journal classification may miss relevant events or produce false positives. Use suggested checks to confirm every hypothesis.

## Highlights

- **Read-only by design:** diagnostics never restart services, change configuration, install packages, or escalate privileges. An optional `--desktop` installer flag creates a user-owned application shortcut only with explicit consent.
- **Modern Qt6 desktop GUI (v0.10.0):** `whydown gui` opens a polished offline PyQt6 application with a prominent **Run diagnostics** button. Focus defaults to **all** (recommended); findings and evidence appear automatically after collection. Includes timeline, coverage, and an explicit private JSON export. The optional Python 3 + PyQt6 dependency is only needed for GUI mode. See [GUI guide](docs/GUI.md).
- **Guided terminal interface (v0.9.0 UX refinement):** run `whydown tui` for a number-and-Enter evidence browser with focus/lookback configuration, incident timeline, findings, current-unit snapshots, limitations and safe export guidance. `whydown tui --demo` does not read host data. Requires a real terminal; see [TUI guide](docs/TUI.md).
- **Offline:** no outbound network requests, no accounts, no telemetry, no remote AI service.
- **Zero third-party Rust crates:** a minimal, auditable command-line application using only the Rust standard library.
- **Useful initial coverage:** systemd service failures, journal-reported DNS/DHCP/link failures, missing default routes, inactive systemd-resolved stub, filesystem capacity, OOM/storage errors, and incomplete dpkg package states on Debian-family distributions.
- **Time-based correlation and per-interface incident windows:** groups nearby journal events separately for identified network interfaces and flags when link problems precede DNS failures; correlation does not prove causation.
- **Source attribution (v0.3.0):** bounded program, interface or systemd unit identifiers from journal messages; no raw log bodies are retained.
- **Temporal scope:** distinguishes current kernel route/interface snapshots from historical journal observations; live internet connectivity is never probed.
- **Comparable JSON schema v2:** warning counts by scope, network incident subjects and emitting programs for multi-distribution analysis.
- **Service diagnostics (v0.3.1):** separate system/user systemd failed-unit snapshots, plus historical service-message windows that distinguish log entries from independent failures.
- **Targeted current systemd unit state (v0.7.1):** for up to four system-manager services with actual historical failure journal messages, read `LoadState`, `ActiveState`, `SubState`, `Result` and `NRestarts` using bounded `systemctl show` calls. A unit reported active is *not* proof of application readiness, and user units are not included in these system-manager queries. No service restart or repair is performed.
- **Incident recovery and hardware context (v0.7.0):** detects Wi-Fi reassociation near a link loss, separates local-choice roaming from hard disconnects, observes systemd restart scheduling without claiming verified recovery, and groups simultaneous USB detachments into a single possible dock/hub incident. Wi-Fi records in the same whole-second journal bucket are not guaranteed to have occurred in the displayed order.
- **USB grouping precision (v0.7.1):** require at least four disconnected topology paths within one USB root-port branch during the same 30-second window. This avoids merging unrelated root-port detachments, while retaining the caveat that a whole subtree may represent just one hub unplug/reset.
- **Measured in-run packet-drop changes (v0.7.0):** captures two read-only /proc/net/dev samples; shows deltas and sample duration separately from cumulative counters. Counter resets or absent comparisons are unknown, not new drops.
- **Earlier disk capacity advisory (v0.7.0):** 85–89% utilization yields informational DISK_CAPACITY_WATCH; the existing >=90% warning remains unchanged.
- **Bounded journal coverage made explicit (v0.6.1):** default tail increased from 500 to 5,000 entries; an N+1 sentinel detects actual truncation, and incomplete journal coverage is marked in the diagnostic findings and text/JSON/HTML reports rather than producing a misleading `NO_CLASSIFIED_ANOMALY`.
- **Accurate evidence and incident attribution (v0.6.0):** fix detection of interface names ending in a colon in kernel journal messages; validate literal DNS nameserver addresses; clearly distinguish missing core evidence from a completed diagnostic with no matching anomalies; expose root mount read-only status and explicit file-descriptor exhaustion events.
- **Clean progress/report boundaries (v0.5.3):** stops the live progress renderer before writing stdout or announcing a saved file, then emits the final 100% line only on success. Prevents merged terminal text such as `elapsed 1sWhyDown 0.5.2`; interrupted writes are not marked successful.
- **Fast and inspectable filesystem collection (v0.5.2):** ignore network mounts and known network-backed FUSE types during default disk/inode checks, including rclone/OneDrive. Use `--include-remote-mounts` to explicitly include them (can stall); keep valid partial `df` output and record actual elapsed time for each subprocess in the report.
- **Live diagnostic progress (v0.5.1):** an animated visual progress bar on interactive terminals, stage count, current collector name and elapsed time; default auto/TTY detection, `--progress` force and `--no-progress` suppression. Always uses standard error, never stdout or report JSON.
- **Portable Linux host health (v0.5.0):** read-only memory headroom from `/proc/meminfo`, sustained 60-second pressure from `/proc/pressure/`, kernel file-handle capacity, inode utilization via `df -Pi`, and Linux MD RAID active-member counts. Only strong evidence produces warnings; optional sensors remain unknown when unavailable.
- **Native Arch/Manjaro/CachyOS package database check (v0.4.1):** run the read-only `pacman -Dk` local database consistency check when a pacman database exists; show a limited-confidence warning for nonzero results. Not an installed-file integrity or security verification.
- **Debian package integrity (v0.3.6):** read-only `dpkg-query` status check detects unpacked, half-configured, half-installed, trigger-awaiting/pending or reinstall-required packages independently of systemd. Does not run apt/dpkg repairs or claim to diagnose why a maintainer script failed.
- **Audit false-positive filtering (v0.3.2):** sudo/doas/pkexec command audit records are no longer mistaken for failures simply because a grep pattern or command argument contains words such as "Failed to start". Historical service windows now indicate whether they predate the estimated current boot, based on uptime; this is an estimate, not a journal boot ID.
- **Optional disk breakdown:** `--disk-details` uses `du` to list large subdirectories on up to two near-full non-root filesystems. No files are removed.
- **Privacy-aware reports:** stores *classified event types and timestamps*, not raw journal message bodies. Reports can still contain local mountpoints, network interface and systemd unit names.
- **Three output formats:** human-readable terminal output, machine-readable JSON, self-contained HTML.
- **Bounded collectors (v0.4.0):** subprocesses run with timeouts and caps on stdout size; a failed or oversized query is reported as a collection limitation, not silently treated as a healthy subsystem.
- **Boot-scoped investigation (v0.4.0):** `--current-boot` uses `journalctl --boot=0` so old journal errors need not pollute the present-boot report. Current systemd/dpkg snapshots are still checked independently of journal scope.
- **Monitoring and safe reports (v0.4.0):** `--exit-on-warning` returns code 1 only when a warning is found; by default findings do not change exit status. Exports use private permissions (0600 on Unix); `--overwrite` stages and atomically replaces existing files.
- **Demo mode:** show a fictional incident without reading any local system information.
- **Linux x86_64 / ARM64:** GNU/glibc and musl release archives, plus a guided non-root binary installer that does not need Rust or Cargo. Linux distributions still require real-world testing.

## Quick start

### Requirements

- Linux (for actual system diagnostics; demo mode also works elsewhere)
- Rust 1.70 or newer and Cargo (**only if compiling from source**; not required for the prebuilt binary)
- `journalctl` and `systemctl` for journal/systemd checks (optional; missing commands are reported)
- `df` for filesystem checks (optional)

### Recommended: guided installer (no Rust/Cargo required)

On Linux x86_64 or Raspberry Pi OS 64-bit / Debian ARM64, download the installer, **review it** and run it as your normal user (not with sudo). Static musl is selected by default:

```bash
curl -fL https://raw.githubusercontent.com/hamdyaea/WhyDown/v0.10.0/scripts/install.sh -o whydown-install.sh
less whydown-install.sh
sh whydown-install.sh
~/.local/bin/whydown --version
~/.local/bin/whydown demo
```

If you have already cloned the repository:

```bash
cd ~/WhyDown
git pull --ff-only
sh scripts/install.sh --dry-run       # inspect target, binary URL, destination
sh scripts/install.sh                 # verify SHA-256 and install under ~/.local/bin
```

**Compatibility fix (v0.3.5):** the installer now selects a **statically linked musl binary by default**, even on glibc-based Debian, Ubuntu and Raspberry Pi OS. The v0.3.4 GNU/ARM64 binary was built on Ubuntu 24.04 and required `GLIBC_2.39`, which Debian 12 / Raspberry Pi OS cannot provide (glibc 2.36). Passing SHA-256 verification demonstrates download integrity, **not runtime compatibility**.

The installer downloads and checks the archive, then runs `--version` **before replacing an existing WhyDown executable**. If that fails, the existing installation is preserved. It stages an executable on the same filesystem and atomically renames it after verification. No Rust/Cargo, root privileges or service changes are needed. The installer downloads from GitHub; WhyDown's diagnostics remain offline. Checksums do **not** constitute publisher signatures. Use `--libc gnu` only when the glibc requirements of that release are satisfied, `--libc musl` to force static musl, and `--prefix /absolute/path` to change the installation directory.

### Verified Raspberry Pi OS ARM64 installation (Debian 12)

**Manually verified on 2026-11-12** on Raspberry Pi OS 64-bit, Debian GNU/Linux 12 (bookworm), kernel `6.12.115+rpt-rpi-v8`: v0.3.5 correctly selected `aarch64-unknown-linux-musl`, downloaded its release, verified the SHA-256 checksum (`OK`), installed `~/.local/bin/whydown`, displayed `whydown 0.3.5` and executed `--focus services` successfully. The installer used a **precompiled executable rather than Cargo**, although Rust was already present on this machine.

```bash
cd ~/WhyDown
git pull --ff-only
sh scripts/install.sh --print-target
sh scripts/install.sh
~/.local/bin/whydown --version
~/.local/bin/whydown diagnose --focus services --since 24h --max-events 5000
```

The service diagnostic reported two failed system units and one failed user unit. WhyDown did not modify or restart these services.

### Raspberry Pi OS / Debian 12: GLIBC_2.39 not found

The v0.3.4 GNU/ARM64 release was linked against glibc 2.39, which is newer than Debian 12's glibc 2.36. **Do not upgrade the Raspberry Pi's system glibc solely to run WhyDown.**

```bash
cd ~/WhyDown
git pull --ff-only
sh scripts/install.sh --print-target   # expected: aarch64-unknown-linux-musl
sh scripts/install.sh --dry-run
sh scripts/install.sh                  # verifies execution before replacement
~/.local/bin/whydown --version
```

This can replace a previously installed incompatible GNU build with the static-musl build. If Rust/Cargo is available, `sh scripts/install.sh --from-source` is an alternative. Manually downloaded `aarch64-unknown-linux-musl` assets are another option. Static builds still require a compatible CPU and Linux kernel.

### Manual download of prebuilt Linux binary (no Rust required)

Download the [latest GitHub Release](https://github.com/hamdyaea/WhyDown/releases) for your CPU architecture and preferred C runtime.

| Architecture | GNU / glibc | musl |
| --- | --- | --- |
| x86_64 | `x86_64-unknown-linux-gnu` | `x86_64-unknown-linux-musl` |
| ARM64 / aarch64 | `aarch64-unknown-linux-gnu` | `aarch64-unknown-linux-musl` |

Example for x86_64 Linux (including Manjaro):

```bash
VERSION=v0.7.0
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

### Build from source (only if Rust and Cargo are wanted)

Rust and Cargo are **not** required to run WhyDown. For a source build on Debian 12, Raspberry Pi OS or Ubuntu, install system build tools:

```bash
sudo apt update
sudo apt install build-essential curl git ca-certificates
```

To install a sufficiently recent Rust toolchain and Cargo, follow the official [rustup instructions](https://rustup.rs/). Some Debian releases include an older rustc in APT than this project's minimum 1.70. After rustup installation or an update:

```bash
. "$HOME/.cargo/env"
rustc --version
cargo --version
cc --version
```

The rustup message `Rust is installed now. Great!` confirms success. `Updating existing toolchain` is informational; `cargo:rerun-if-env-changed=...` and `CC_... = None` are normal build-script diagnostics, **not errors by themselves**. If `cargo` is missing after rustup, reopen the shell or source `$HOME/.cargo/env`; look for the actual `error:` line if a build fails.

For a cloned WhyDown repository, check prerequisites and optionally build/install locally without downloading a binary:

```bash
sh scripts/install.sh --check-build-prereqs
sh scripts/install.sh --from-source
```

Or compile manually:

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
./target/release/whydown --version   # expects the version recorded in Cargo.toml
```

Do **not** use `git reset --hard`, `git clean -fd`, or `git stash -u` as a generic workaround: they can discard unrelated local work. The `mv` command above preserves your local lockfile. If you have other modifications, inspect `git status` before updating.

### First try: safe demo

```bash
whydown demo
whydown demo --format json --output example.json
whydown demo --format html --output example.html
```

The demo shows **fictional** Wi-Fi disconnect, DHCP failure, DNS error and a failed service. It does not access system logs, network sockets or external services.

### Modern Qt6 desktop GUI (v0.10.0) — recommended on a Linux desktop

```bash
# Start a real local analysis using a window and one button
~/.local/bin/whydown gui

# Explore the GUI using fictional data only, no host inspection
~/.local/bin/whydown gui --demo

# Optional: create a clickable user application-menu entry while installing
sh scripts/install.sh --desktop

# Recheck GUI dependencies only, with no download or installation
sh scripts/install.sh --check-gui-prereqs
```

**How it works:** select the scope if desired (**all** is preselected),
click **Run diagnostics**, then inspect the automatically populated
**Findings & evidence** tab. The new Qt6 interface has a dark navigation
sidebar, readable summary cards and resizable results. Importantly,
**you can now drag the mouse across an arbitrary passage to highlight it**
in an enabled, read-only Qt text pane. Use **Ctrl+C**, the native right-click
menu or **Copy selection** to copy *only that passage*. Timeline entries are
copyable as rows; an empty selection never silently copies the whole report.

The diagnostic engine remains offline and read-only. JSON exports require
an explicit user action. The GUI requires Python 3 + **PyQt6**, available as
`python-pyqt6` on Manjaro/Arch and `python3-pyqt6` on Debian/Ubuntu.
The installer checks `from PyQt6.QtWidgets import QApplication` automatically,
prints the distro-specific command if missing and never installs packages
itself. CLI and TUI run without Qt. See the [Qt6 GUI guide](docs/GUI.md).

### Guided terminal browser (improved in v0.9.0)

```bash
cargo run -- tui --demo
cargo run -- tui --focus all --since 4h --current-boot
```

**Type a menu number and press Enter.** The dashboard now explicitly says **START HERE: 1 = analyze** and immediately shows a concise list of findings after analysis. Use **6** only for detailed explanations and evidence, 7 for events or 8 for coverage. On a details page, Enter returns or a number 1–9 opens that next action. Browsing evidence never automatically saves files. TUI mode requires interactive stdin/stdout and rejects report-export flags to protect machine-readable output. See [the TUI guide](docs/TUI.md) for keys, privacy and troubleshooting.

## Usage

```text
whydown [diagnose] [OPTIONS]
whydown demo [OPTIONS]
whydown tui [OPTIONS]
whydown tui --demo
whydown gui [OPTIONS]
whydown gui --demo

--focus VALUE     all | network | services | system     (default: all)
--since VALUE     e.g. 30m, 24h, 7d                (default: 24h)
--max-events N    newest journal lines read         (default: 5000; max: 10000)
--current-boot    restrict journal evidence to the current boot
--exit-on-warning return exit code 1 when a warning is found (opt-in)
--progress        show progress on stderr even if it is not a terminal
--no-progress     disable progress (default: auto, only when stderr is a terminal)
--disk-details    inspect directories on near-full filesystems (optional; can take time)
--include-remote-mounts  include remote/FUSE mounts in df checks (slower; opt-in)
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

# Modern Qt6 desktop GUI (requires Python 3 and PyQt6 on a graphical desktop)
whydown gui

# Guided TUI for SSH or terminals (number + Enter)
whydown tui --since 4h --current-boot

# Wi-Fi, DHCP, routes and DNS
whydown diagnose --focus network --since 3h

# Services and related journal messages
whydown diagnose --focus services --since 7d

# Ignore events from previous Linux boots when investigating a startup
whydown diagnose --focus services --since 24h --current-boot
whydown diagnose --focus all --current-boot --progress

# Use in monitoring/cron/CI; exit 1 for warnings, 2 for invocation/output errors
whydown diagnose --focus all --exit-on-warning --format json --output report.json

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

**Important:** `--since` limits the time range; `--max-events` limits the number of journal entries inspected within that period. The default is **5,000** (maximum 10,000). WhyDown requests **N+1** entries from journalctl and examines only the newest N, so it can determine whether the tail was truncated. A truncated tail is marked **PARTIAL** in text/HTML/JSON, even if some anomalies were found. If no anomaly matches but the journal was truncated, WhyDown reports `INSUFFICIENT_EVIDENCE`, not `NO_CLASSIFIED_ANOMALY`. To see older incidents, increase `--max-events` or narrow/move the `--since` window. This is **not continuous monitoring** and does not guarantee every relevant event was captured.

### Example terminal output

```text
WhyDown 0.7.0 [DEMO]
====================================================================
Generated : 2026-11-11T20:00:00Z
System    : Demo Linux 1.0 (fictional)
Kernel    : 6.12.0-demo
Focus     : all
Journal   : last 1440 minutes, newest entries only
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
| dpkg package states (Debian-family) | `dpkg-query --show` against local package database | Warns about incompletely configured and reinstallation-required packages, not harmless residual configuration files; packages are only read, never configured |
| Disk utilization | `df -PT --local` plus exclusions for common network-backed FUSE types | Warns at >=90% for writable-capable local filesystem types; remote/FUSE mounts are excluded unless explicitly requested |
| Optional disk details | `du -x -k --max-depth=2` | Read-only breakdown for up to two eligible near-full non-root mounts; may be slow or incomplete |
| Memory pressure | `journalctl` | Detects OOM-related messages |
| Storage errors | `journalctl` | Detects common kernel/application storage error strings |
| Interface error counters | `/proc/net/dev` | Reports cumulative RX/TX error counters (not rates) |

### Correlating incidents and later observations (v0.7.0)

This release adds read-only context based on the real Manjaro logs from 2026-11-12. The journal showed multiple USB detachments in about 19 seconds, a Realtek r8152 error, `wlan0` deauthentication followed by association about 18 seconds later, a later local-choice Wi-Fi handoff with immediate reassociation, and a `lightdm.service` failure followed by a scheduled restart. Those observations do **not** prove the events were causally related.

WhyDown now:
- Treats **Wi-Fi association** as positive historical evidence. When the same explicitly named interface associates within 120 seconds of a link-loss event, the `LINK_DISCONNECTS` warning records that association, without declaring Internet connectivity restored.
- Distinguishes **local-choice Wi-Fi handoffs** from link-loss warnings. When a subsequent association is observed on the same interface within 30 seconds, `WIFI_LOCAL_ROAM_OBSERVED` is informational; normal roaming does not automatically mean the network failed.
- Recognizes `lightdm.service: Scheduled restart job` and other explicitly identified systemd units. The historical `SERVICE_FAILURE_LOGS` warning remains, but it shows the restart scheduling and current failed-unit state. `SERVICE_RESTART_OBSERVED` is informational when the unit is no longer listed as failed; it does not prove application readiness.
- Detects **USB_DISCONNECT_BURST** when four or more distinct USB topology paths detach within 30 seconds. This may reflect a dock/hub being unplugged or powered off, not a hardware fault. A nearby r8152 USB status error is mentioned only as context, never as proven causation. An isolated r8152 status -71 message is informational (`USB_TRANSPORT_STATUS`), not proof of hardware failure.
- Samples `/proc/net/dev` twice without an extra delay. The `system.interface_drop_deltas` JSON array records changes and elapsed milliseconds; only an increase of at least 10 drops over at least 100 ms produces the informational `RECENT_INTERFACE_DROPS`. Kernel counters may increase for reasons unrelated to real Internet packet loss. Counter resets are discarded.
- Adds informational `DISK_CAPACITY_WATCH` at 85–89% used, preserving the warning `DISK_NEAR_FULL` at 90% or more. The Manjaro samples of /home 88% and /opt 86% are advisory, not evidence of a current outage.

Firewall `[UFW BLOCK]` lines are not interpreted as automatic network outages; blocked multicast discovery messages can be normal under the configured rules. No raw USB serials, Wi-Fi BSSIDs, firewall IPs or journal text bodies are retained.

```bash
whydown diagnose --focus all --current-boot --since 24h --max-events 5000 --progress
whydown diagnose --focus all --current-boot --format json --output whydown-v070.json
jq '{findings: [.findings[].id], interface_drops: .system.interface_drop_deltas}' whydown-v070.json
```

### Why two similar diagnostics can give different results (v0.6.1)

A Manjaro v0.6.0 test on 12 November 2026 exposed an important coverage gap: `--max-events 5000` found one historical `wlan0` disconnection and one `lightdm.service` failure; a JSON rerun seconds later without `--max-events` used the old default **500** and reported `NO_CLASSIFIED_ANOMALY`. The JSON still said that only the newest 500 entries had been read. This does **not** show recovery: the two commands inspected different subsets of the journal.

WhyDown v0.6.1 raises the default to 5,000 and fetches one extra entry to establish **actual journal truncation** without exporting raw logs. It records `journal_truncated` and `journal_entries_examined` in schema-v2 JSON. Text and HTML say `PARTIAL` when records were excluded; any matching findings remain valid but a separate informational `JOURNAL_WINDOW_TRUNCATED` finding makes incomplete coverage visible. With no matching rule and truncated journal, the result is informational `INSUFFICIENT_EVIDENCE`, not `NO_CLASSIFIED_ANOMALY`.

```bash
# Same limits and scope for text and JSON: comparable results
whydown diagnose --focus all --current-boot --since 24h --max-events 5000
whydown diagnose --focus all --current-boot --since 24h --max-events 5000 --format json --output report.json
jq '{journal_checked, journal_truncated, journal_entries_examined, findings: [.findings[].id]}' report.json
```

### Evidence completeness and safer incident interpretation (v0.6.0)

A report with no recognized errors can still be incomplete. WhyDown now tracks whether the journal and filesystem capacity were actually queried. If no rule matches **and** core collectors were unavailable, it emits `INSUFFICIENT_EVIDENCE` (info, scope `unverified`) instead of `NO_CLASSIFIED_ANOMALY`. A visible **COLLECTION COVERAGE** section in text/HTML and additional JSON fields make unavailable data explicit. An optional sensor being absent is *never* proof that the corresponding subsystem is healthy.

The kernel journal parser now correctly extracts colon-suffixed interface identifiers such as `wlan0:` and `enp3s0:` from explicit link-loss messages. Previously the colon was stripped before a test for the colon, making that extraction path unreachable. This improves attribution for many drivers; messages with no explicit interface remain correctly labeled `unknown` rather than guessed.

The resolver parser now requires a syntactically valid IPv4 or IPv6 literal in a `nameserver` directive; arbitrary words are no longer counted as working DNS configuration. This is only a **configuration syntax check**, not an active DNS probe or proof that a nameserver is reachable.

The system focus also reports whether the visible root mount is `ro` or `rw` according to `/proc/self/mountinfo`. `ROOT_FILESYSTEM_READ_ONLY` is informational because immutable OS images and containers can deliberately use a read-only root. Explicit `too many open files` / `file table overflow` messages produce historical `FD_EXHAUSTION_LOGS` warnings independently of the kernel-global file-handle snapshot. High sustained CPU PSI `some avg60 >=90%` produces the **informational** `CPU_CONTENTION_PRESSURE` finding; intentional heavy workloads are not necessarily faults. `--current-boot` journal incident windows are now labeled `current_boot_filtered` when journalctl successfully applied the boot filter, rather than using a clock/uptime estimate.

```bash
whydown diagnose --focus all --current-boot --max-events 5000 --progress
whydown diagnose --focus network --since 2h --format json
```

### Filesystem collection and diagnosing slow runs (v0.5.2)

WhyDown v0.5.0 occasionally spent a long time on a diagnostic and then reported `Filesystem usage unavailable` on Manjaro; a second run produced disk data. The affected host has a mounted OneDrive path. A transient FUSE/remote-mount delay is **a hypothesis, not a verified root cause** because v0.5.0 did not measure individual command timings.

By default, WhyDown now runs `df -PT --local` and `df -Pi --local`, additionally excluding common network-backed FUSE types such as `fuse.rclone` and `fuse.sshfs`. This reduces the risk of a remote or broken FUSE mount delaying local filesystem diagnosis, but **does not test disk usage on excluded mounts**. To deliberately include remote mounts, opt in:

```bash
whydown diagnose --focus system --current-boot --progress
whydown diagnose --focus system --include-remote-mounts --progress
```

On both successful and failed collectors, reports now contain a **COLLECTOR TIMINGS** section showing measured wall-clock milliseconds and an explicit success/failure indicator. The JSON report contains a new top-level `collector_timings` array with safe command labels, elapsed milliseconds and success flags; HTML also displays the timings. Actual errors (timeout or failed `df`) are reported as limitations. If `df` exits with an error but supplies valid data for some mounts, those mounts are retained and the report clearly marks the result as partial instead of discarding every filesystem.

These checks remain read-only. Network storage, rclone mounts, NFS, CIFS and other special filesystems may still need dedicated diagnostics; `--local` and the named FUSE exclusions do not guarantee protection from every possible blocked kernel filesystem operation. Individual tool timeouts are also subject to Linux uninterruptible I/O behavior.

### Live diagnostic progress (v0.5.1)

When diagnostics take time, WhyDown displays a **progress bar on stderr** in an interactive terminal (Bash, Zsh, SSH). It includes the number of completed stages, the running collector (such as `pacman -Dk`, `dpkg-query`, `df`, `du` or `journalctl`) and elapsed time. The bar keeps updating while a slow subprocess is running. **Percent is calculated from completed phases; it is not an estimated remaining-time percentage or a count of journal entries.** There is no invented ETA.

```text
[########------------]  40% (2/5) Checking storage and resource health | pacman -Dk (max 20s) | elapsed 6s
```

Use `--progress` to force visual progress even when stderr is redirected (newline-based logging without ANSI control codes), or `--no-progress` to keep automation completely quiet. Default is automatic, enabled only when **stderr is an interactive TTY**. Progress is never written to stdout, so `--format json`, piping to `jq`, or writing `--output report.json` remains valid. Progress rendering is paused and its terminal line cleared **before** the report is displayed or its filename is announced; the final completion line appears **after** successful output. An interrupted command never incorrectly displays 100% completion.

```bash
whydown diagnose --focus all --current-boot --max-events 5000
whydown diagnose --focus system --progress
whydown diagnose --format json --progress > result.json  # JSON remains valid
whydown diagnose --format json --no-progress > result.json
```

### Portable Linux host resource diagnostics (v0.5.0)

The `--focus system` and `--focus all` modes now include these **read-only** signals wherever the kernel exposes them:

| Signal | Source | Conservative warning rule | Interpretation limit |
| --- | --- | --- | --- |
| Low memory headroom | `/proc/meminfo` `MemAvailable` | Below 3% of total **and** below 512 MiB | Host-level reading; not a container/cgroup quota or proof of OOM |
| Memory stalls | `/proc/pressure/memory` | `full avg60` at least 10% | One-minute stall signal; not the responsible process |
| I/O stalls | `/proc/pressure/io` | `full avg60` at least 15% | Cannot prove a disk is defective |
| Kernel file-handle allocation | `/proc/sys/fs/file-nr` | At least 95% of kernel maximum | Not per-process open-file limit |
| Inode usage | `df -Pi --local` (same remote/FUSE exclusions) matched to selected `df -PT` mounts | At least 95% | Excludes known remote mounts and full-by-design immutable images unless remote scanning is enabled |
| Linux MD RAID | `/proc/mdstat` | Explicit active member count below expected count | Does not cover hardware RAID, ZFS or Btrfs array health |

The tool reports PSI averages as percentages in terminal and HTML. Machine-readable JSON v2 exposes integer **hundredths of a percent** (`*_avg60_percent_x100`), avoiding ambiguous floating-point representation. An unsupported `/proc` source is **unverified, not healthy**. Host-wide measurements can differ from container resource limits.

These rules complement existing checks for systemd, DNS/DHCP and interfaces, journal errors, disk capacity, pacman and dpkg. **No single snapshot can identify all possible Linux failures**: SMART devices, firmware, hardware RAID, GPU faults, container health, Btrfs/ZFS pool condition, application-specific failures, detailed network reachability, SELinux/AppArmor policy and service-level SLOs require specialized probes and evidence. WhyDown remains intentionally offline, non-privileged and non-mutating; it cannot certify a system as healthy.

```bash
whydown diagnose --focus system --current-boot --since 24h
whydown diagnose --focus all --format json --output whydown-v050.json
```

### Arch / Manjaro / CachyOS pacman diagnostics (v0.4.1)

On systems with `/var/lib/pacman/local`, `--focus system` and `--focus all` now perform `pacman -Dk` in read-only mode, using the same bounded, local-only subprocess collector. This is a **pacman local database consistency check**, not installed file integrity checking, an update check or a proof of overall system health. It can detect missing metadata, dependency inconsistencies and ownership conflicts. A nonzero return yields `PACMAN_DATABASE_CHECK_FAILED` with limited confidence because command access/errors also need to be ruled out.

```bash
whydown diagnose --focus system --current-boot
pacman -Dk       # read-only consistency check to inspect details
pacman -Qk       # separate, potentially slower installed-file presence check
```

On Manjaro, an optional dpkg installation may also have a database; WhyDown clearly labels that as a **dpkg-only** check. A dpkg clean result does **not** establish that pacman is healthy. The native pacman check does not synchronize repositories, install, remove or configure packages.

### Debian / Ubuntu package-manager diagnostics (introduced in v0.3.6)

On dpkg-managed systems, `--focus system` and `--focus all` now check the package database with a read-only `dpkg-query` call. Broken/incomplete states appear as `DPKG_INCOMPLETE_PACKAGES` (current snapshot) with affected package names and states. A successful systemd check does **not** imply healthy packages.

This rule is independent of `--since` (which limits journal entries), does not inspect historical APT transactions, and does not determine *why* package configuration failed. Missing/inaccessible dpkg tools are recorded as collection limitations rather than clean results. **Live Debian 13 verification (2026-11-12):** WhyDown v0.3.6 reported all four incomplete kernel packages previously identified by `dpkg --audit`: three `unpacked` and one `half-configured`. Both systemd managers reported zero failed units. No repair was performed.

```bash
whydown diagnose --focus system --since 24h
dpkg --audit
apt-get -s -f install  # simulation only; no changes
```

WhyDown **never executes** either `dpkg --configure -a` or `apt-get -f install`. Such commands can change the system and should be run only after inspecting the specific failure, especially for kernel packages.

### How confident are the diagnoses?

- **Observed:** a specific state or classified message was present in collected evidence. Does not mean the journal classifier understood the context perfectly.
- **Moderate:** a bounded chronological correlation was detected. It may suggest the order of investigation but **does not prove causation**.
- **Limited:** missing or inconclusive evidence prevents a stronger conclusion.

`NO_CLASSIFIED_ANOMALY` does **not** mean a healthy system. It means that no available observation matched the current detection rules.

## Cross-distribution testing (v0.3.0)

WhyDown provides a common report format for comparing Manjaro, Raspberry Pi OS, Debian, CachyOS, Ubuntu Server and Kali Purple. **Manjaro x86_64 has been manually tested through v0.7.1:** static-musl installation and SHA-256 verification, interactive progress without interleaved report lines, current-boot network/services/system diagnostics, resource PSI/memory/inode snapshots, native pacman consistency, auxiliary dpkg, JSON export and per-collector timings. A representative v0.5.3 run completed in around 2 seconds, reported zero failed system/user units and two historical log warnings (link loss and lightdm), not verified ongoing failures. The v0.6.0 kernel-interface fix has been confirmed live with `wlan0` attribution, boot-filtered journal windows and coverage states. A same-day JSON report with the old 500-entry default missed historical events detected at 5,000 entries; v0.6.1 addresses that discrepancy. v0.6.1 was subsequently tested live: a 4,598-entry current-boot window was not truncated, and both text and JSON consistently retained the historical wlan0 and lightdm warnings. The v0.7.0 correlations were subsequently validated on Manjaro with 5,356 journal entries examined without truncation. The v0.7.1 source build passed 88 Rust unit tests, 11 CLI integration tests, and a live diagnostic with one-second runtime, confirmed `lightdm.service` active/running after its restart, and isolated a USB burst of 10 topology paths on the `1-1` root-port branch. The published v0.7.1 x86_64-musl archive was subsequently installed using the guided installer with successful SHA-256 verification and `whydown 0.7.1` output. Its live four-hour all-focus report examined 6,706 journal entries without truncation in about one second, confirmed the LightDM state (`active/running`, `Result=success`, `NRestarts=1`), and identified 10 USB topology paths on root-port branch `1-1` with nearby r8152 status. This validates the official release binary on Manjaro without identifying the original hardware fault. The cause of the original USB and Wi-Fi incidents remains unverified. Raspberry Pi OS 64-bit (Debian 12, ARM64, kernel 6.12.115+rpt-rpi-v8) compiled and ran service diagnostics with v0.3.3/v0.3.4. The v0.3.4 GNU binary failed at runtime with `GLIBC_2.39 not found`; the v0.3.5 static-musl installer has now been tested successfully on real Raspberry Pi hardware: ARM64 target detection, archive download, SHA-256 verification, installation under `~/.local/bin`, version check, and service diagnosis all passed. A Debian 13 (Trixie) x86_64 virtual machine has also been manually tested with the v0.3.5 static-musl installer and text diagnostics for `all`, `network`, `services` and `system` as root. It identified `enp0s3` as up, local IPv4/IPv6 default routes, and zero failed system units. The root session's user systemd manager was **unavailable, not verified healthy**. A separate APT/dpkg kernel-package failure was not detected because package-manager transaction diagnostics are not implemented. JSON/HTML exports on this VM are still pending; user-service diagnostics were subsequently checked successfully with the normal `hamdy` account (zero failed user units). The v0.3.6 dpkg rule was subsequently **validated live** on the same Debian 13 VM: `--focus all` and `--focus system` both reported `DPKG_INCOMPLETE_PACKAGES` with exactly four affected kernel packages, matching `dpkg --audit` (three unpacked and one half-configured). The diagnostic independently reported zero failed system and user systemd units. JSON/HTML export checks on this VM are still pending. Other distributions and ARM64 devices beyond the tested Raspberry Pi remain untested. See the [validation matrix](docs/TEST_MATRIX.md).

```bash
whydown diagnose --since 24h --max-events 5000 --format json --output report.json
python3 -m json.tool report.json >/dev/null
```

**JSON schema:** `schema_version` remains `2` in v0.10.0 (the breaking transition from v1 occurred in v0.3.0). New optional fields include `system.failed_user_units`, `system.system_units_checked`, `system.service_statuses`, and `service_event_windows`. Reports include `summary`, `findings[].scope`, optional `source`/`subject`, kernel interface `operstate`, and per-interface incident subjects. See [JSON report schema v2](docs/REPORT_SCHEMA.md). `operstate=up` does not establish internet reachability.

## Reliable automation and bounded collection (v0.4.0)

`whydown diagnose` is a **snapshot**, not an ongoing health agent. By default, exit code **0** means the diagnostic command executed and produced a report; it does not imply the system is healthy. For scripts, opt in to warning-dependent status:

```bash
whydown diagnose --focus system --exit-on-warning
status=$?
# 0 = no WARNING findings; 1 = one or more WARNING findings;
# 2 = invalid invocation or report output failure.
printf 'WhyDown exit status: %s\n' "$status"
```

`--current-boot` limits journal collection to the active boot through `journalctl --boot=0` in addition to `--since`. This avoids mixing previous-boot journal events into a targeted investigation. It does **not** restrict current systemd/dpkg/disk/network snapshots. The report explicitly states the journal scope, and JSON schema v2 includes `journal_current_boot_only` (additive, no schema bump). A historical boot estimate remains a heuristic when current-boot filtering is not requested.

To prevent indefinite hangs or unbounded memory, WhyDown enforces a **20-second timeout** on normal subprocess collectors, **45 seconds on journalctl** and **120 seconds on optional `du` disk breakdowns**, with an output cap of **16 MiB** (32 MiB for journalctl). Incomplete/oversized or timed-out results are reported as collection limitations rather than silently used as complete evidence. Under `--disk-details`, directory traversal may still cause substantial disk I/O until its timeout.

Reports may expose unit names, mountpoints and other local data. On Unix, files written with `--output` are mode `0600`. Without `--overwrite`, an existing output file is never overwritten. With `--overwrite`, WhyDown writes and syncs a temporary private file in the destination directory before an atomic rename so an interrupted write does not destroy the existing report. Review exports before sharing.

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
- **v0.3.3 (released):** correct singular/plural labels in diagnostics and reports without changing detection logic or JSON schema.
- **v0.3.4 (released):** guided Linux x86_64/ARM64 installer with automatic ABI detection, SHA-256 verification, and optional source build/preflight help; Raspberry Pi OS guidance.
- **v0.3.5 (released):** static musl by default, candidate compatibility checks before installing, and preservation of previous executables when installation fails.
- **v0.3.6 (released):** detect incomplete Debian dpkg package states in system/all diagnostics, show safe investigation commands, and include findings in JSON/HTML reports.
- **v0.4.0 (released):** bounded and timed subprocess collection, optional current-boot-only journal analysis, monitoring exit codes, safer atomic private report replacement and associated tests.
- **v0.4.1 (released):** distinguish native pacman checks from optional dpkg data on Arch-family systems, add read-only pacman database consistency results to text/JSON/HTML, and document Manjaro v0.4.0 field validation.
- **v0.5.0 (released):** extend read-only host diagnostics to MemAvailable, 60-second memory/I/O PSI, kernel file-handle limits, inode exhaustion and Linux MD RAID degradation; include test fixtures, privacy-safe metrics and schema-v2 exports.
- **v0.5.1 (released):** interactive progress bar with animated heartbeat, completed-stage percentage, current command and elapsed time; separate stderr output and opt-in/out flags, with tests and no JSON schema changes.
- **v0.5.2 (released):** default local-only disk and inode collection excluding common remote/FUSE backends, optional remote scan, partial df results preserved, precise per-collector timing telemetry in all report formats (local-only; no network transmission).
- **v0.5.3 (released):** prevent progress/JSON or progress/text collisions and duplicate completion lines, keeping the final 100% status conditional on successfully written output; validate Manjaro v0.5.2 field timings.
- **v0.6.0 (released):** fix kernel interface attribution, validate DNS IP literals, distinguish missing evidence from no anomaly, record boot-filtered incident provenance, and add informational root/CPU observations plus historical file descriptor exhaustion classification.
- **v0.6.1 (released):** increase default journal tail to 5,000, detect true truncation with N+1 sentinel, explicitly disclose partial journal coverage and prevent misleading no-anomaly conclusions when older entries were excluded.
- **v0.7.0 (released):** correlate Wi-Fi reassociation and local-choice roaming, systemd restart scheduling, and USB-detachment bursts; sample network drop counter deltas; add informational 85–89% disk utilization advice.
- **v0.7.1 (released):** refine USB disconnect grouping by root-port branch, avoid assuming event ordering within the same journal second, and add bounded read-only current systemd unit-state snapshots after attributable historical service failures. Verified in live Manjaro source tests and with the published x86_64-musl release archive; application readiness and hardware root causes remain outside the tool's claims.
- **v0.8.0 (released):** interactive, dependency-free terminal evidence browser with configurable focus, journal lookback, current-boot filter and entry cap, findings with explanations/evidence, timeline, collection coverage and safe export guidance; supports fictional demo mode and rejects non-TTY automation. Existing diagnostic engine and JSON schema v2 remain unchanged.
- **v0.8.1 (released):** clarify number-plus-Enter TUI controls, remove the extra pause that could swallow input after diagnosis, show setting-change feedback, and allow menu shortcuts at detail-page return prompts. Add real Linux PTY navigation tests to CI and release checks; on-host published-binary verification pending.
- **v0.9.0 (released):** optional offline Tkinter desktop GUI, embedded in the Rust binary and launched with `whydown gui`; focus **all** by default, one-click diagnostics, immediate findings/evidence, timeline and coverage tabs, safe opt-in JSON export, guided CLI/TUI defaults and clearer analyze-first terminal flow. `install.sh --desktop` can create an explicit opt-in application launcher. CI includes Python GUI model tests and an actual Tk window under Xvfb; real Manjaro desktop testing remains pending.
- **v0.9.1 (released):** installer checks optional Python 3 and a working `tkinter` import, prints actionable distribution-specific package advice, warns if a new desktop shortcut cannot yet launch the GUI, and adds a standalone `--check-gui-prereqs` preflight. CLI/TUI installs stay valid without Python/Tk; no automatic package installation or privilege escalation. Dedicated installer smoke tests cover Python/Tk missing or available.
- **v0.9.2 (released):** selectable GUI evidence/coverage text, context-menu copying, Ctrl+C/Ctrl+A shortcuts, a Copy selection button, and row copying from findings/timeline; clipboard use is explicit, scoped to selected content and never uploads or automatically saves reports. Real Xvfb smoke tests exercise partial copying and error handling.
- **v0.10.0 (released):** replace the experimental Tkinter GUI with a modern Qt 6/PyQt6 desktop: dark sidebar, polished cards, clearer hierarchy, interactive finding/event views and native, mouse-selectable *read-only* evidence. Async bounded Qt process runs Rust diagnostics; private opt-in JSON export and offline rules remain unchanged. New real Qt6/Xvfb regression test exercises mouse double-click selection, Ctrl+C and exact clipboard contents; the installer checks PyQt6 on Arch/Manjaro, Debian/Ubuntu and Fedora.
- **v0.10.0 (planned):** broader cross-distribution test fixtures, optional privacy-conscious evidence bundles and configurable diagnostic rules.
- **Future:** optional local model explanations and broader Linux diagnosis, *never at the expense of verifiable evidence*.

Features not implemented in v0.10.0: continuous monitoring, live network health checks, predictive failures, auto-repair, system-wide event tracing, package history correlation, privileged packet capture, external DNS probing, and cloud dashboards.

## Contributing

Contributions are welcome: reproducible bug reports, sanitized fixtures, new detection rules, documentation and portable build tooling. See [CONTRIBUTING.md](CONTRIBUTING.md).

Please **do not upload unredacted logs** in public issues. Include Linux distribution, kernel, relevant command output with private data removed, and a clear reproduction recipe.

## License

**GNU General Public License v3.0 only (GPL-3.0-only).** See [LICENSE](LICENSE).

Copyright © 2026 Hamdy Abou El Anein and contributors.

WhyDown is provided **without warranty**, including any implied warranty of fitness for a particular purpose. See the license for details.
