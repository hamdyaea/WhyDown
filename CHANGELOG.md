# Changelog

All notable changes to WhyDown will be documented here.

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
