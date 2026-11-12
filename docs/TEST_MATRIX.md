# Cross-distribution validation matrix

**Status: mixed manual validations and pending test plan.** CI builds on Ubuntu and checks synthetic journal fixtures. Manjaro was manually tested with released v0.7.0 static-musl and **v0.7.1 source-built code and official v0.7.1 x86_64-musl archive** on 2026-11-12; all-focus JSON, current service state and per-root-port USB grouping were confirmed. On Raspberry Pi OS ARM64 (Debian 12), v0.3.3/v0.3.4 source builds and service diagnostics succeeded; v0.3.4 GNU failed with `GLIBC_2.39 not found`; the v0.3.5 static-musl installation and service diagnostics succeeded on real hardware. Debian 13 (Trixie) x86_64 VM installation and four text diagnostic focuses were manually exercised with v0.3.5 as root and later as the normal user. The **v0.3.6 dpkg diagnostic has now been validated live**, identifying four kernel packages in incomplete states while both systemd managers reported zero failed units. JSON/HTML exports on the VM still require manual checking.

| Platform | Architecture | Manual test status | Targeted checks |
| --- | --- | --- | --- |
| Manjaro | x86_64 | **v0.7.0 and v0.7.1 release binaries + v0.7.1 source tested live (2026-11-12):** x86_64-musl SHA-256 passed; v0.7.0 scanned 5,356 journal entries without truncation. v0.7.1 source build passed 99 tests; official v0.7.1 x86_64-musl archive passed installer SHA-256 verification, version check and live 6,706-entry untruncated JSON diagnosis, confirming active LightDM and 10 USB topology paths on root port `1-1`. | NetworkManager, XFS /var, pacman local DB, reboots |
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


## Manjaro x86_64: v0.5.2 live diagnostics (2026-11-12)

Installer verified `whydown 0.5.2`. A complete `--focus all --current-boot --since 24h --max-events 5000 --progress` run finished in about **2 seconds**; the per-collector timings were systemd system/user 25/25 ms, pacman 1606 ms, dpkg 25 ms, `df` disk 50 ms, `df` inode 50 ms and journalctl 202 ms. A separate `--focus system --format json` run recorded pacman 1354 ms, dpkg 25 ms, both `df` calls 50 ms and journalctl 226 ms. No filesystem collection failure occurred and the known network-backed FUSE mount was explicitly excluded.

Two historical journal warnings were recorded: a kernel network link disconnect and a lightdm.service failure message. At the current snapshot `wlan0` was up, IPv4/IPv6 default routes were present and the system/user failed-unit counts were both zero. These warnings do not verify ongoing incidents. The previously long v0.5.0 run was not timed per collector, so its exact cause remains unknown.

The v0.5.2 log also shows a progress line abutting the start of the report (`elapsed 1sWhyDown 0.5.2`). v0.5.3 adds an explicit progress-renderer pause/join before output and a clean final line after successful completion. Automated ordering checks are included, but manual validation in an interactive terminal is still pending.


## Manjaro x86_64: v0.5.3 live validation (2026-11-12)

The user installed the x86_64 musl binary v0.5.3 after Git fast-forward and SHA-256 verification. The complete `--focus all --current-boot --since 24h --max-events 5000 --progress` report was displayed **without the previous progress/report text collision**. It identified `wlan0` up, both default routes present, zero systemd failed system/user units, healthy pacman database, no incomplete auxiliary dpkg packages, and host CPU/memory/I/O PSI/inode observations.

The two warnings were **historical journal events**, not active-fault proof: one kernel network link loss (interface not attributable from the retained event) and one lightdm.service failure message, with both unit manager snapshots currently at 0 failed units. Collector timings were systemd 25+25ms, pacman 1354ms, dpkg 25ms, df 50+50ms and journalctl 201ms. The manual test confirms the progress-output coordination fix and current snapshot behavior in v0.5.3.

## v0.6.0 correctness regression plan

