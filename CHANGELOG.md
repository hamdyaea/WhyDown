# Changelog

All notable changes to WhyDown will be documented here.

## [0.12.3] - 2026-11-13

### Portable release checksums
- Publish a separate checksum-pinned `whydown-bin` AUR recipe for x86_64 and aarch64, alongside the source-build `whydown` recipe.
- Fix `.deb.sha256` sidecar files to contain only the archive basename instead of an absolute build-runner path, so `sha256sum -c` works after downloading or moving the packages.
- Add CI guards that validate release checksum portability. No diagnostic behavior or JSON schema changes relative to 0.12.2.

## [0.12.2] - 2026-11-13

### Service failure usability and native distribution packages
- Attribute sentence-final `Failed to start example.service.` records to the correct validated unit; do not print blank diagnostic evidence for unattributed messages.
- Capture and label the numeric `ExecStart` exit status separately from `ExecMainStatus`, without retaining command paths or arguments, including failures of a service's control/start process. JSON schema remains **2**.
- When original journal messages are requested, include relevant application-level fatal/error excerpts from bounded, correlated per-unit journald queries, as introduced after v0.12.1.
- Add verified native Debian packages (`amd64` and `arm64`) for Debian, Kali, and Ubuntu to the automated release workflow, with GUI desktop launcher and icon.
- Add AUR source package recipe for Arch Linux, Manjaro, and CachyOS; update README with native installation and privacy information.

## [0.12.1] - 2026-11-13

### CachyOS systemd user-session incident context
- Fix false association of user-session **transient GUI launchers** with the **system** manager. Bounded read-only historical queries now check the appropriate user/system managers and record the verified manager, with safe fallback and an explicit unknown state when an expired transient unit has disappeared.
- Extract **ExecMainStatus** (numeric only), `Transient` and a boolean desktop-entry marker from trusted `systemctl show` properties. Never store command-line arguments, executable paths, desktop filenames, raw journal messages, or sensitive data.
- Preserve multiple **whitelisted diagnostic indicators** from duplicate same-second journal messages while keeping **one classified event**. E.g., Opera's `code=exited, status=24/n/a` and `Failed with result 'exit-code'` now produce complementary `process_code=exited`, `exit_status=24`, and `systemd_result=exit-code`.
- Identify a **narrowly defined, no-longer-failed** `app-*.service` launcher exit with observed status 24 as an **informational historical desktop-launcher observation**, not an active systemd outage. This is not proof that opening the GUI document succeeded. **Unattributed or other service failure messages remain separate warnings**, and still-failed units or other exit statuses are never downgraded.
- Print retained, sanitized event status codes in CLI timeline; include verified manager, numeric exit status, transient/desktop classification in CLI/HTML/JSON and Qt incident dossiers. Suggest correctly scoped `systemctl --user` / `journalctl --user` commands for user units, with safe identifier validation.
- JSON **schema remains 2**, with additional optional metadata under `system.service_statuses[]`; routine Nagios checks and their machine-readable output are unchanged.

### Validation
- Rust regression fixtures for the exact CachyOS Opera-style same-second log lines, user-manager scope, data privacy, disappearance of transient units, and separation of cleared launcher events from unrelated historical warnings.
- Qt GUI unit tests for matching only relevant historical events, displaying manager-specific states, and providing safe user-manager check commands; original CI and static build targets must still pass.
- This update is CI-tested. A live v0.12.1 post-upgrade scan on CachyOS remains to be verified; the provided CachyOS v0.12.0 logs are the reproduction basis.

## [0.12.0] - 2026-11-13

### Native Nagios/Icinga monitoring and offline trend charts
- Introduce `whydown nagios check` with standard plugin output (OK/WARNING/CRITICAL/UNKNOWN and exit codes 0/1/2/3) and native Nagios/Monitoring Plugins performance data. No CLI banners, JSON, ANSI progress or unrelated output is mixed into the plugin result.
- Provide twelve fast, host-wide metrics: memory/swap usage, local disk and inode capacity, load/core ratio, CPU/memory/I/O PSI, kernel file-handle use, default route presence, configured DNS nameservers and currently failed systemd units. Add a targeted named systemd check (`--metric service --unit UNIT`) that returns 1/0 and CRITICAL when the named unit is inactive. Sensor unavailability produces UNKNOWN instead of false zero.
- Support standard Nagios threshold ranges (including `~:` and `@`), metric-specific WARN/CRIT overrides, bounded subprocesses and a global sampling timeout. Unlike the evidence-oriented diagnostics, routine checks do not scan the journal.
- Add opt-in `--record` to append private (0600) time-stamped metric samples with configured thresholds. No files are written without explicit consent.
- Add `whydown nagios graph` for self-contained SVG time-series plots and `whydown nagios dashboard` for a standalone dark HTML dashboard. No JavaScript, network, external Rust packages or background daemon. Graphs use actual observed timestamps, retain spikes in downsampled histories, distinguish demo data and never invent a trend from one timestamp.
- Supply production Nagios/Icinga command examples, perfdata/PNP4Nagios guidance, defaults, privacy and history retention in `docs/NAGIOS.md`. Keep the README concise.

