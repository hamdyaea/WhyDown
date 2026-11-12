# Changelog

All notable changes to WhyDown will be documented here.

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
