#!/bin/sh
# WhyDown: install prebuilt Linux binaries (default), or build locally.
# No automatic system package installation, sudo or service changes.
set -eu
VERSION="${WHYDOWN_VERSION:-v0.12.1}"
PREFIX="${WHYDOWN_PREFIX:-${HOME:-}/.local}"
LIBC=auto
MODE=binary
DRY_RUN=0
DESKTOP=0
say() { printf '%s\n' "$*"; }
fail() { printf 'WhyDown installer: %s\n' "$*" >&2; exit 1; }
usage() {
cat <<'HELP'
WhyDown Linux installer (x86_64 and aarch64)
  sh scripts/install.sh [OPTIONS]
  (no options)           Download verified static musl binary; Rust is NOT required
  --from-source          Build checked-out repository using Rust/Cargo
  --check-build-prereqs  Explain missing compiler/Cargo prerequisites
  --check-gui-prereqs    Verify optional Python 3 and PyQt6 (exit 1 if missing)
  --print-target         Display detected target without installing
  --dry-run              Show selected binary without downloading
  --desktop              Install an optional local application-menu launcher
  --libc auto|gnu|musl   Auto selects static musl; gnu is opt-in
  --prefix DIR           Install to DIR/bin (default ~/.local/bin)
  --help                 Display this help
Set WHYDOWN_VERSION=vX.Y.Z to select a different published release.
This installer never invokes sudo or installs a compiler automatically.
HELP
}
while [ "$#" -gt 0 ]; do
    case "$1" in
        --from-source) MODE=source; shift ;;
        --check-build-prereqs) MODE=check; shift ;;
        --check-gui-prereqs) MODE=gui-check; shift ;;
        --print-target) MODE=target; shift ;;
        --dry-run) DRY_RUN=1; shift ;;
        --desktop) DESKTOP=1; shift ;;
        --libc|--prefix)
            [ "$#" -ge 2 ] || fail "Missing value for $1"
            if [ "$1" = --libc ]; then LIBC="$2"; else PREFIX="$2"; fi
            shift 2 ;;
        -h|--help) usage; exit 0 ;;
        *) fail "Unknown option $1; use --help" ;;
    esac
done
case "$LIBC" in auto|gnu|musl) ;; *) fail "Invalid libc: use auto, gnu or musl" ;; esac
case "$VERSION" in v[0-9]*.[0-9]*.[0-9]*) ;; *) fail "Invalid WHYDOWN_VERSION: expected vX.Y.Z" ;; esac
[ "$(uname -s)" = Linux ] || fail "Only Linux is supported"
source_env() {
    if [ -f "${HOME:-/nonexistent}/.cargo/env" ]; then
        . "$HOME/.cargo/env"
    fi
}
prereqs() {
    source_env
    missing=0
    for tool in rustc cargo cc; do
        if command -v "$tool" >/dev/null 2>&1; then
            say "OK: $tool ($(command -v "$tool"))"
        else
            say "MISSING: $tool" >&2
            missing=1
        fi
    done
    if [ "$missing" -ne 0 ]; then
cat >&2 <<'GUIDE'
For source builds on Debian, Raspberry Pi OS or Ubuntu, install prerequisites:
  sudo apt update
  sudo apt install build-essential curl git ca-certificates
Install recent Rust and Cargo using the official instructions at https://rustup.rs/
Then reopen your terminal or reload the PATH with:
  . "$HOME/.cargo/env"
  rustc --version
  cargo --version
Note: older distribution rustc packages may be below WhyDown's minimum 1.70.
For ordinary use, run this installer WITHOUT --from-source. No Cargo needed.
GUIDE
        return 1
    fi
}
if [ "$MODE" = check ]; then prereqs; exit $?; fi