### Verification
- Rust unit tests for ranges, perfdata, missing metrics, graphic sampling and file safety.
- CLI integration tests for all four exit statuses, demo samples, optional recording, privacy, SVG/HTML exports and no-overwrite guarantees.
- Native host field testing on CachyOS, Manjaro and Debian remains to be confirmed.

## [0.11.0] - 2026-11-13

### Evidence-first incident investigation
- Replace the small findings/evidence split view with an **Incident overview** showing warning-first rows, specific affected interfaces or units when identifiable, first UTC event time and retained event counts. Single-click provides a short evidence preview.
- **Double-click** an alert to open the dedicated **Incident details** page, with Back navigation. Every dossier contains severity, scope/confidence, precise classified event timestamps, source and subject, safely extracted diagnostic codes, finding evidence, current network/systemd snapshots, bounded incident-window context, and suggested read-only checks. Timeline rows support their own double-click dossier.
- Make the first page less generic: name unique affected Wi-Fi/network interfaces and failed systemd units directly in the alert title when confidently attributed.
- Surface representative sanitized indicators and interpretations (such as Wi-Fi reason 2 or systemd exit status 1) in the incident preview/details, and generate target-specific but never automatically executed read-only command suggestions.
- Increase the bounded classified event timeline from 60 to 300 events. Details show up to the newest 80 matching events, and clearly distinguish retained history from a full journal.
- Extract limited, **whitelisted** Wi-Fi disconnect reason codes, service exit statuses/results, DHCP timeouts and known state transitions without retaining free-form journal lines, secrets, SSIDs, MACs or command arguments. Report schema v2 adds optional `timeline[].diagnostic_hint` (null when unavailable).
- Improve the fictional demo to illustrate specific Wi-Fi/DHCP/systemd indicator values. Neither the diagnostic engine nor the GUI probes network hosts, restarts services or exports data automatically.

### Verification
- Unit tests for strict indicator whitelisting, privacy, timestamp and affected-system attribution; GUI model tests for per-finding dossiers and isolation between unrelated incidents.
- Real Qt6/Xvfb test double-clicks a finding, verifies second-page evidence, navigates back, inspects timeline items and repeats selection/clipboard tests.
- Live CachyOS, Manjaro and Debian incident-drilldown testing remains to be confirmed after release.

## [0.10.3] - 2026-11-12

### Official visual identity
- Adopt the selected WD monogram and descending turquoise diagnostic line as the official WhyDown icon, with a transparent PNG derived from the chosen artwork and a scalable SVG wordmark for GitHub.
- Embed the PNG in the no-dependency Rust executable. `whydown icon` exports the bytes, including from static/musl binaries.
- Qt6 now displays the icon in the window and sidebar. `install.sh --desktop` installs the same embedded icon into the user's XDG hicolor icon theme and sets Icon=whydown.
- Add a matching plain-ASCII `whydown logo` command and visual identity to TUI dashboards. Show ASCII branding in human-readable CLI results only on real terminals; piped JSON/HTML, files, scripts and --version are unaffected.
- No changes to read-only collectors, rules, telemetry or JSON schema v2.

### Validation
- Verify PNG signature, ASCII output, JSON/HTML integrity, desktop icon byte-for-byte export and Qt6 window icon loading.
## [0.10.2] - 2026-11-12

