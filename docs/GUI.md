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

- **System** (the default on first launch): use the desktop theme when Qt
  exposes it. On Qt 6.5+ the app observes platform color-scheme updates and
  switches automatically as your desktop changes. On earlier Qt versions,
  the application falls back to the current Qt desktop palette and follows
  palette-change notifications when provided by the platform integration.
- **Light**: always use the clear, light WhyDown layout, independent of OS theme.
- **Dark**: use the full dark palette for the sidebar, settings, metrics,
  tabs, event lists, evidence editors and text selections.

The change is immediate: no restart or new diagnostic is required. WhyDown
**remembers your explicit choice** using a local Qt `QSettings` preference
(`appearance/theme`, under the user's XDG configuration). No theme choice
is written merely by opening the program, and no configuration is uploaded.

The system setting is best-effort where older Qt or desktop integrations do
not expose live theme-change notifications. You can always choose Light
or Dark manually. GUI appearance does not affect CLI/TUI output, diagnostic
rules, the read-only collector, clipboard behavior or JSON exports.

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