- Pure journal fixture: `wlan0:` deauthentication and `enp3s0:` kernel link down must attribute each interface; ambiguous messages must stay unknown.
- Resolver fixture: malformed `nameserver` values must not count as valid DNS, whereas IPv4/IPv6/scoped IPv6 addresses count.
- Evidence completeness fixture: if critical journal or capacity collector is unverified and there are no positive findings, use informational `INSUFFICIENT_EVIDENCE`, never a clean-check interpretation. Verified healthy synthetic fixtures retain `NO_CLASSIFIED_ANOMALY`.
- Root mountinfo fixture: distinguish read-only from read-write root without touching mounts; intentional read-only roots get informational findings only.
- Journal fixture: classify explicit file descriptor exhaustion messages and never export raw sensitive log text.
- PSI fixture: high sustained CPU contention should be informational rather than an automatic failure.
- Rendering fixture: text/JSON/HTML include coverage states and accurately label successfully boot-filtered journal incident windows.
- Real v0.6.0 tests on Manjaro, Raspberry Pi OS, Debian and other distributions are **pending**; do not claim validation from v0.5.3 hardware logs.


## Manjaro v0.6.0: different event limits, different incident results (2026-11-12)

The user installed v0.6.0 on Manjaro x86_64 (static-musl, SHA-256 passed). The `--focus all --current-boot --since 24h --max-events 5000` run reported **two historical** observations: `LINK_DISCONNECTS` attributable to `wlan0` and `SERVICE_FAILURE_LOGS` attributable to `lightdm.service`. There were zero current system/user systemd failed units. Collection coverage showed journal, disk and systemd queried and root mount read-write.

The next JSON run used the same scope **but no explicit `--max-events`**, so the previous default of 500 was used. That report had no classified journal incidents and said only the newest 500 journal entries were examined; older events were potentially excluded. Both outputs are consistent with different journal sampling limits. It is **not evidence of spontaneous recovery**. The JSON report also recorded /home 88% and /opt 86% disk utilization, beneath the existing 90% warning threshold.

### v0.6.1 regression and manual validation plan

- Default CLI journal limit 5000; explicit 1–10000 bounds remain unchanged
- N-entry journal is **not** truncated, N+1 entries are truncated; newest N remain eligible for classification, one older sentinel is discarded
- Truncation plus no matched anomalies yields `INSUFFICIENT_EVIDENCE`; truncation plus matched warning retains the warning and emits `JOURNAL_WINDOW_TRUNCATED` info
- Text, JSON, HTML expose the sampling status and number of examined entries; JSON schema remains 2, no raw journal lines retained
- Manually compare text and JSON using identical `--since`, `--current-boot` and `--max-events` arguments on Manjaro after v0.6.1 release. No on-host v0.6.1 validation yet.


## v0.6.1 Manjaro x86_64 confirmed field test (2026-11-12)

The user installed the static-musl v0.6.1 release with successful SHA-256 verification. Text and JSON diagnostics with identical current-boot/all/24-hour/5,000-entry settings agreed on `LINK_DISCONNECTS` (wlan0) and `SERVICE_FAILURE_LOGS` (lightdm). JSON recorded `journal_checked=true`, `journal_truncated=false`, `journal_entries_examined=4598`. The user also supplied local historical kernel and systemd logs: many USB detachments close together (USB hub, Realtek r8152 and DisplayLink), wlan0 deauthentication followed 18s later by association, later deauthentication *by local choice* followed by another association, and a LightDM failure with a systemd restart scheduled shortly afterward. These facts do not prove causality, hardware failure, Internet recovery or application readiness.

## v0.7.0 regression and manual validation plan

- Parse real-style `usb 1-5.1: USB disconnect` lines and r8152 status -71 as USB context, with no retention of serials or raw messages. Four separate bus paths in 30s produce one `USB_DISCONNECT_BURST`; fewer do not.
- Parse `wlan0: associated` after `wlan0: deauthenticated`, report a matched positive association after 18s in warning evidence. A different interface never counts as the same link recovery.
- Treat `wlan0: deauthenticating ... by local choice` followed immediately by association as informational `WIFI_LOCAL_ROAM_OBSERVED`, not an additional link-down warning.
- Recognize `lightdm.service: Scheduled restart job` separately from failure. With a verified current systemd unit snapshot showing no failure, add the informational restart observation but keep the historical failure warning.
- Confirm non-fault UFW BLOCK lines and positive association messages do not become false network incident windows.
- Compare two /proc/net/dev samples without additional delay; report packet-drop deltas in terminal, JSON, HTML; exclude resets and unmatched interfaces. Informational changes require >=10 drops over >=100ms.
- Filesystem advisory at 85–89% is informational, >=90% is still a warning; no disk cleanup is attempted.
- CI tests all outputs automatically. Real Manjaro v0.7.0 and source-built v0.7.1 diagnostics are now validated; Debian 13 and Raspberry Pi OS still require v0.7.x manual tests.