### First-run appearance and documentation
- Set **Dark** as the default Qt6 appearance when a user has never stored an appearance preference. Existing Light, Dark and System selections remain unchanged after upgrades; opening the GUI still does not write any preference.
- Keep Light/Dark/System switching, automatic system-theme detection where available, and explicit local `QSettings` persistence intact. Invalid preference values fall back to Dark.
- Replace the previously overloaded, 644-line README with a **142-line quick-start** emphasizing download/installation, desktop setup, Debian/Manjaro Qt requirements, GUI/TUI/CLI launch commands, exports and upgrades.
- Preserve the former technical README and validation narrative in `docs/PROJECT_HISTORY.md`; the short README links to the detailed GUI, TUI, schema and testing guides.
- Add a real, checked-in `docs/images/whydown-gui-dark.jpg` screenshot generated by the actual Qt6 GUI from **fictional demo evidence**, not private host journals. Add a reproducible optional screenshot generator script.
- Ensure GUI help explicitly documents first-run Dark and saved preference behavior. No changes to the diagnostic engine, JSON schema v2, CLI/TUI or private export behavior.

### Verification
- Update pure Python and real Qt6/Xvfb smoke tests to assert Dark on first run, no preference written on launch, and preservation of explicit System/Light/Dark settings.
- Add a README smoke test checking the concise installation/launch examples, committed JPEG signature/size and the presence of the preserved project history.
- Run these tests in both normal CI and release CI; static Linux binaries remain built with zero third-party Rust crates. Screenshot generation reads **demo mode only**.

## [0.10.1] - 2026-11-12

### Qt6 appearance controls
- Add an accessible **Appearance** selector in the Qt6 GUI header: **System** (fresh-install default), **Light**, or **Dark**, without restarting WhyDown or rerunning a diagnostic.
- Apply a full dark palette across the application surface, navigation, settings, summary cards, tabs, results trees, headers, read-only evidence editors, selection colors and progress indicators; retain the existing light theme.
- Persist **only explicit user choices** using local Qt `QSettings` (`appearance/theme`); do not upload preferences or write configuration simply on GUI launch.
- In System mode, use Qt's desktop color-scheme hint when available (Qt >= 6.5) and react to color-scheme/palette change signals. On older Qt versions, fall back to the desktop palette; automatic change notifications are best-effort depending on desktop integration.
- Preserve native mouse text selection, Ctrl+C and exact clipboard copying regardless of theme. CLI/TUI outputs, diagnostic engine and JSON schema v2 are unchanged.

### Tests
- Add headless tests for preference normalization, dark/light overrides and system theme resolution.
- Extend the real Qt6/Xvfb GUI test with theme toggling, local preference persistence across reopening a window, simulated desktop appearance changes and evidence selection/copy regression.
- Keep the release binary matrix, Rust tests, strict Clippy, installer checks and privacy boundaries unchanged. Real Manjaro Wayland/X11 theme-follow testing is still pending.

## [0.10.0] - 2026-11-12

### Qt 6 desktop redesign
- Replace the old Tkinter GUI with a modern, responsive **Qt 6 (PyQt6)** desktop application: dark navigation sidebar, clearly differentiated cards and hierarchy, accessible controls, resizable finding/evidence panels, current observation metrics, and distinct timeline/coverage tabs.
- Resolve the reported inability to select text with a mouse: use active, **read-only QPlainTextEdit** widgets with native mouse selection, Ctrl+C, contextual Copy, select-all and explicit Copy selection. No entire-report copying on empty selection.
- Run the same Rust diagnostic engine asynchronously via bounded, local QProcess with no shell invocation, 8 MiB report limit and 240-second timeout. Keep focus=all/4h/current boot/10,000 as guided defaults.
- Change optional desktop prerequisite from Python/Tkinter to Python **PyQt6**; installer detects the actual QtWidgets import and offers native package names for Manjaro/Arch, Debian 12/13, Ubuntu and Fedora. Never installs packages automatically.
- Keep the CLI, headless TUI, report JSON schema v2, zero-third-party-Rust-crate design, read-only diagnostics and user opt-in private export unchanged.

### Verification
- Add a real Qt 6/Xvfb GUI smoke test exercising a fictional diagnostic, actual *mouse double-click text selection*, exact selection copying via native Ctrl+C and toolbar, coverage and timeline selection, and empty-selection protection.
- Pure Python GUI-model tests, installer preflight tests, Rust unit and integration tests, Clippy and the Linux binary release matrix remain required.
- On-host manual clipboard/Wayland integration on Manjaro and Debian still requires field validation after release.

## [0.9.2] - 2026-11-12

