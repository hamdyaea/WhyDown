#!/bin/sh
# WhyDown: install prebuilt Linux binaries (default), or build locally.
# No automatic system package installation, sudo or service changes.
set -eu
VERSION="${WHYDOWN_VERSION:-v0.4.0}"
PREFIX="${WHYDOWN_PREFIX:-${HOME:-}/.local}"
LIBC=auto
MODE=binary
DRY_RUN=0
say() { printf '%s\n' "$*"; }
fail() { printf 'WhyDown installer: %s\n' "$*" >&2; exit 1; }
usage() {
cat <<'HELP'
WhyDown Linux installer (x86_64 and aarch64)
  sh scripts/install.sh [OPTIONS]
  (no options)           Download verified static musl binary; Rust is NOT required
  --from-source          Build checked-out repository using Rust/Cargo
  --check-build-prereqs  Explain missing compiler/Cargo prerequisites
  --print-target         Display detected target without installing
  --dry-run              Show selected binary without downloading
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
        --print-target) MODE=target; shift ;;
        --dry-run) DRY_RUN=1; shift ;;
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
if [ "$MODE" = source ]; then
    repo_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd -P)
    [ -f "$repo_dir/Cargo.toml" ] ||
        fail "--from-source requires the complete WhyDown Git checkout"
    prereqs || exit 1
    if [ "$DRY_RUN" -eq 1 ]; then say "Dry run: source build at $repo_dir"; exit 0; fi
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
    if [ "$DRY_RUN" -eq 1 ]; then say "Dry run: nothing downloaded or changed."; exit 0; fi
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
