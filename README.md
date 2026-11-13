<p align="center">
  <img src="assets/whydown-logo.svg" alt="WhyDown WD diagnostics logo" width="760">
</p>

# WhyDown

**Linux diagnostics with evidence — without changing your system.**

WhyDown detects common system, network, storage and service incidents from
local Linux state and journals. Examine findings through a desktop GUI, a
guided terminal menu or the command line.

<p align="center">
  <img src="docs/images/whydown-gui-with-logo.png" alt="WhyDown Qt 6 GUI in Dark mode with the official WD logo" width="1050">
</p>

<p align="center"><em>WhyDown Qt 6 desktop on Linux — Dark theme and official WD logo, before a diagnostic run.</em></p>

**Read-only · Offline · No telemetry · GPL-3.0-only**

## Install in 2 minutes

Linux **x86_64 or ARM64** (Manjaro/Arch, Debian, Ubuntu, Raspberry Pi OS and
other compatible distributions). The installer downloads a release binary,
checks its SHA-256 checksum and does not require Rust or `sudo`.

```bash
git clone https://github.com/hamdyaea/WhyDown.git
cd WhyDown
sh scripts/install.sh
~/.local/bin/whydown --version
```

**Want a desktop application menu entry?** Run:

```bash
sh scripts/install.sh --desktop
```

The installer checks optional graphical dependencies and tells you what is
missing. It does **not** install system packages automatically.

### Install `.deb` on Kali, Debian and Ubuntu

Official **v0.12.3** packages (check `dpkg --print-architecture`):
- **Intel/AMD 64-bit:** [amd64 .deb](https://github.com/hamdyaea/WhyDown/releases/download/v0.12.3/whydown_0.12.3-1_amd64.deb) · [SHA-256](https://github.com/hamdyaea/WhyDown/releases/download/v0.12.3/whydown_0.12.3-1_amd64.deb.sha256)
- **ARM 64-bit:** [arm64 .deb](https://github.com/hamdyaea/WhyDown/releases/download/v0.12.3/whydown_0.12.3-1_arm64.deb) · [SHA-256](https://github.com/hamdyaea/WhyDown/releases/download/v0.12.3/whydown_0.12.3-1_arm64.deb.sha256)

```bash
sudo apt update
sudo apt install -y curl ca-certificates python3 python3-pyqt6  # GUI dependencies
curl -fLO https://github.com/hamdyaea/WhyDown/releases/download/v0.12.3/whydown_0.12.3-1_amd64.deb  # replace amd64 with arm64 as needed
curl -fLO https://github.com/hamdyaea/WhyDown/releases/download/v0.12.3/whydown_0.12.3-1_amd64.deb.sha256
sha256sum -c whydown_0.12.3-1_amd64.deb.sha256
sudo apt install -y ./whydown_0.12.3-1_amd64.deb
whydown --version
whydown gui
```

APT resolves dependencies; CLI/TUI need no Qt, while the GUI uses `python3` and `python3-pyqt6`.
See the [package guide](docs/PACKAGING.md); Manjaro/CachyOS/Arch use [AUR recipes](packaging/aur/README.md).

### AUR — Arch Linux, Manjaro and CachyOS

```bash
sudo pacman -S --needed python python-pyqt6  # GUI dependencies
yay -S whydown-bin  # or: paru -S whydown-bin (prebuilt)
# To compile from source instead: yay -S whydown
whydown --version
whydown gui
```

If AUR says `target not found`, clone the [WhyDown repository](https://github.com/hamdyaea/WhyDown)
and run `makepkg -si` in `packaging/aur/whydown-bin/` (requires `base-devel`
and `git`). Install only one variant. Check `type -a whydown` if an old
`~/.local/bin/whydown` shadows the package. [Details](packaging/aur/README.md).

### GUI prerequisites (Qt 6)

The CLI and terminal menu do not require Python or Qt. For the desktop GUI,
install Python 3 and PyQt6 with your Linux package manager:

| Distribution | Install GUI prerequisites |
| --- | --- |
| Manjaro / Arch | `sudo pacman -S python python-pyqt6` |
| Debian / Ubuntu | `sudo apt install python3 python3-pyqt6` |
| Fedora | `sudo dnf install python3 python3-pyqt6` |

Verify before starting: `sh scripts/install.sh --check-gui-prereqs`.

## Launch WhyDown

| Interface | Command | Best for |
| --- | --- | --- |
| **Graphical GUI** | `~/.local/bin/whydown gui` | Desktop, mouse navigation, evidence and timeline |
| **Terminal TUI** | `~/.local/bin/whydown tui` | SSH, servers, guided menus |
| **Command line** | `~/.local/bin/whydown diagnose` | Shell, scripts, automation |
| **Nagios / Icinga** | `~/.local/bin/whydown nagios check --metric all` | Monitoring, thresholds, performance data |

If `~/.local/bin` is in your `PATH`, use `whydown` instead of the full path.

The GUI defaults to **Dark**; use **Appearance** for saved Light/System themes.

**Investigate an alert:** double-click for original application errors and
scoped systemd `ExecMainStatus`/`ExecStart` codes. Diagnostics are read-only;
original journal messages may contain secrets.

## Useful commands

```bash
~/.local/bin/whydown diagnose --focus all --since 4h --current-boot
~/.local/bin/whydown diagnose --focus all --since 4h --current-boot --journal-messages
~/.local/bin/whydown tui
~/.local/bin/whydown gui --demo
~/.local/bin/whydown diagnose --format json --output report.json
~/.local/bin/whydown diagnose --format html --output report.html
~/.local/bin/whydown --help
```

## Nagios / Icinga monitoring and graphs

Standard plugin states, thresholds, performance data and offline SVG/HTML charts:

```bash
whydown nagios check --metric memory --warning 80 --critical 95
whydown nagios check --metric all --record /path/to/private/nagios.tsv
whydown nagios dashboard --history /path/to/private/nagios.tsv --output dashboard.html
```

Create the history directory first. See [Nagios setup](docs/NAGIOS.md).

## Update WhyDown

```bash
cd ~/WhyDown && git pull --ff-only
sh scripts/install.sh --desktop
~/.local/bin/whydown --version
```

Omit `--desktop` on headless systems. Only installation uses the network.

## What it examines

WhyDown identifies historic service failures, network/DNS and disk problems,
resource pressure, and package errors. It distinguishes past events from
current snapshots but does not prove a root cause, repair systems or send logs.

## Documentation

- [Desktop GUI and themes](docs/GUI.md)
- [Terminal TUI](docs/TUI.md)
- [JSON report schema](docs/REPORT_SCHEMA.md)
- [Validation / supported scenarios](docs/TEST_MATRIX.md)
- [Detailed project background & validation history](docs/PROJECT_HISTORY.md)
- [Changelog](CHANGELOG.md) · [Releases](https://github.com/hamdyaea/WhyDown/releases)

## Develop from source

Rust toolchain required; no third-party Rust crates:

```bash
git clone https://github.com/hamdyaea/WhyDown.git && cd WhyDown
cargo test --locked && cargo build --release --locked
./target/release/whydown --version
```

See [CONTRIBUTING.md](CONTRIBUTING.md); never publish sensitive logs.

**License:** [GPL-3.0-only](LICENSE). © 2026 Hamdy Abou El Anein and contributors.