### Desktop usability
- Make detailed findings/evidence and coverage results explicitly selectable, including partial passages. Keep text read-only while preserving selection when focus moves to toolbar/context actions.
- Add `Ctrl+C` and `Ctrl+Insert` copy shortcuts, `Ctrl+A` text selection, right-click menus with **Copy selected text** / **Select all text**, and a visible **Copy selection** button.
- Enable copying selected findings or timeline rows as tab-separated text using the same button, keyboard shortcuts or a row context menu.
- Copy **only** explicitly selected text/rows to the local system clipboard; no background copying, no telemetry, no full report export, and no accidental entire-report fallback for an empty text selection. Expose concise status feedback.
- Extend the real Tk/Xvfb GUI smoke suite to test partial evidence and coverage copying, keyboard handlers, timeline row copies, contextual bindings and empty-selection safety.

### Compatibility
- CLI, TUI, underlying Rust diagnostics, report schema v2 and existing optional Python/Tk GUI dependencies remain unchanged. Manual validation of clipboard integration on Manjaro/X11 or XWayland is still pending.

## [0.9.1] - 2026-11-12

### Installer usability
- Automatically check for a `python3` executable and successful `python3 -c 'import tkinter'` after installation, with clear `GUI dependency OK` / `GUI dependency MISSING` diagnostics. Python/Tk remain **optional** for the CLI and TUI.
- Add `sh scripts/install.sh --check-gui-prereqs`: a standalone read-only preflight that returns 0 when ready and 1 when Python 3 or Tkinter cannot be used. Also show the check on `--dry-run`, without downloading or modifying anything.
- Give installation suggestions for Manjaro/Arch (pacman), Debian/Ubuntu (apt), Fedora (dnf) and a generic fallback, without executing any package manager or using sudo.
- When `--desktop` is requested but Python/Tkinter are absent, keep the explicit user-owned launcher and warn that it will not start until the dependencies are installed. The normal CLI/TUI installation still succeeds.

### Validation
- Add offline installer smoke tests for Python missing, a Python interpreter with Tkinter import failing, a successful import, and non-mutating `--dry-run` including `--desktop`.
- Run those cases in the existing installer smoke suite for both normal CI and release workflow. No third-party Rust dependencies or diagnostic behavior changes.

## [0.9.0] - 2026-11-12

### Desktop GUI
- New optional **`whydown gui`** command launches a full offline desktop UI using embedded Python 3 / Tkinter, without new Rust crates or a separate GUI download. The normal Rust CLI and remote/headless TUI remain available without Python/Tk.
- Guided default settings for GUI and TUI: `focus=all`, four-hour current-boot journal window and 10,000 entries; the normal `whydown diagnose` defaults are unchanged. Explicit lookback/focus arguments are preserved.
- The desktop starts with a prominent **Analyze this computer** button and automatically displays classified findings with explanations, evidence, confidence and suggested checks when collection finishes. Separate tabs show the classified timeline and collector coverage/limitations.
- GUI runs the same read-only Rust collector without shell invocation on a background worker; shows a clear progress indicator while collecting and does not block the desktop. `whydown gui --demo` uses fictional evidence.
- Optional read-only disk breakdown and remote mount checks are available as opt-in checkboxes; reports can be saved with explicit user action to a private JSON file (0600, no overwrite). No automatic exports or telemetry.
- Installer option `sh scripts/install.sh --desktop` creates a user-owned XDG application launcher explicitly, with no sudo or automatic Python/Tk installation.

### Terminal usability
- TUI dashboard now says **START HERE: 1 = analyze** and clearly explains that `6` opens the evidence only after a diagnostic.
- Once a run completes, a brief list of findings is visible immediately on the dashboard; the user no longer needs to navigate into a separate menu merely to discover what was found.

### Tests and compatibility
- Added pure Python GUI model tests for JSON-v2 parsing, data scope and read-only command construction. Added a real GUI smoke test under an Xvfb virtual display, opening Tk and running a fictional diagnostic; these tests run in normal and release CI.
- Existing Rust TUI PTY smoke checks updated for the new interactive 10,000-entry default; installer smoke checks the optional XDG shortcut using an isolated `XDG_DATA_HOME`.
- Existing CLI JSON schema v2 and rules remain unchanged. Python/Tk/X11 or XWayland are **optional runtime requirements only for the GUI**; no new external network probes, privileged changes or automatic repair.
- Real Manjaro desktop GUI validation remains pending, independently of Linux CI and virtual-display smoke tests.

