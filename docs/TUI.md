# WhyDown interactive terminal (TUI)

`whydown tui` provides a **read-only, zero-third-party-dependency** interactive menu
to browse the existing WhyDown diagnostic findings. It runs locally without
cloud services, telemetry, privileged commands or network reachability probes.

## Launch

```bash
whydown tui --focus all --since 4h --current-boot
# Or test without collecting host data:
whydown tui --demo
```

Use an interactive Bash, Zsh or SSH terminal. Both stdin and stdout must be
attached to a TTY. Pipelines, CI and scheduled jobs should use `whydown diagnose`
instead. Non-terminal TUI use immediately fails with a clear message and exit 2.

## Navigation improvements in v0.8.1

Always type the selected number and press **Enter**. Unlike the initial
v0.8.0 menu, the dashboard now explains this visibly and confirms changes
to focus, time range, current-boot scope and journal cap.

After option 1 completes a diagnostic, **you can immediately enter option 6**
to inspect findings, 7 for events, or 8 for collection coverage. There is no
extra continue prompt that can absorb the next number.

On a detailed evidence, timeline, coverage or export page, press **Enter** to
return to the dashboard, or type a dashboard option 1–9 followed by Enter to
run that action next. Selecting a changed focus/time scope clears the last
report and requires option 1 to run a fresh diagnostic.

The first-party CI now includes a Linux pseudo-terminal test that exercises
real input/output for these transitions, not just in-memory menu fixtures.

## First use: two-step workflow

The interactive terminal dashboard now leads with **START HERE: 1 = Analyze**.
The default scope is **all** (recommended); the initial journal lookback is
4h on the current boot with the full 10,000-entry cap, unless you explicitly
choose another scope. The existing `whydown diagnose` defaults do not change.

After option 1 finishes, the dashboard immediately lists the newest findings
with their severity. You can read this overview without pressing 6. Option 6
is only for inspecting the explanations, evidence and suggested checks.

On a graphical Linux desktop, `whydown gui` is even simpler: a single
**Analyze this computer** button and findings that appear automatically in a
window. See [desktop guide](GUI.md). The TUI remains useful for SSH and servers
without a graphical display.

## Navigation (number + Enter)

The UI is line-oriented by design: no raw terminal mode, ncurses, third-party Rust
crates or external `stty` call. Select each item using its number and Enter.

- `1`: run the diagnostic (or generate fictional observations in `--demo` mode)
- `2`: cycle focus between all, network, services and system
- `3`: cycle journal lookback between 1h, 4h, 24h and 7d
- `4`: toggle current-boot journal scope
- `5`: toggle maximum journal records between 5,000 and 10,000
- `6`: browse findings; choose a number to inspect its evidence, confidence and checks
- `7`: show the latest 25 classified events
- `8`: inspect collector coverage, current service states and limitations
- `9`: display copyable, private CLI report-export commands
- `0` or `q`: quit without modifying anything

Configuration changes deliberately clear the previous report. Run a new diagnosis
to avoid interpreting findings from an earlier focus or lookback as current data.
The UI does not monitor continuously or refresh without user action.

Live options accepted at launch: `--focus`, `--since`, `--max-events`,
`--current-boot`, `--disk-details` and `--include-remote-mounts`. Remote-mount
and directory details remain opt-in because they may be slow.

Because interactive output cannot also be a machine-readable report, TUI mode
rejects `--format`, `--output`, `--overwrite`, `--exit-on-warning`,
`--progress` and `--no-progress`. Export via the normal CLI outside the TUI.

## Security and interpretation

- The UI never modifies services, files, packages or network configuration.
- Report strings are sanitized of terminal control characters before display.
- It uses classified evidence, not raw journal message bodies.
- Current `active/running` systemd state is not proof of application readiness.
- Wi-Fi association does not establish usable Internet connectivity.
- Historical events are observations, not proof of current outages.
- Local identifiers may still be sensitive; inspect exports before sharing.
- When `TERM` is missing or equals `dumb`, the display avoids ANSI clearing.
- EOF or interruption does not perform repair or create any files.

## Validation

Automated tests cover menu navigation, demo isolation, stale-report invalidation,
terminal-control sanitization, EOF and non-TTY refusal. CI also checks strict
Clippy, release compilation and the existing diagnostic regression suite.

```bash
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
cargo build --release --locked
```

Manual interactive verification on real TTYs and over SSH is still recommended.