Isolated r8152 -71 messages produce an informational `USB_TRANSPORT_STATUS` rather than a hardware fault warning; <=3 USB detachments or detaches outside the 30-second window do not trigger the burst finding.

## Manjaro x86_64 — v0.7.0 complete-window JSON validation (2026-11-12)

User-provided terminal transcript confirms a successful `git pull --ff-only` to `main` commit `7cde6d1`, installer `--dry-run`, automatic `x86_64-unknown-linux-musl` selection, successful SHA-256 verification, installed `~/.local/bin/whydown` and `whydown 0.7.0` output. No package installation, repair or network probes were made by the diagnostic.

```bash
~/.local/bin/whydown diagnose --focus all --current-boot --since 3h --max-events 10000 --format json --output whydown-v070-live.json --overwrite
```

- Live Manjaro report: kernel `7.2.9-1-MANJARO` from earlier same-day host snapshots; this transcript did not separately repeat the kernel version.
- `journal_entries_examined=5356`, `journal_truncated=false`, diagnostic completed in approximately one second.
- `LINK_DISCONNECTS`: `wlan0` disconnection at `15:02:07Z`, association at `15:02:25Z` (18 s later); no verified Internet probe.
- `WIFI_LOCAL_ROAM_OBSERVED`: local-choice deauthentication and association both at `14:18:57Z`; exact within-second order **cannot be inferred**. A later `15:03:19Z` roam-start and association also co-occur within one second. This motivated the follow-up wording fix.
- `SERVICE_FAILURE_LOGS` and `SERVICE_RESTART_OBSERVED`: `lightdm.service` historical failure and scheduled restart at `14:16:01Z`; no currently failed system units. Application/session readiness was not verified.
- `USB_DISCONNECT_BURST`: 12 distinct USB **topology paths** detached in a 30-second window beginning `15:01:24Z`, with nearby Realtek `r8152` transport status. A hub/dock power-off or cascade is plausible, not established.
- `DISK_CAPACITY_WATCH`: `/home` at 88%, `/opt` at 86% (current df snapshots), both below the 90% warning threshold.
- `interface_drop_deltas`: `wlan0` RX +0, TX +0 over 1,074 ms; a one-second no-change window does not certify ongoing packet-loss absence.
- Network incident grouping reported one historical link incident and no spurious zero-count association-only incidents.

**Validated:** installer, JSON report, journal completeness, observed findings/correlations, sampled interface deltas. **Still pending:** live HTML review, independent cause attribution for USB or LightDM, additional distribution and architecture validation, and broader platform testing. Avoid publishing raw journal identifiers or site-specific details from private logs.

## Additional real kernel/systemd journal validation — 2026-11-12

User-supplied `journalctl -b -k` and `systemctl show` output supports the following, without proving root cause:

- At 16:01:24 CET, USB paths `1-5` and `1-5.1` detached. At 16:01:42–16:01:43 CET, another group of USB paths under `1-1` detached; nearby `r8152` reported interrupt submission status `-71`, and `evdi` reported two DisplayLink card detachments. This is consistent with a possible hub/dock disconnection or reset, not definitive proof of its power state or cause.
- At 16:02:07 CET, `wlan0` reported `PREV_AUTH_NOT_VALID`. It authenticated and associated at 16:02:25 CET (18 seconds later) with a different access-point identifier; this does not by itself prove usable network connectivity. At 16:03:19 CET, local-choice deauthentication and successful association to another access point were printed in order by the original kernel journal within one second; after WhyDown strips fractional timestamps and sorts classified events, it must **not** claim within-second order.
- UFW `BLOCK` entries in the same window include local discovery traffic. They do not establish a DNS or Internet outage and should not be classified as failures purely because `BLOCK` appears.
- LightDM started at 15:15:57 CET, its main process exited with code 1 at 15:16:01 CET, and systemd scheduled and started a restart at 15:16:01 CET. The greeter session opened at 15:16:03 CET and the user's session at 15:16:10 CET. A separate later **current** query returned `ActiveState=active`, `SubState=running`, `Result=success`, `NRestarts=1`. Thus, systemd service operation is verified at the time of that snapshot, although the underlying cause of the initial crash and full application readiness remain unproven.

