# Cross-distribution validation matrix

**Status: mixed manual validations and pending test plan.** CI builds on Ubuntu and checks synthetic journal fixtures. Manjaro was manually tested with v0.4.0 on 2026-11-12 (x86_64 static-musl installer, current-boot all-focus diagnostic, JSON export and no-warning exit code 0). On Raspberry Pi OS ARM64 (Debian 12), v0.3.3/v0.3.4 source builds and service diagnostics succeeded; v0.3.4 GNU failed with `GLIBC_2.39 not found`; the v0.3.5 static-musl installation and service diagnostics succeeded on real hardware. Debian 13 (Trixie) x86_64 VM installation and four text diagnostic focuses were manually exercised with v0.3.5 as root and later as the normal user. The **v0.3.6 dpkg diagnostic has now been validated live**, identifying four kernel packages in incomplete states while both systemd managers reported zero failed units. JSON/HTML exports on the VM still require manual checking.

| Platform | Architecture | Manual test status | Targeted checks |
| --- | --- | --- | --- |
| Manjaro | x86_64 | **v0.4.1 tested live:** binary install/sha passed; pacman database check passed and matched pacman -Dk; optional dpkg check independently reported 0. v0.5.0 host resource probes still need a manual run. | NetworkManager, XFS /var, pacman local DB, reboots |
| Raspberry Pi OS 64-bit (Debian 12) | aarch64 | v0.3.3/v0.3.4 source builds passed; v0.3.5 static-musl installer and service diagnostic **passed on hardware**; v0.3.4 GNU ABI issue documented | systemd, headless ARM, low-power disk I/O |
| Debian 13 (Trixie) VM | x86_64 | **v0.3.6 dpkg detection validated live**: `all` and `system` reported exactly four incompletely configured kernel packages previously confirmed by `dpkg --audit`; zero failed system/user units. Static-musl installation and previous text diagnostics also tested. JSON/HTML exports pending | systemd, DHCP, dpkg package-state integrity, virtual NIC `enp0s3` |
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
- A prior `dpkg` command exited with code 1 during a kernel package operation. WhyDown v0.3.5 did not detect the resulting incomplete configurations, but **v0.3.6 did detect those package states** on the same VM. It still does not determine the root cause of the package transaction failure. Collect `dpkg --audit` and `apt-get -s -f install` in the VM to investigate separately, without automatic repair.
- Subsequently manually verified on the same VM: diagnostics as unprivileged `hamdy` show zero currently failed system/user units. Still pending: JSON schema-v2/HTML export, representative real network/storage failures, and all available journal entries.


## dpkg status fixture — Debian 13 kernel package failure (2026-11-12)

`dpkg --audit` showed four not-fully-configured packages:
- `linux-headers-6.12.116+deb13-amd64`: unpacked
- `linux-headers-amd64`: unpacked
- `linux-image-amd64`: unpacked
- `linux-image-6.12.116+deb13-amd64`: half-configured

`apt-get -s -f install` showed four `Conf` actions and no actual repair was run. WhyDown v0.3.5 reported no classified anomaly under `--focus system`, demonstrating a gap in package-state coverage despite zero currently failed systemd units. v0.3.6 adds read-only dpkg-query collection and synthetic regression tests reproducing these four states. **Real VM validation succeeded on 2026-11-12**: `--focus all` and `--focus system` reported the `DPKG_INCOMPLETE_PACKAGES` warning, counted four packages and listed all affected names/states, while the current systemd snapshot reported zero failed system/user units. Root-cause investigation and any package repair have not been performed.

## Real Debian 13 dpkg diagnostic validation (2026-11-12)

WhyDown v0.3.6 was manually run as the normal user `hamdy` in the Debian GNU/Linux 13 (Trixie) x86_64 VM, kernel `6.12.115+deb13.1-amd64`. Both `--focus system` and `--focus all` identified:
- `linux-headers-6.12.116+deb13-amd64`: `unpacked`
- `linux-headers-amd64`: `unpacked`
- `linux-image-6.12.116+deb13-amd64`: `half-configured`
- `linux-image-amd64`: `unpacked`

The finding was `DPKG_INCOMPLETE_PACKAGES`, severity `WARN`, scope `current_snapshot`, and the count was exactly **4**, consistent with an earlier `dpkg --audit` inspection. Both system and user systemd managers reported **0 failed units**. These observations validate the new rule on a real broken dpkg state; they do **not** establish why the package configuration failed, that system health is otherwise verified, or that a repair succeeded. The session did not run repair commands. JSON/HTML reports have not yet been manually verified on this VM.


