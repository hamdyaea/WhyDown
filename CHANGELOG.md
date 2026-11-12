# Changelog

All notable changes to WhyDown will be documented here.

## [0.4.1] - 2026-11-12

### Added
- Read-only `pacman -Dk` local package database consistency check on hosts with `/var/lib/pacman/local`, covering native Arch Linux, Manjaro and CachyOS systems.
- Current-snapshot warning `PACMAN_DATABASE_CHECK_FAILED` for nonzero pacman exit codes (limited confidence: not proof of corruption); separate `pacman_checked` and `pacman_consistent` fields in JSON schema v2, plus terminal and HTML display.
- Clear label that a dpkg database on Manjaro is a separate, auxiliary check and does not verify pacman.
- Regression tests for successful, failing and unavailable pacman database checks, including report formats.
- Record successful field test of v0.4.0 on Manjaro: static-musl x86_64 installer, SHA-256, current-boot diagnostics, JSON export and zero exit status for no warnings. The new v0.4.1 pacman check still requires on-host testing.

### Compatibility
- Existing dpkg handling and rules unchanged. `schema_version` remains 2 with additive fields. No network probes, package changes, third-party crates, privileged actions or raw package logs.

## [0.4.0] - 2026-11-12

### Added
- `--current-boot` (alias `--since-boot`) restricts journal evidence to the active Linux boot using `journalctl --boot=0`, combined with the requested `--since` window. The same scope is exposed in text, HTML and JSON schema v2 (`journal_current_boot_only`).
- `--exit-on-warning` gives automation a stable opt-in exit status: 0 after a successfully generated report without warning findings, 1 after a successfully generated report containing warnings, 2 for command or output errors. The default exit policy is unchanged.
- Safe, private report overwrite: stage on the destination filesystem, sync and atomically rename. Existing reports survive incomplete writes.
- Time-limited and output-bounded subprocess collection (20s ordinary commands, 45s journalctl, 120s optional du; 16 MiB stdout limit or 32 MiB for journalctl) to avoid indefinite hangs and unlimited memory usage.
- Unit/integration tests for CLI flags, timeout and output caps, machine-readable boot scope, warning exit policy, output replacement and 0600 permissions.

### Compatibility
- No new third-party Rust dependencies; read-only/offline diagnostic engine, existing finding IDs, installer ABI policy and JSON schema v2 remain unchanged.
- Current-boot filtering is opt-in; previous-boot journal diagnostics remain the default.

## [0.3.6] - 2026-11-12

### Added
- Read-only dpkg status diagnostics for Debian/Ubuntu and other dpkg-managed Linux distributions, using machine-readable `dpkg-query` rather than localized `dpkg --audit` text.
- Report unpacked, half-installed, half-configured, trigger-pending/awaited or reinstallation-required packages as current `DPKG_INCOMPLETE_PACKAGES` findings; ignore fully installed and residual config-only packages.
- Surface dpkg status and affected package identifiers in terminal, JSON schema v2 (additive fields) and HTML; distinguish absent dpkg databases from unsuccessful queries.
- Add regression tests using Debian 13 kernel package configuration failures observed on a real VM, and subsequently validate the v0.3.6 finding **on that VM**: four packages detected (three unpacked, one half-configured), while system/user systemd units remained healthy.
- Update README, report schema and cross-distribution test matrix. No repairs, package changes, telemetry or network requests.

## [0.3.5] - 2026-11-12

### Fixed
- Prefer statically linked musl binaries by default on both glibc and musl hosts so that older-glibc systems (Debian 12 / Raspberry Pi OS 64-bit) do not hit `GLIBC_2.39 not found`.
- Run and validate each downloaded candidate before replacing an existing binary; use a staged atomic rename to preserve prior installs if execution fails.
- Verify release musl binaries have no dynamic ELF interpreter at packaging time.

### Tests and documentation
- Offline installer tests cover default musl target, GNU override, successful installation, and rejected candidate rollback.
- README and test matrix document the v0.3.4 GNU ABI failure and the subsequent successful v0.3.5 ARM64 static-musl installation and service-diagnostic test on Raspberry Pi OS (2026-11-12).
- Debian 13 (Trixie) x86_64 VM: v0.3.5 binary installation and text diagnostics (`all`, `network`, `services`, `system`) manually exercised as root. User systemd manager and export formats remain unverified; a separate dpkg failure is outside current diagnostic coverage.
- Diagnostics and JSON schema version 2 unchanged.

## [0.3.4] - 2026-11-12

### Added
- Guided, unprivileged binary installer `scripts/install.sh` for Linux x86_64 and aarch64, with glibc/musl detection and verified release SHA-256 checksums.
- Optional `--from-source` and `--check-build-prereqs` modes explain when rustc, cargo and a C compiler are missing. No automatic package installation.
- No-network installer smoke test and source-install CI verification, including release packaging of `install.sh`.
- Detailed Raspberry Pi OS / Debian Rust installation and `PATH` troubleshooting guidance in README.

### Validation
- Raspberry Pi OS ARM64 (Debian 12 bookworm, kernel 6.12.115+rpt-rpi-v8): WhyDown v0.3.3 compiled successfully using Rust 1.99.1 and GCC 12.2 and detected failed system/user units. v0.3.4 installer still requires manual Raspberry Pi validation.

