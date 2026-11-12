# Cross-distribution validation matrix

**Status: mixed manual validations and pending test plan.** CI builds on Ubuntu and checks synthetic journal fixtures. Manjaro diagnostics were tested through v0.3.3. On Raspberry Pi OS ARM64 (Debian 12), v0.3.3/v0.3.4 source builds and service diagnostics succeeded; v0.3.4 GNU failed with `GLIBC_2.39 not found`; the v0.3.5 static-musl installation and service diagnostics succeeded on real hardware. Debian 13 (Trixie) x86_64 VM installation and four text diagnostic focuses were also manually exercised with v0.3.5 as root; user-manager, exports and APT/dpkg diagnostics remain incomplete.

| Platform | Architecture | Manual test status | Targeted checks |
| --- | --- | --- | --- |
| Manjaro | x86_64 | Tested through v0.3.3; retest v0.3.5 installer | NetworkManager, XFS /var, pacman cache, reboots |
| Raspberry Pi OS 64-bit (Debian 12) | aarch64 | v0.3.3/v0.3.4 source builds passed; v0.3.5 static-musl installer and service diagnostic **passed on hardware**; v0.3.4 GNU ABI issue documented | systemd, headless ARM, low-power disk I/O |
| Debian 13 (Trixie) VM | x86_64 | **Partially validated on v0.3.5**: static-musl installation and `--focus all/network/services/system` text reports run under root; user systemd unverified, JSON/HTML not yet checked | systemd, DHCP, APT/dpkg packaging, virtual NIC `enp0s3` |
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

Run `sh scripts/install.sh --print-target` and `--dry-run` to check architecture selection without downloading. Run `--check-build-prereqs` to verify source-build tools. The default installer downloads to `~/.local/bin` without Rust/Cargo or root privileges. After installation verify `~/.local/bin/whydown --version`. The v0.3.5 default static-musl installer **was validated manually on Raspberry Pi OS ARM64 on 2026-11-12**. Other host types and installer modes still require manual tests.


## Debian 12 glibc ABI incident (2026-11-12)

The WhyDown v0.3.4 ARM64 GNU archive downloaded and passed its checksum on Raspberry Pi OS, but could not run because it required glibc 2.39 (Debian 12 provides 2.36). This is a **failed binary compatibility test**, not an installation of Rust failure. The v0.3.5 installer prefers statically linked musl and preflights executables before replacing earlier installs. CI verifies static musl releases and simulates failing candidates. The v0.3.5 test on Raspberry Pi OS hardware **succeeded**: target `aarch64-unknown-linux-musl`, archive and checksum, executable installation and service diagnosis.


## Hardware validation record — Raspberry Pi OS ARM64 (2026-11-12)

- OS: Debian GNU/Linux 12 (bookworm) / Raspberry Pi OS 64-bit; kernel `6.12.115+rpt-rpi-v8`.
- Auto-selected target: `aarch64-unknown-linux-musl`.
- v0.3.5 installer downloaded and SHA-256-verified the archive (`OK`), and installed the binary under `~/.local/bin`.
- Runtime: `whydown 0.3.5` and read-only `diagnose --focus services --since 24h --max-events 5000` completed.
- Current systemd results: two failed system units and one failed user unit, consistent with prior source-built runs.
- The tested installer did not invoke Cargo, but Rust/Cargo were present on the test host. This is not a test on a Rust-free installation.
- Other CPU/kernel combinations, all feature areas, and other Linux distributions remain outside this manual validation.

## Manual validation record — Debian 13 (Trixie) x86_64 VM (2026-11-12)

- Distribution: Debian GNU/Linux 13 (Trixie), kernel `6.12.115+deb13.1-amd64` in screenshots.
- Installer: under a **root shell**, automatic target `x86_64-unknown-linux-musl`, `--dry-run`, download and successful SHA-256 verification; installed under `/root/.local/bin/whydown`. `--version` reported `whydown 0.3.5`.
- Successful read-only text report execution: `--focus all`, `network`, `services`, and `system`, using `--since 24h --max-events 5000`. Each returned `NO_CLASSIFIED_ANOMALY`; this is a rule-match result, **not system health certification**.
- Network snapshot: local IPv4 and IPv6 default routes present, `enp0s3` kernel operstate `up`; neither demonstrates internet connectivity.
- Systemd: zero failed system units at snapshot; root user's systemd manager unavailable, correctly reported as `not checked`. The normal user's manager was not tested.
- A prior `dpkg` command reportedly exited with code 1 during a kernel package operation. This **was not detected** by WhyDown's current rules; it does not currently inspect APT/dpkg transaction failures or unfinished package configurations. Collect `dpkg --audit` and `apt-get -s -f install` in the VM to investigate separately, without automatic repair.
- Not yet manually verified on this VM: diagnostics as unprivileged user, JSON schema-v2/HTML output, representative real network/storage failures, or all available journal entries.