## [0.8.1] - 2026-11-12

### Fixed
- Make the number-plus-Enter interaction explicit at the TUI dashboard and show visible status feedback after changing focus, lookback, current-boot filter or journal cap.
- Remove the extra pause after a completed interactive diagnostic: the next number (especially 6, 7 or 8) is handled directly instead of disappearing.
- From a finding/detail, timeline, coverage or export page, forward menu numbers 1–9 typed at the return prompt to the dashboard, rather than silently consuming them. Pressing Enter alone returns normally.
- Clearly explain why changing settings resets the previous report and that option 1 is required to recollect. Offer suitable guidance for a partial journal window depending on whether the cap is 5,000 or 10,000.

### Tests
- Reproduce the previously swallowed post-diagnostic 6 and detail-to-timeline 7 in Rust regression tests.
- Add a real Linux pseudo-terminal (PTY) smoke test using only the Python standard library to drive `tui --demo` end-to-end through actual terminal input/output.
- Run that PTY test in both normal Rust CI and the release workflow before publication. No new third-party Rust dependency, network probe or privileged operation.
- User-reported v0.8.0 terminal navigation issue motivates the fix; live Manjaro validation of the published v0.8.1 executable is still pending.

## [0.8.0] - 2026-11-12

### Added
- Introduce a read-only interactive terminal mode (`whydown tui`) with a number-and-Enter menu for focus/lookback/current-boot configuration, per-finding evidence and suggested checks, classified timeline, collection limitations and copyable report commands.
- `whydown tui --demo` uses the existing fictional data fixture without reading local system information.
- Require interactive stdin and stdout for TUI use, rejecting pipelines and automation, as well as incompatible export/monitoring flags. No raw terminal mode, new dependencies or external `stty` invocation.
- Sanitize terminal control characters in evidence strings before display; invalidate stale reports after changing scope.
- Add TUI unit tests and integration tests for interactive navigation, demonstration isolation, EOF, unsafe output combinations and non-terminal execution.

### Validation
- Local and GitHub CI tests cover interactive navigation, evidence browsing, fictional-mode isolation, EOF, stale-report invalidation, terminal control character sanitization, and non-TTY safety. Strict Clippy with `-D warnings` and optimized release build passed on Linux CI before release.
- Manual validation of the published v0.8.0 executable on a real Manjaro interactive terminal remains pending; the automated tests do not replace it.

### Compatibility
- Preserve normal `whydown diagnose` / `demo` text, JSON v2 and HTML output, exit codes, read-only behavior and all v0.7.1 diagnostics.
- The TUI presents existing evidence only. An active service is not proof of application readiness; there are no active network probes and no automatic exports.


## [0.7.1] - 2026-11-12

### Added
- For up to four recognized systemd **system** services with historical failure journal messages, query a current read-only `systemctl show` snapshot (3-second timeout and 8 KiB output cap per unit). Record validated `LoadState`, `ActiveState`, `SubState`, `Result` and `NRestarts` in additive `system.service_statuses` JSON, text and HTML. These observations supplement restart evidence without replacing historical warnings or implying application readiness.
- Validate bounded unit identifiers before passing them as subprocess arguments; unavailable, user-only or missing system units remain unverified, not healthy.
- Regression tests for a real-style LightDM `ActiveState=active`, `SubState=running`, `Result=success`, `NRestarts=1` snapshot and all report formats.
- Group USB disconnect bursts by **root-port topology branch** as well as the existing 30-second window, avoiding false aggregation of unrelated detaches from separate USB root ports. Regression case: two paths on `1-5` plus two on `1-1` are not one burst.

### Fixed
- Avoid asserting that a Wi-Fi association happened *after* local-choice deauthentication or link loss when both are retained in the same whole-second journal bucket; co-occurrence is still reported, with event order explicitly unknown. Preserve the validated 18-second reassociation evidence when seconds differ.
- Label USB disconnect bursts as **topology-path detachments** rather than a count of distinct devices or independent faults.