### Compatibility
- Diagnostic engine and JSON schema v2 unchanged.

## [0.3.3] - 2026-11-12

### Improved

- Correct count-dependent English wording in text and HTML diagnostics (e.g. "1 classified entry", "1 time window", "last 1 event", "0 failed units").
- Preserve classification rules, service time windows, severity and diagnostics unchanged.
- Add regression tests covering singular and plural report labels and historical pre-boot findings.

### Compatibility

- JSON schema remains version 2, with no new fields.

## [0.3.2] - 2026-11-12

### Fixed

- Avoid classifying sudo/doas/pkexec command audit messages as real service/network/storage failures when the command arguments merely include diagnostic phrases (for example, journalctl --grep patterns).
- Avoid classifying privilege-wrapper service-error text as a systemd unit failure.
- Keep real systemd failures visible, with a regression test for the search-term false positive.

### Improved

- Show an approximate boot relation for historical service-message windows, based on current uptime and report time, to distinguish entries older than the current boot.
- Indicate that wall-clock adjustments can make the boot-time estimate inaccurate. No raw journal logs or network probes are added.
- Update README and JSON schema reference without changing JSON schema version 2.

## [0.3.1] - 2026-11-12

### Improved
- Collect currently failed units from both the system and the invoking user's systemd managers. Unavailable user managers are reported as unchecked, not healthy.
- Group historical classified service messages into bounded five-second time windows and list identified units, including entries without identifiable units.
- Clarify that multiple service log entries do not establish the same number of failed services; a signal termination may coincide with shutdown and a historical failure may have recovered.
- Include current system/user failed-unit status and historical service-message windows in text, HTML and JSON reports.
- Add tests based on real Manjaro observations of rclone-onedrive and proton-backup failures, and update README/schema documentation.

### Compatibility
- JSON schema remains version 2, with additive fields only.

## [0.3.0] - 2026-11-12

### Added
- Attribute classified journal events to bounded program, network-interface or systemd-unit identifiers while discarding raw messages.
- Group historical network incidents separately by identified interface.
- Report read-only kernel interface `operstate` and local default routes separately from historical warnings.
- Add finding `scope` values (`current_snapshot`, `historical_journal`, `unverified`).
- JSON schema v2 introduces summary counters, interface `operstate`, incident `subject`, and timeline `subject`/`source` (breaking update).
- Document the test matrix for six Linux environments and the schema/limitations.

### Privacy
- Only short ASCII-validated source/subject identifiers are retained from journal messages.

## [0.2.2] - 2026-11-12

### Fixed

- Do not classify kernel messages such as `ata3: SATA link down (SStatus 4 SControl 300)` as Wi-Fi/Ethernet disconnects.
- Require network-component or interface context before interpreting generic `link down` text as a connectivity failure.
- Add tests for SATA/Bluetooth/non-network messages and genuine Ethernet link losses.

## [0.2.1] - 2026-11-12

### Fixed

- Do not flag NetworkManager's normal `activation: beginning transaction (timeout in 45 seconds)` informational message as an actual DHCP timeout.
- Ignore informational `state changed new lease` lines while preserving real DHCP timeout detection.
- Add regression tests reproducing Manjaro journal entries and confirm a real DHCP request timeout is still reported.

## [0.2.0] - 2026-11-11

### Added
- Group historical network journal observations into bounded incident windows; do not equate them with verified outages.
- Optional `--disk-details` for read-only directory-size breakdowns of up to two near-full non-root filesystems.
- Release workflow producing Linux x86_64 and ARM64 builds for GNU/glibc and musl with SHA-256 checksums.

### Fixed
- Avoid classifying normal NetworkManager shutdown/reboot DHCP cancellation as a Wi-Fi link failure.
- Add reboot and real-link-failure regression cases to distinguish routine administrative transitions from actual problems.

### Documentation
- Document release binaries, `--disk-details`, network incident windows and the one-time `Cargo.lock` migration required for some v0.1.x source checkouts.

## [0.1.1] - 2026-11-11

### Fixed

- Avoid false disk-full alerts from Snap SquashFS and other immutable compressed filesystem images that normally report 100% usage.
- Use filesystem type information from GNU `df -PT` when checking capacity.
- Preserve warnings for real mounts such as `/var`, overlay and tmpfs; add regression tests based on a Manjaro report.
- Fix README to reflect the published repository and disk collection method.

## [0.1.0] - 2026-11-11

### Added

- Rust CLI with `diagnose` and `demo` commands
- System identity, networking, DNS, systemd and filesystem snapshots
- Rule-based classification of journal incidents
- Link/DHCP-to-DNS chronological correlation
- Explanations and verification steps with confidence labels
- Plain text, structured JSON and self-contained HTML reports
- Privacy-by-default design without third-party dependencies
- Unit/integration tests, CI workflow, GPLv3 license and documentation

### Known limitations

- Linux-focused collectors and English-language journal patterns
- Only newest N journal lines are inspected within the selected time range
- No background event recording, network probes or automatic repairs
