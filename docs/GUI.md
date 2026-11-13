# WhyDown Qt 6 desktop interface

WhyDown v0.10.0 replaces its experimental Tkinter frontend with a **modern Qt 6
(PyQt6)** graphical interface. The diagnostic engine remains entirely in Rust;
the headless CLI and SSH-friendly TUI remain available without Qt.

## Start

```bash
whydown gui                 # Real local read-only diagnostics
whydown gui --demo          # Fictional results, no host data collected
```

The graphical frontend is embedded into the normal WhyDown Rust executable;
the launcher uses Python 3 and the distribution-provided PyQt6 bindings when
requested. **There is no pip download, automatic package installation, web
browser, network probe, telemetry or privilege escalation.**

Use the installer to check its optional dependencies:

```bash
sh scripts/install.sh --check-gui-prereqs
```

If PyQt6 is missing, install the native package as appropriate:

```bash
# Manjaro / Arch Linux
sudo pacman -S python python-pyqt6

# Debian 12/13 and Ubuntu
sudo apt install python3 python3-pyqt6

# Fedora
sudo dnf install python3 python3-pyqt6
```

The prerequisite check returns nonzero if Python 3 or the PyQt6 QtWidgets
bindings are missing and prints the relevant instruction. It **does not**
install them on the user's behalf. Qt requires an available X11, XWayland or
compatible Wayland graphical session; SSH/headless users can use
`whydown tui`. Python/Tkinter is no longer used by the GUI.

To install a user-owned application-menu entry:

```bash
sh scripts/install.sh --desktop
```

The shortcut is created only when requested, and the installer warns if
optional Qt dependencies are missing. Ordinary binary installations and
terminal commands do not require Python or PyQt6.

## Appearance: System / Light / Dark

Use the **Appearance** menu at the top right of the Qt6 window to choose:

- **System**: use the desktop theme when Qt
  exposes it. On Qt 6.5+ the app observes platform color-scheme updates and
  switches automatically as your desktop changes. On earlier Qt versions,
  the application falls back to the current Qt desktop palette and follows
  palette-change notifications when provided by the platform integration.
- **Light**: always use the clear, light WhyDown layout, independent of OS theme.
- **Dark**: use the full dark palette for the sidebar, settings, metrics,
  tabs, event lists, evidence editors and text selections.

**Dark is the default only on a fresh installation** (when there is no saved
appearance preference). Choosing Light or System explicitly preserves that
choice across restarts, including upgrades; existing settings are never
silently replaced. Invalid saved values safely fall back to Dark.

