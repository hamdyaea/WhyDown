# Cross-distribution validation matrix

**Status: test plan, not a claim of completed manual validation.** CI builds on Ubuntu and checks synthetic journal fixtures. Manjaro diagnostics were tested through v0.3.3. Raspberry Pi OS ARM64 successfully built and diagnosed services with v0.3.3; the v0.3.4 GNU binary failed with GLIBC_2.39 not found on Debian 12. Retest with the v0.3.5 default static musl installer.

| Platform | Architecture | Manual test status | Targeted checks |
| --- | --- | --- | --- |
| Manjaro | x86_64 | Tested through v0.3.3; retest v0.3.5 installer | NetworkManager, XFS /var, pacman cache, reboots |
| Raspberry Pi OS 64-bit (Debian 12) | aarch64 | v0.3.3 source build passed; v0.3.4 GNU download failed at runtime; v0.3.5 musl retest pending | systemd, headless ARM, low-power disk I/O |
| Debian VM | x86_64 | Planned | systemd, DHCP, small VM disk |
| CachyOS | x86_64 | Planned | custom kernel, Arch packaging |
| Ubuntu Server | x86_64 or aarch64 | Planned | headless services, journal permissions |
| Kali Purple VM | x86_64 | Planned | nonstandard service log formats |

## Same tests on each host

```bash
whydown --version
whydown demo
whydown diagnose --focus all --since 24h --max-events 5000
whydown diagnose --format json --output report.json
python3 -m json.tool report.json >/dev/null
```

Review `schema_version: 2`, the `scope` of each finding, the `operstate` and `source`/`subject` fields, and any collection-limitations messages.
Compare live snapshots to `ip route`, `ip -6 route`, `ip -brief link`, `systemctl --failed` and `df -hT`.

Run `--disk-details` only when a directory walk is acceptable; it can cause significant I/O and is not part of the default run.
Do not deliberately break production networking or storage to simulate an incident. Use isolated VMs and sanitized journal fixtures for positive/negative detection cases.

**Redact reports before publishing:** interfaces, mountpoints, unit names, local paths and IP configurations may be sensitive even when raw journal bodies are excluded.


For services, compare `systemctl --failed` with `systemctl --user --failed`, and verify that `service_event_windows` represent log clusters, not the number of independently failed services. A user manager may be unavailable on headless/root-only systems.


In v0.3.2, verify that a `sudo journalctl --grep='Failed to start'` audit command does not itself become a service-failure event, and compare the approximate `boot_relation` with `journalctl --list-boots` when practical.


## v0.3.4 guided installer checks

Run `sh scripts/install.sh --print-target` and `--dry-run` to check architecture selection without downloading. Run `--check-build-prereqs` to verify source-build tools. The default installer downloads to `~/.local/bin` without Rust/Cargo or root privileges. After installation verify `~/.local/bin/whydown --version`. This feature must be validated manually on the Raspberry Pi before it is marked as hardware-tested.


## Debian 12 glibc ABI incident (2026-11-12)

The WhyDown v0.3.4 ARM64 GNU archive downloaded and passed its checksum on Raspberry Pi OS, but could not run because it required glibc 2.39 (Debian 12 provides 2.36). This is a **failed binary compatibility test**, not an installation of Rust failure. The v0.3.5 installer prefers statically linked musl and preflights executables before replacing earlier installs. CI verifies static musl releases and simulates failing candidates. Real Raspberry Pi OS v0.3.5 testing remains pending.