## v0.4.0 release validation checklist

- Test `--current-boot --since 24h` on systems with multiple boots; compare visible journal timestamps with `journalctl --boot=0`. The JSON field `journal_current_boot_only` should be true.
- Test `--exit-on-warning` on synthetic demo data: exit code 1 with warnings and a valid report, while the default command still returns 0. Invalid arguments/output errors use code 2.
- Test report overwrite: existing file is protected by default; explicit `--overwrite` replaces a report using private 0600 permissions and atomic rename.
- Test bounded child output and timeout with synthetic commands. Limitations should identify failed collection instead of claiming a clean system.
- Do not claim new manual Raspberry Pi OS, Debian, Manjaro or VM validation until v0.4.0 is actually executed on the respective host. These additions are subject to GitHub Actions tests only at publication time.


## Manjaro x86_64 — v0.4.0 live validation (2026-11-12)

- Downloaded and installed `whydown-v0.4.0-x86_64-unknown-linux-musl.tar.gz` with the installer. SHA-256 verification succeeded; running `~/.local/bin/whydown --version` returned `0.4.0`.
- On Manjaro Linux kernel `7.2.9-1-MANJARO`, `diagnose --focus all --current-boot --since 24h --max-events 5000` returned no classified anomalies in examined observations. The snapshot showed `wlan0` up, both default routes present and zero currently failed system/user systemd units.
- The system also exposed a dpkg database reporting 0 incomplete states, **not a valid pacman health check**. v0.4.1 adds a separate native pacman check.
- A JSON report was saved with `--exit-on-warning`; its exit code was 0 (no warning), as expected. The warning exit code 1 was not manually tested on Manjaro; automated tests cover it.
- Pending: actual v0.4.1 pacman output on Manjaro/CachyOS, HTML export and failure-injection checks.


## Manjaro x86_64 — v0.4.1 native pacman check validated (2026-11-12)

User installed v0.4.1 via the default x86_64-musl installer, with successful SHA-256 validation. The `--focus system --current-boot --since 24h` report on kernel `7.2.9-1-MANJARO` showed `CURRENT PACMAN LOCAL DATABASE CHECK: consistency check passed`. A separate manual `pacman -Dk` command returned no database errors. The optional dpkg database also reported 0 incomplete states but is correctly labeled as dpkg-only. Collection was limited to the newest 500 journal events. **This establishes pacman database check behavior, not comprehensive system health.**

## v0.5.0 resource sensor manual test plan

Run `whydown diagnose --focus system --current-boot --since 24h` and `whydown diagnose --format json --output resource-report.json` on each target distribution. Compare host metrics with `cat /proc/meminfo`, `cat /proc/pressure/{cpu,memory,io}`, `cat /proc/sys/fs/file-nr`, `df -Pi`, and `cat /proc/mdstat` where those interfaces exist. Do not fabricate outages or intentionally exhaust RAM, inodes, file handles or RAID members on production machines; rely on deterministic parser fixtures for fault cases. Real host tests of v0.5.0 are pending.


## v0.5.1 live progress validation

- Automated: CLI parses `--progress` and `--no-progress` with auto-by-default; formatting unit tests verify phase-based 0/50/100%.
- Automated: forcing progress on redirected stderr still produces machine-readable JSON on stdout and progress on stderr; suppressing it produces no progress output.
- Automated: progress thread is joined on normal completion and early errors; the report is not marked 100% complete on write failure.
- Manual pending: confirm smooth animated TTY bar during the potentially slow `pacman -Dk` / `journalctl` phases on Manjaro, Raspberry Pi OS and Debian 13. Avoid interpreting phase percentages as remaining time.


## v0.5.2 slow filesystem collection regression

On Manjaro x86_64, the first v0.5.0 run took noticeably longer than expected and reported `Filesystem usage unavailable`; a subsequent v0.5.0 JSON report successfully listed disks, including `/mnt/onedrive`. The v0.5.1 run took about two seconds and collected local disk/inode data. **The source of the original delay is unconfirmed** because timings were not captured.

Automated tests now cover `df -PT/-Pi --local` with known network-backed FUSE exclusions, explicit `--include-remote-mounts` opt-in, resilient partial `df` parsing, and safe collector timing recording/serialization. Manually compare v0.5.2 default and `--include-remote-mounts` on Manjaro, capturing `collector_timings`, and confirm that normal local disk usage is still reported. Do not deliberately stall remote mounts on production systems.
