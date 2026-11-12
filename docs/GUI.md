# WhyDown Desktop GUI

WhyDown now offers an **optional graphical desktop interface**, while retaining
the fast read-only Rust CLI and the SSH-friendly terminal menu.

## Launch it

Install or update the normal WhyDown binary and then start the GUI:

```bash
whydown gui
# Optional fictional demonstration, without reading local system data:
whydown gui --demo
```

The full Python/Tkinter frontend is **embedded in the Rust release binary**. No
separate Python script download, pip package or new Rust crate is needed. A
local Python 3 interpreter with Tkinter is required **only for the graphical
mode**. Before using the desktop GUI, **the installer automatically checks Python 3
and whether `python3 -c 'import tkinter'` succeeds**. The standalone check
also works without downloading/installing WhyDown:

```bash
sh scripts/install.sh --check-gui-prereqs
```

The check exits 0 when both are available and 1 if either is missing. It does
not start a GUI or test the window system. On Manjaro/Arch, the installer
suggests `sudo pacman -S python tk` when dependencies are missing; on
Debian/Ubuntu it suggests `sudo apt install python3 python3-tk`. It never
runs these commands itself. X11 or XWayland is needed
by Tk on Linux; if there is no graphical display, use `whydown tui`.

To add an **optional application-menu shortcut** when installing:

```bash
sh scripts/install.sh --desktop
```

The new launcher is written only with explicit `--desktop` consent to
`${XDG_DATA_HOME:-$HOME/.local/share}/applications/whydown.desktop`, invokes
the installed binary as `whydown gui`, and uses the system's normal monitoring
icon. The installer does not require root or install Python/Tk on your behalf.
When `--desktop` is used without the GUI prerequisites, it still creates the
requested launcher, but emits a clear warning that the GUI cannot start until
they are supplied. CLI/TUI binaries remain fully functional without either
Python or Tkinter. Normal installation leaves the applications menu unchanged.

## User journey: no number menus required

1. **Start WhyDown GUI.** Focus defaults to **all** (recommended), the journal
   window to **4 hours**, current-boot-only to **enabled**, and the maximum
   journal entries to **10,000**. These are guided-mode defaults only;
   `whydown diagnose` preserves its existing CLI defaults.
2. **Click "Analyze this computer".** WhyDown runs its unchanged local Rust
   diagnostic engine in a worker process, without blocking the graphical
   interface or invoking a shell.
3. **Read the results automatically.** The Findings & evidence tab is opened,
   displaying the findings and an explanation for the first one. Click any
   finding to see evidence, confidence, temporal scope and *suggested checks*.
   The Timeline and Coverage tabs explain the sequence of events and which
   parts of the machine could or could not be verified.
4. Optional: click **Save private JSON report** to export exactly the analyzed
   snapshot. Nothing is automatically written or transmitted.

### Settings

- **Focus:** all (recommended) / network / services / system.
- **Journal lookback:** 1h / 4h / 24h / 7d, plus a validated custom CLI
  lookback if supplied when launching GUI.
- **Journal limit:** 5,000 / 10,000.
- **Current boot:** on by default, off for older accessible journal entries.
- **Advanced opt-in:** inspect large disk directories or include remote mounts.
  These may take longer or block on unavailable remote storage.

Changing settings never silently changes an existing report. Press Analyze
again for a fresh, clearly labeled snapshot. A truncated journal window is
marked **PARTIAL** with guidance to narrow the time window.

## Privacy and safety

- Offline and local: GUI makes no network requests and sends no telemetry.
- No privileged actions, repairs, package installs or service restarts.
- The GUI invokes the same installed Rust `whydown diagnose` process with
  fixed, validated argument values. It does not use `sh -c` or `bash -c`.
- No raw journal message bodies are retained by WhyDown's structured report.
  Local unit names, mountpoints and interfaces may be visible: inspect before
  sharing a saved report.
- File export is **explicit opt-in**. New JSON reports have permissions
  `0600` on Linux and an existing path is never overwritten.
- A finding with confidence "observed" does not prove causality. A systemd
  service that is `active/running` is not guaranteed application-ready and
  a Wi-Fi association does not prove Internet availability.
- GUI is not continuous monitoring; each click collects a new snapshot.
- If Tkinter or a desktop display is unavailable, the CLI and terminal menu
  continue to work without those optional dependencies.

## Testing

Pure GUI model tests run without a display:

```bash
python3 -m unittest discover -s tests -p 'test_gui.py' -v
```

A separate smoke test creates an actual Tk window and drives a fictional
diagnosis under Xvfb on Linux CI:

```bash
xvfb-run -a python3 tests/gui_xvfb_smoke.py ./target/release/whydown
```

The first GUI release still requires confirmation on real Manjaro desktop
hardware. Synthetic and virtual display tests cannot establish compatibility
with every window manager or display server.