### Field validation
- Manjaro x86_64, kernel `7.2.9-1-MANJARO`, live v0.7.0 static-musl installer and SHA-256 passed on 2026-11-12. `--focus all --current-boot --since 3h --max-events 10000` finished in about one second; 5,356 entries were examined without journal truncation. Confirmed 18-second `wlan0` reassociation after a link event, LightDM restart scheduling, 12 USB topology paths detached in a burst with nearby `r8152` status, 0 RX/TX drop increments in 1,074 ms, and 88% `/home` / 86% `/opt` capacity advisories. This validates detection on the supplied live run, **not** a verified root cause or recovery.
- Real Manjaro v0.7.1 **source-build** validation: `cargo test --locked` passed 88 unit tests and 11 CLI integration tests (99 total), and `cargo build --release --locked` completed. A subsequent all-focus diagnosis finished in approximately one second and exported the JSON-v2 `system.service_statuses` snapshot: `lightdm.service` load=loaded, active=active, sub=running, result=success, NRestarts=1. The USB burst contained **10 topology paths only on root branch `1-1`** with nearby r8152 transport status, excluding unrelated detaches on branch `1-5`. This validates source code functionality on Manjaro, not a v0.7.1 release archive or a proven USB hardware cause.
- Real Manjaro v0.7.1 **published archive** validation: installed `whydown-v0.7.1-x86_64-unknown-linux-musl.tar.gz` via the guided installer with SHA-256 verification, `whydown 0.7.1` confirmed. The installed binary scanned 6,706 journal entries for `--focus all --current-boot --since 4h --max-events 10000` without truncation in about one second. JSON v2 verified `lightdm.service` load=loaded, active=active, sub=running, result=success, NRestarts=1. `SERVICE_RESTART_OBSERVED` remained informational and `USB_DISCONNECT_BURST` reported 10 topology paths on branch `1-1` with nearby r8152 transport status. The published release is therefore validated on Manjaro x86_64-musl, but the initial service failure cause, hardware causes and application-level readiness are not proven.

## [0.7.0] - 2026-11-12

### Added
- Historical Wi-Fi reassociation on the **same** interface can now be correlated with a preceding link loss (up to 120s). The original warning remains, with additional evidence of association and no claim that Internet access was restored.
- Local-choice deauthentication followed by association within 30s is `WIFI_LOCAL_ROAM_OBSERVED` (informational), not a hard link-down warning. Plain Wi-Fi associations are positive context rather than standalone failures or zero-count incident windows.
- Explicit systemd `Scheduled restart job` messages are retained as bounded service-restart events. Historical service-failure findings note a correlated restart and current failed-unit state; `SERVICE_RESTART_OBSERVED` is informational and does not certify service readiness.
- `USB_TRANSPORT_STATUS` records isolated Realtek r8152 status -71 events as informational, without implying defective hardware.
- `USB_DISCONNECT_BURST` (warning) groups four or more distinct short USB topology paths detached within 30s, potentially an unplugged hub/dock. Nearby r8152 status errors enrich evidence without claiming hardware failure or causation of network events.
- Two read-only /proc/net/dev samples per network run record per-interface RX/TX dropped-counter deltas and duration. Informational `RECENT_INTERFACE_DROPS` requires >=10 new drops over >=100ms. Counter resets are excluded; cumulative counters remain separate.
- `DISK_CAPACITY_WATCH` informational advisory at 85–89% filesystem occupancy. `DISK_NEAR_FULL` warning at >=90% is unchanged.

### Tests and compatibility
- New Rust regression fixtures based on real Manjaro USB detach / Wi-Fi reassociation / local roaming / LightDM restart cases and clean UFW-blocked multicast noise.
- Unit tests for matching interface correlation, counters that reset, USB grouping and disk threshold boundaries. JSON schema v2 additive field `system.interface_drop_deltas`; terminal and HTML also expose measured deltas.
- No active connectivity probes, package changes, privilege escalation, telemetry, network calls or external Rust dependencies. These are evidence-based observations; hardware fault and service recovery are not automatically verified.
- Real v0.6.1 Manjaro validation: 4,598 current-boot journal lines scanned with no truncation, and two consistent historical findings in text/JSON. v0.7.0 field test remains pending.

## [0.6.1] - 2026-11-12

