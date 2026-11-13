# WhyDown native packages — Kali, Debian, Ubuntu, Arch, Manjaro, CachyOS

WhyDown v0.12.3 ships verified Debian packages and AUR recipes. The
diagnostic engine is read-only and does not install a service or telemetry.

## Kali Linux / Debian / Ubuntu (.deb)

Choose the architecture reported by `dpkg --print-architecture`.

| CPU architecture | Official release downloads |
| --- | --- |
| Intel/AMD 64-bit (`amd64`) | [whydown_0.12.3-1_amd64.deb](https://github.com/hamdyaea/WhyDown/releases/download/v0.12.3/whydown_0.12.3-1_amd64.deb) · [SHA-256](https://github.com/hamdyaea/WhyDown/releases/download/v0.12.3/whydown_0.12.3-1_amd64.deb.sha256) |
| ARM 64-bit (`arm64`) | [whydown_0.12.3-1_arm64.deb](https://github.com/hamdyaea/WhyDown/releases/download/v0.12.3/whydown_0.12.3-1_arm64.deb) · [SHA-256](https://github.com/hamdyaea/WhyDown/releases/download/v0.12.3/whydown_0.12.3-1_arm64.deb.sha256) |

### Download, verify and install (Bash/Zsh/Fish compatible)

Paste this entire block; it chooses the package matching your CPU:

```bash
bash <<'BASH'
set -euo pipefail
arch="$(dpkg --print-architecture)"
case "$arch" in
  amd64|arm64) ;;
  *) echo "Unsupported Debian package architecture: $arch" >&2; exit 1 ;;
esac
sudo apt update
sudo apt install -y curl ca-certificates python3 python3-pyqt6
base="https://github.com/hamdyaea/WhyDown/releases/download/v0.12.3"
file="whydown_0.12.3-1_${arch}.deb"
curl -fL --retry 3 -O "$base/$file"
curl -fL --retry 3 -O "$base/$file.sha256"
sha256sum -c "$file.sha256"
sudo apt install -y "./$file"
whydown --version
BASH
```

After installation, launch `whydown gui`, `whydown tui`, or
`whydown diagnose --focus all --since 4h --current-boot --journal-messages`.
These commands do not require a Git checkout or a Rust compiler.

### Required and optional dependencies

- **CLI/TUI:** The packaged native static musl executable has no Qt/Python
  dependency. Standard Linux tools such as `journalctl`, `systemctl`, `df`
  and `dpkg-query` are used only when available; inaccessible checks are
  reported explicitly.
- **Qt6 graphical frontend:** `python3` and `python3-pyqt6` from APT are
  needed. These are `Recommends` of the .deb, not strict dependencies, so
  headless installs can use `sudo apt install --no-install-recommends
  ./whydown_0.12.3-1_${arch}.deb` after downloading the correct file.
  A graphical X11/XWayland/Wayland session is needed to display the window.
- **Installer/download:** `curl` and `ca-certificates` are used for
  fetching the assets securely and can be omitted if you already downloaded
  them via a browser. APT resolves native package dependencies automatically.

`/usr/bin/whydown`, its desktop launcher and its icon are installed by the
.deb. This is distinct from the optional unprivileged installer under
`~/.local/bin/whydown`; to check which binary your shell executes, run
`type -a whydown`. If you previously installed a user-local binary, it may
take precedence over the APT package in `PATH`.

For upgrades, install the newer .deb using `sudo apt install ./new-file.deb`.
For removal, run `sudo apt remove whydown`. APT does not auto-add the upstream
GitHub Releases page as an APT repository.

### Building a .deb manually

From a checked-out WhyDown repository, on amd64 or arm64:

```bash
cargo test --all-targets --locked
cargo build --release --locked
bash packaging/deb/build-deb.sh target/release/whydown dist
( cd dist && sha256sum -c ./*.deb.sha256 )
dpkg-deb --info dist/whydown_*.deb
```

The GitHub release workflow builds packages separately for both supported
architectures, using static musl binaries. It checks file integrity, installed
payload and the launcher before publication.

## Arch Linux / Manjaro / CachyOS (AUR)

The same AUR recipes work across Arch-derived distributions:

- [`whydown` — build from tagged source](../packaging/aur/whydown/)
- [`whydown-bin` — prebuilt musl binaries, SHA-256 pinned](../packaging/aur/whydown-bin/)

Install only one variant. The GUI requires `python-pyqt6` (optional dependency
in both PKGBUILDs). Follow the
[SSH publication procedure](../packaging/aur/README.md) to submit the
`PKGBUILD` and generated `.SRCINFO` for each package. AUR itself hosts
recipes, not the release binaries or compiled Arch packages.
