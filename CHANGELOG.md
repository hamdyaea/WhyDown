# Changelog

All notable changes to WhyDown will be documented here.

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