Under branch-aware grouping, the future output will identify the cascade on `1-1` independently from the two events on `1-5` (rather than treating all 12 topology paths as one root-port incident). The previously released v0.7.0 grouped all 12 events by time only. This branch-aware output has since been validated by the real source-built v0.7.1 diagnostic.

The current unit-state collector has since been manually validated on Manjaro using the newly compiled v0.7.1 source, returning loaded/active/running/success and one LightDM restart in the JSON report. The **published v0.7.1 x86_64-musl binary** has since been validated on real Manjaro hardware. The three other published architecture/libc variants remain untested on their target systems.

## Live Manjaro v0.7.1 source-build validation (2026-11-12)

The user's `cargo test --locked` completed successfully: **88 unit tests** in the library and **11 CLI integration tests** (99 total, no failures). `cargo build --release --locked` succeeded. The binary was then invoked from `./target/release/whydown` with `--focus all --current-boot --since 3h --max-events 10000 --format json --output whydown-validation.json --overwrite`; report creation succeeded in about one second.

A separate `jq` read of the generated report verified `system.service_statuses` for `lightdm.service`: `load_state=loaded`, `active_state=active`, `sub_state=running`, `result=success`, `n_restarts=1`. `SERVICE_RESTART_OBSERVED` had `confidence=observed` and the matching systemd evidence. The `USB_DISCONNECT_BURST` finding reported **10 distinct topology paths under root-port `1-1`** within 30 seconds starting `2026-11-12T15:01:42Z`, and nearby Realtek `r8152` status. Neither fact proves application readiness or the physical cause of USB detachments. The installed v0.7.0 static-musl binary and the source-built v0.7.1 code must not be conflated: the latter was validated here, the former in the prior run.

## Published v0.7.1 archive — final live Manjaro validation (2026-11-12)

The user upgraded the checked-out `main` branch to release commit `d89f117`. The guided installer selected `x86_64-unknown-linux-musl`, downloaded the **published v0.7.1 archive and its SHA-256 file**, verified the checksum successfully and installed the executable under `~/.local/bin/whydown`. `~/.local/bin/whydown --version` returned `whydown 0.7.1`. This is a different verification from the earlier source-built binary tests.

The **installed release executable** completed:

```bash
~/.local/bin/whydown diagnose --focus all --current-boot --since 4h --max-events 10000 --format json --output whydown-v071-final.json --overwrite
```

- Duration approximately one second; report saved successfully.
- JSON `version=0.7.1`, `journal_entries_examined=6706`, `journal_truncated=false`.
- Current `system.service_statuses` returned `lightdm.service` load=loaded, active=active, sub=running, result=success, NRestarts=1.
- Historical `SERVICE_RESTART_OBSERVED` remains an informational finding with `confidence=observed`; its evidence includes systemd's current state, not a claim of application readiness.
- Historical `USB_DISCONNECT_BURST` reported exactly **10 distinct USB topology paths under root-port branch `1-1`** within 30 seconds beginning `2026-11-12T15:01:42Z`, with a nearby Realtek `r8152` USB transport error.

**Validated:** released x86_64-musl artifact, checksum verification, version, JSON output, full journal window, bounded service-status collection and branch-aware USB grouping on Manjaro. **Not validated:** root causes, continuous network reachability, graphical HTML rendering on this host, and the release archives for other architectures/libc variants.
## v0.8.0 interactive terminal automated tests (manual TTY testing pending)

- Unit tests use an in-memory input/output transcript and a fictional demo
  report to exercise menu navigation, evidence details, timeline, coverage,
  scope invalidation and exit behavior without reading the host.
- CLI integration tests assert that a TUI invocation from a pipe fails
  immediately with exit code 2 rather than hanging or consuming system data.
- CLI rejects output formats/paths and warning-driven exit controls in TUI
  mode, preserving the noninteractive JSON, text and HTML behavior.
- The TUI sanitizes control characters in journal-derived identifiers and
  evidence to prevent terminal escape injection.
- Linux x86_64 and ARM64 CI compile/test the same Rust source without new
  third-party crates. Interactive testing on a real Manjaro TTY and over SSH
  is **pending**; no claims of that test are made yet.
