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