### Fixed
- Increase the default maximum journal tail from 500 to 5,000 (CLI still allows 1–10,000) to reduce missed historical incidents in busy journals.
- Request `--lines=N+1` from journalctl and analyze only the newest N. The extra entry acts as a **sentinel**, allowing exact distinction between an N-entry window and actual truncation.
- Record `journal_truncated` and `journal_entries_examined` in additive JSON schema v2 fields. Terminal and HTML reports explicitly mark truncated journal coverage as **PARTIAL**.
- When no diagnostic rule matches but journal entries were excluded by the limit, emit informational `INSUFFICIENT_EVIDENCE` instead of `NO_CLASSIFIED_ANOMALY`. When other findings exist, retain them and add informational `JOURNAL_WINDOW_TRUNCATED` to warn that older incidents may also exist.
- Add deterministic regression tests for N vs N+1 journal lines, quiet journal, limited journal with and without classified findings, CLI defaults, and all report formats.

### Manjaro v0.6.0 field evidence
- Installation of the x86_64-musl package with SHA-256 validation passed and interactive progress completed cleanly. A `--max-events 5000` current-boot report on kernel `7.2.9-1-MANJARO` correctly attributed one historical WLAN disconnect to `wlan0`, identified one historical lightdm unit failure, and observed zero currently failed system and user units. Root mount was read-write.
- A JSON invocation seconds later omitted `--max-events`, so v0.6.0 used its **500-line default**. The JSON reported `NO_CLASSIFIED_ANOMALY` and explicitly noted that older journal entries might be missing. This result **does not contradict** the longer scan.
- v0.6.1 journal-sampling behavior still requires a new on-host test. No claim is made that either earlier journal warning represents an ongoing failure.

### Compatibility
- No privilege escalation, package changes, network probes, telemetry or raw log exports. `schema_version` remains 2 with additive fields. Report findings can now include an informational coverage notice.

## [0.6.0] - 2026-11-12

### Diagnostic correctness
- Fix an unreachable network-interface extraction branch for kernel messages such as `wlan0:` / `enp3s0:`: the original code removed a trailing colon before checking whether it was present. Regression fixtures confirm distinct interfaces are attributed without preserving raw journal text.
- Validate `nameserver` addresses in resolv.conf as IPv4/IPv6 literals rather than counting arbitrary text as configured DNS. Scoped IPv6 link-local addresses are accepted.
- Explicitly track successful journal collection and filesystem usage collection; when **no anomaly matches but core sources are unavailable**, emit informational `INSUFFICIENT_EVIDENCE` instead of `NO_CLASSIFIED_ANOMALY`. A new COLLECTION COVERAGE section in text/HTML and additive JSON-v2 fields show what's been checked.
- When `--current-boot` is successfully applied by journalctl, label historical incident windows `current_boot_filtered` rather than an approximate wall-clock/uptime-derived relation.

### New read-only signals
- Inspect root mount `ro` / `rw` flags from the visible `/proc/self/mountinfo`; a read-only root is **informational**, because immutable operating systems or containers can intentionally use it.
- Classify explicit file descriptor exhaustion messages such as `Too many open files` and `file table overflow` as historical `FD_EXHAUSTION_LOGS` warnings, independently of the global file-nr limit. No raw message bodies retained.
- Informational `CPU_CONTENTION_PRESSURE` for extremely high sustained CPU PSI some avg60 (>=90%); intentional compute-intensive work is not automatically faulty.

### Tests and platform evidence
- Add parser, engine and report rendering regressions for network attribution, invalid/IPv6 DNS, incomplete evidence, read-only root, file descriptors, CPU PSI and exact boot filtering.
- Manjaro v0.5.3 field evidence: static-musl binary ran and progress completed cleanly; pacman database consistent, 0 currently failed system/user units, two historical journal warnings, and per-collector timings around 1.3s pacman / 0.2s journal. This does not confirm v0.6.0 on-host behavior.

### Compatibility and limitations
- Read-only/offline operation, no privileged changes or network probes, no new third-party Rust runtime dependencies. JSON `schema_version` remains 2 with additive fields.
- These changes increase accuracy but do not cover every distro, service framework, hardware fault or application-specific failure.

## [0.5.3] - 2026-11-12

### Fixed
- Serialize terminal presentation correctly: pause and join the live progress redraw thread before writing a text/JSON/HTML report to stdout or printing a saved-report notification. Clear the transient terminal progress line so it never collides with `WhyDown 0.5.2` at report start.
- Emit the final 100% completion message only after successful report output. On an output error, retain an interrupted progress status; do not duplicate completion output when the progress object is dropped.
- Add unit and CLI integration regressions for pause/resume-to-completion, duplicate progress lines, saved-file notifications and clean JSON stdout.