# Qt6 GUI dependencies are optional for the Rust CLI/TUI. Check whether Python 3
# can actually import PyQt6.QtWidgets; finding the executable alone is not enough.
# No display server is needed for this preflight; no packages are installed.
gui_prereqs() {
    missing=0
    if command -v python3 >/dev/null 2>&1; then
        version=$(python3 --version 2>&1 || :)
        say "GUI dependency OK: Python 3 available (${version:-version unavailable})"
        if python3 -c 'from PyQt6.QtWidgets import QApplication' >/dev/null 2>&1; then
            say "GUI dependency OK: PyQt6 QtWidgets import succeeded"
        else
            say "GUI dependency MISSING: PyQt6 QtWidgets could not be imported by python3"
            missing=1
        fi
    else
        say "GUI dependency MISSING: python3 is not on PATH"
        say "GUI dependency MISSING: PyQt6 cannot be checked without Python 3"
        missing=1
    fi
    if [ "$missing" -ne 0 ]; then
        say "WhyDown CLI and TUI remain fully usable without Python/PyQt6."
        if command -v pacman >/dev/null 2>&1; then
            say "Manjaro / Arch: sudo pacman -S python python-pyqt6"
        elif command -v apt-get >/dev/null 2>&1; then
            say "Debian / Ubuntu: sudo apt install python3 python3-pyqt6"
        elif command -v dnf >/dev/null 2>&1; then
            say "Fedora: sudo dnf install python3 python3-pyqt6"
        else
            say "Install Python 3 and your distribution's Qt6 PyQt6 package to enable the GUI."
        fi
        say "No packages were installed or changed."
        return 1
    fi
    say "GUI dependencies available (a graphical X11/XWayland session is still required)."
    return 0
}
if [ "$MODE" = gui-check ]; then
    gui_prereqs
    exit $?