The change is immediate: no restart or new diagnostic is required. WhyDown
**remembers your explicit choice** using a local Qt `QSettings` preference
(`appearance/theme`, under the user's XDG configuration). No theme choice
is written merely by opening the program, and no configuration is uploaded.

The system setting is best-effort where older Qt or desktop integrations do
not expose live theme-change notifications. You can always choose Light
or Dark manually. GUI appearance does not affect CLI/TUI output, diagnostic
rules, the read-only collector, clipboard behavior or JSON exports.

## Incident overview and double-click investigation (v0.11.0)

After **Run diagnostics**, the first results page is now an **Incident overview**.
Warnings appear before informational observations. Each row shows the finding's
severity, useful title, affected interface/systemd unit when identifiable,
first matching UTC timestamp, and number of relevant events still available
in the retained timeline. Selecting a row displays a short evidence preview.

**Double-click an alert** to open the second page, **Incident details**.
Use **Back to overview** to return; no diagnostic rerun is needed.

The details page shows:

- Diagnostic rule ID, severity, confidence and historical/current scope.
- Every *retained* relevant classified event, in UTC, with the emitting
  program, affected interface/unit, and its safely extracted codes (when
  available). Examples: Wi-Fi deauthentication reason, DHCP timeout,
  `status=1/FAILURE` or systemd unit result.
- The associated finding's actual evidence, time-window context, and
  current local network or systemd snapshot (which may differ from the
  state when the earlier error occurred).
- Suggested **read-only** checks and explicit limitations. It does not
  diagnose a confirmed root cause unless directly established by evidence.

The **Event timeline** also supports double-click to inspect a single event.
All detail text remains mouse-selectable and copyable via Ctrl+C; nothing
is sent to an external service. This is designed to work on CachyOS,
Manjaro, Debian and other Linux distributions with PyQt6.

### Journal privacy and interpretation

WhyDown does **not** store or export raw journal messages, Wi-Fi BSSIDs,
hostnames from log messages, credentials or command arguments. The Rust
collector extracts only short, whitelisted reason/status codes plus existing
bounded unit/interface/source identifiers. If a reason cannot be safely
extracted, WhyDown displays **not available** rather than inventing it.
The report's JSON schema remains v2 with an additive, nullable
`timeline[].diagnostic_hint` field.

For bounded output, the JSON timeline retains up to the latest **300
classified events**, while a detailed incident page displays at most its
latest 80 matching events. Event counts and hours on the overview therefore
refer to retained timeline observations, not a complete system history.
Journal truncation and unverified collectors are shown explicitly.

## Design and use

The app features a dark sidebar, visual warning/observation/event summary
cards, a single **Run diagnostics** action, resizable results, and Qt-styled
navigation. **Focus defaults to `all`**, current-boot evidence, 4-hour
lookback and 10,000 journal entries. Change scope or optional disk/remote
checks before pressing Run diagnostics; re-run when you change settings.

After a diagnostic, findings appear automatically. Click a finding to inspect
its evidence, scope, confidence and suggested read-only checks. Switch
between **Findings & evidence**, **Event timeline**, and **Coverage & limits**.
The backend uses a separate, bounded `QProcess` to run the existing Rust
diagnostic executable asynchronously, without shell interpretation. Output
is capped at 8 MiB and the run times out after 240 seconds.

### Mouse selection and copy: fixed

The finding and coverage panes use **enabled but read-only Qt
`QPlainTextEdit` widgets**. Drag across words or lines with the mouse to
highlight them; the normal native Qt text context menu and **Ctrl+C**
work. The **Copy selection** button copies exactly that highlighted passage
to your local clipboard. There is no entire-report fallback if nothing is
selected. Timeline rows can be selected and copied as tab-separated text.

`Ctrl+A` selects the current pane's text (not the entire diagnostic report).
Selected text may reveal local service names, network interfaces or mountpoint
paths; review it before pasting into a public report.

### Saving a report

An independent **Export JSON** action is enabled after analysis. It requires
explicit file selection, creates private `0600` files on Linux, and never
overwrites existing files. Nothing is saved or transmitted automatically.

## Guarantees and limits

- Read-only diagnostics: no service restarts, package modifications, automatic
  repairs, external DNS probes or telemetry.
- Findings are time-scoped observations, not verified causes. `active/running`
  does not prove a service is application-ready.
- JSON schema v2 and the diagnostic engine are unchanged. GUI, TUI and CLI
  expose the same underlying report model.
- Python/PyQt6 are optional *GUI-only* runtime dependencies. Qt visual behavior
  follows desktop font rendering and may vary between distributions.

## README screenshot

The main README includes a screenshot of the **real Qt6 application running
only fictional demo evidence**, not private host journals. The checked-in
image is at [`docs/images/whydown-gui-dark.jpg`](images/whydown-gui-dark.jpg)
and can be regenerated on a Linux desktop with Qt6 and a virtual X display:

```bash
cargo build --release --locked
xvfb-run -a python3 scripts/capture_gui_for_readme.py ./target/release/whydown
```

The generator always starts with isolated appearance preferences, selects the
first-launch Dark theme and runs `whydown demo`. It does not collect machine
logs or contact the Internet. Only the generated image needs to be committed.

## Automated and on-host testing

```bash
python3 -m unittest discover -s tests -p 'test_gui.py' -v
xvfb-run -a python3 tests/gui_xvfb_smoke.py ./target/release/whydown
```

The Xvfb test opens the real Qt6 GUI and uses **actual mouse input** to select
a word from the evidence pane, asserting that native Ctrl+C and the copy
button place exactly that selection on the clipboard. It also checks
coverage text, timeline rows and empty-selection safety. Normal CI and
release CI both run the test.

After installing the released executable, real-world Manjaro and Debian
desktop compatibility should still be confirmed, particularly on Wayland
compositors and external clipboard managers.

## Official branding (v0.10.3)

WhyDown now uses the selected WD monogram and descending diagnostic graph.
The same PNG is embedded in the Rust binary, loaded into the Qt window
icon and sidebar, and installed in the user's XDG hicolor icon theme
when the desktop launcher is explicitly requested.
The GitHub wordmark is assets/whydown-logo.svg.
Headless usage can display a matching plain ASCII logo with whydown logo.
