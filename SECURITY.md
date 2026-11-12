# Security policy

## Supported releases

WhyDown is an early-stage MVP. Security issues should be reported against the most recent release or `main` branch once published.

## Reporting a vulnerability

Please use **GitHub private vulnerability reporting**, if enabled for the repository, rather than creating a public issue. If unavailable, contact the repository owner using a contact method they publish after repository setup. No security email address has been configured for this initial source package.

Do not post sensitive system logs, credentials, exploitable proof-of-concept payloads, or private network information in public issues.

## Security principles

- No root requirement or privilege escalation
- No auto-remediation or arbitrary command execution
- Fixed command invocation without shell interpretation
- No outbound network communication or telemetry
- Report exports omit raw journal message bodies
- HTML output escapes dynamic content to prevent embedded scripting

This software is **not** intended to defend a compromised host or to serve as a trusted forensic record.


## v0.4.0 collection and report safety

Subprocess data collection is bounded by elapsed time and stdout volume to avoid indefinite hangs or unbounded output retention. A failed collector is reported as a limitation; no incomplete capture should be interpreted as a clean check. The optional disk breakdown can still cause disk I/O until its timeout.

Unix report files use permission mode `0600`. `--overwrite` stages, synchronizes and atomically renames a replacement, preserving previous content when staging/writing fails. Exported data may reveal local package names, interface labels, service names or filesystem paths; review before distributing.

`--exit-on-warning` is opt-in and reports findings, not verified root causes. Never use it as a substitute for authoritative host monitoring.


## v0.7.0 safe incident correlation

Only short, validated USB topology port identifiers and Wi-Fi/systemd interface or unit identifiers are kept in parsed events; Wi-Fi BSSIDs, kernel message bodies, firewall IPs/ports and device serial numbers remain excluded. Two /proc/net/dev reads are passive, local snapshots and do not inject traffic. USB and Wi-Fi correlation is timing evidence rather than proof of hardware failure or network recovery; a scheduled systemd restart is not proof of service readiness. Never include sensitive raw log lines in published test fixtures.