fi
target() {
    case "$(uname -m)" in
        x86_64|amd64) cpu=x86_64 ;;
        aarch64|arm64) cpu=aarch64 ;;
        *) fail "Unsupported CPU architecture: $(uname -m)" ;;
    esac
    kind="$LIBC"
    # Use a statically linked musl release by default, even on glibc hosts.
    # GNU release binaries built on Ubuntu 24.04 may need newer glibc than
    # Debian 12 / Raspberry Pi OS provides.
    if [ "$kind" = auto ]; then kind=musl; fi
    printf '%s-unknown-linux-%s\n' "$cpu" "$kind"
}
if [ "$MODE" = target ]; then target; exit 0; fi
case "$PREFIX" in /*) ;; *) fail "--prefix must be an absolute path" ;; esac
if [ "$DESKTOP" -eq 1 ]; then
    [ -n "${HOME:-}" ] || fail "--desktop requires a home directory"
    # .desktop Exec does not accept arbitrary shell text; reject unusual paths
    # instead of interpolating untrusted characters into an application entry.
    case "$PREFIX/bin/whydown" in
        *[!A-Za-z0-9_./-]*) fail "--desktop requires a prefix without spaces/special characters" ;;
    esac
fi
if [ "$MODE" = source ]; then
    repo_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd -P)
    [ -f "$repo_dir/Cargo.toml" ] ||
        fail "--from-source requires the complete WhyDown Git checkout"
    prereqs || exit 1
    if [ "$DRY_RUN" -eq 1 ]; then
        say "Dry run: source build at $repo_dir"
        gui_prereqs || :
        exit 0
    fi
    (cd "$repo_dir" && cargo build --release --locked)
    # Validate a staging executable before replacing a working installation.
    mkdir -p "$PREFIX/bin"
    staged=$(mktemp "$PREFIX/bin/.whydown-stage.XXXXXXXX") ||
        fail "Cannot stage executable in $PREFIX/bin"
    trap 'rm -f -- "$staged"' 0
    install -m 755 "$repo_dir/target/release/whydown" "$staged"
    "$staged" --version ||
        fail "Compiled binary cannot run; existing installation preserved"
    mv -f -- "$staged" "$PREFIX/bin/whydown"
    staged=""
else
    release_target=$(target)
    archive="whydown-${VERSION}-${release_target}.tar.gz"
    url="https://github.com/hamdyaea/WhyDown/releases/download/${VERSION}/${archive}"
    say "Selected target: $release_target"
    say "Download: $url"
    say "Install: $PREFIX/bin/whydown"
    if [ "$DRY_RUN" -eq 1 ]; then
        say "Dry run: nothing downloaded or changed."
        gui_prereqs || :
        exit 0
    fi
    for tool in curl sha256sum tar mktemp install mv chmod; do
        command -v "$tool" >/dev/null 2>&1 ||
            fail "Missing $tool; on Debian/Raspberry Pi OS install curl coreutils tar"
    done
    tmp=$(mktemp -d "${TMPDIR:-/tmp}/whydown-install.XXXXXXXX") ||
        fail "Cannot create temporary directory"
    staged=""
    trap '[ -z "$staged" ] || rm -f -- "$staged"; rm -rf -- "$tmp"' 0
    trap 'exit 130' 1 2 3 15
    curl -fL --retry 3 -o "$tmp/$archive" "$url"
    curl -fL --retry 3 -o "$tmp/$archive.sha256" "$url.sha256"
    (cd "$tmp" && sha256sum -c "$archive.sha256") ||
        fail "SHA-256 verification failed"
    tar -xOzf "$tmp/$archive" whydown > "$tmp/whydown" ||
        fail "Cannot extract whydown executable"
    # Verify the downloaded candidate can actually start on this host, then
    # atomically replace the installed executable; never clobber on failure.
    chmod 755 "$tmp/whydown"
    if ! reported=$("$tmp/whydown" --version 2>&1); then
        say "$reported" >&2
        fail "Downloaded binary cannot run. Try --libc musl (recommended) or --from-source; existing installation preserved."
    fi
    expected="whydown ${VERSION#v}"
    [ "$reported" = "$expected" ] ||
        fail "Unexpected binary version ($reported, expected $expected); existing installation preserved"
    mkdir -p "$PREFIX/bin"
    staged=$(mktemp "$PREFIX/bin/.whydown-stage.XXXXXXXX") ||
        fail "Cannot stage executable in $PREFIX/bin"
    install -m 755 "$tmp/whydown" "$staged"
    mv -f -- "$staged" "$PREFIX/bin/whydown"
    staged=""
fi
"$PREFIX/bin/whydown" --version
say "Installed: $PREFIX/bin/whydown"
case ":$PATH:" in *":$PREFIX/bin:"*) ;;
    *) say "If needed: export PATH=\"$PREFIX/bin:\$PATH\"" ;;
esac
say "Try: $PREFIX/bin/whydown demo"
say "GUI (optional Python 3 + PyQt6): $PREFIX/bin/whydown gui"
say "Checking optional GUI prerequisites (the CLI/TUI never require them):"
if gui_prereqs; then
    gui_ready=1
else
    gui_ready=0
fi
if [ "$DESKTOP" -eq 1 ]; then
    datadir="${XDG_DATA_HOME:-$HOME/.local/share}"
    appdir="$datadir/applications"
    icondir="$datadir/icons/hicolor/256x256/apps"
    mkdir -p "$appdir" "$icondir" || fail "Cannot create desktop application directories"
    icon_tmp=$(mktemp "$icondir/.whydown-icon.XXXXXXXX") ||
        fail "Cannot stage the WhyDown icon"
    if ! "$PREFIX/bin/whydown" icon > "$icon_tmp"; then
        rm -f -- "$icon_tmp"
        fail "Cannot obtain embedded WhyDown PNG icon"
    fi
    chmod 644 "$icon_tmp" ||
        { rm -f -- "$icon_tmp"; fail "Cannot set icon file permissions"; }
    mv -f -- "$icon_tmp" "$icondir/whydown.png" ||
        { rm -f -- "$icon_tmp"; fail "Cannot install WhyDown icon"; }
    say "Application icon: $icondir/whydown.png"
    desktop_tmp=$(mktemp "$appdir/.whydown-app.XXXXXXXX") ||
        fail "Cannot stage desktop launcher"
    if ! cat > "$desktop_tmp" <<EOF
[Desktop Entry]
Version=1.0
Type=Application
Name=WhyDown
GenericName=Linux Diagnostics
Comment=Evidence-based, read-only Linux troubleshooting
Exec=$PREFIX/bin/whydown gui
TryExec=$PREFIX/bin/whydown
Icon=whydown
Terminal=false
Categories=System;Utility;
Keywords=Linux;System;Diagnostics;Network;
EOF
    then
        rm -f -- "$desktop_tmp"
        fail "Cannot create desktop launcher"
    fi
    chmod 644 "$desktop_tmp" || { rm -f -- "$desktop_tmp"; fail "Cannot set desktop launcher permissions"; }
    mv -f -- "$desktop_tmp" "$appdir/whydown.desktop" ||
        { rm -f -- "$desktop_tmp"; fail "Cannot install desktop launcher"; }
    say "Desktop launcher: $appdir/whydown.desktop"
    if [ "$gui_ready" -eq 0 ]; then
        say "WARNING: The desktop launcher was created, but the GUI will not start until Python 3/PyQt6 is available."
        say "Recheck later with: sh scripts/install.sh --check-gui-prereqs"
    fi
fi