### Field validation recorded
- Manjaro x86_64 v0.5.2: full `--focus all --current-boot --max-events 5000` run reported completion in about 2 seconds, with `pacman -Dk` 1606 ms, dpkg 25 ms, `df` 50 ms for each disk/inode query, and journalctl 202 ms. A second system-focused run reported pacman 1354 ms and journalctl 226 ms. Both pacman and dpkg checks passed. This confirms a fast recent local run, **not the exact cause of the earlier v0.5.0 delay**.
- Historical observations: one kernel link disconnect and one lightdm service-failure message in the window, with wlan0 up and zero currently failed system/user systemd units. Neither warning proves an active fault or a root cause.
- v0.5.3 progress ordering remains to be manually verified in the user's interactive Manjaro terminal after publication.

## [0.5.2] - 2026-11-12

### Fixed
- Make filesystem and inode collection local-only by default with GNU df `--local` and exclusions for known network-backed FUSE mounts (including `fuse.rclone`), reducing stalls on unstable OneDrive/rclone, SSHFS and related mounts. Remote mount scanning is opt-in via `--include-remote-mounts`.
- Retain valid mount entries when `df` exits nonzero but produces partial output. Explicitly mark partial/failed disk or inode queries and preserve real errors in collection notes rather than discarding all disk data with a generic message.
- Add read-only per-collector timing capture for systemd, pacman, dpkg, df, du and journalctl. Report elapsed wall-clock milliseconds and success/failure in terminal, JSON schema v2 (additive `collector_timings`) and HTML.
- Add tests for local/remote df option selection, mountinfo remote type detection, timing records, and report rendering.

### Verification scope
- A real Manjaro v0.5.0 run was reported very slow and once said `Filesystem usage unavailable` but a later invocation collected disk data. A remote/FUSE delay is plausible, **not confirmed**; previously there were no individual collector timings.
- No repair, package change, network probe, telemetry or new Rust third-party dependency. Excluding remote mounts intentionally limits disk coverage unless explicitly opted in.

## [0.5.1] - 2026-11-12

### Added
- Live, continuously refreshed progress bar for slow diagnostic runs, automatically shown when stderr is an interactive terminal. Displays completed phases, progress percentage, active collector command and elapsed time, including during potentially slow pacman, dpkg, df, systemd and journalctl checks.
- `--progress` to force progress in noninteractive sessions (newline-based stderr without ANSI control characters), and `--no-progress` to disable it. The default is auto: interactive stderr only.
- Progress is exclusively written to stderr; stdout text, HTML and JSON and saved report files are unchanged. A failed report write does not falsely mark the diagnostic 100% successful.
- Dedicated progress formatter/worker tests and integration assertions for JSON purity, forced progress and suppressed progress.

### Compatibility
- Percentage measures *completed diagnostic phases*, never a predicted remaining time or percentage of files/bytes scanned. Existing collector timeouts, read-only behavior, default CLI result codes and JSON schema v2 remain unchanged.
- The no-progress library collector API remains available.

## [0.5.0] - 2026-11-12

### Added
- Cross-distribution read-only Linux host sensor module: memory available/swap from procfs, CPU/memory/I/O PSI 60-second averages, kernel-wide file-handle allocation, inode utilization and Linux MD software RAID member counts.
- Conservative warning IDs: `LOW_MEMORY_HEADROOM`, `MEMORY_STALL_PRESSURE`, `IO_STALL_PRESSURE`, `FILE_HANDLE_LIMIT_NEAR`, `INODE_NEAR_FULL`, `MD_RAID_DEGRADED`. All are current snapshots and do not assert verified root cause.
- Bounded procfs input reads (128 KiB max per source), bounded inodes and array lists, no raw process information or disk member paths. Sensors not exposed by a kernel remain absent rather than being reported healthy.
- Cross-format text, self-contained HTML and additive JSON-v2 host resource metrics; PSI averages are encoded as integer hundredths of a percent.
- Regression fixtures for healthy/degraded MD arrays, no-inode/immutable mounts, malformed file counts, memory headroom and low/high PSI.
- Manjaro manual validation update: WhyDown v0.4.1 pacman -Dk matched command-line pacman and the binary installation passed checksums.

### Limitations
- The new rules do not cover every failure mode or replace SMART, application, GPU, Btrfs/ZFS, hardware RAID or active network tests. Container-specific quotas can differ from host readings. Real-world tests of v0.5.0 on the target distributions remain pending.

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
