#!/usr/bin/env bash
# Build one native .deb from a previously compiled, runnable WhyDown binary.
# Ubuntu, Debian and Kali share this package format; target architecture matters.
set -euo pipefail

if [[ $# -lt 1 || $# -gt 2 ]]; then
  echo "Usage: $0 /path/to/native/whydown [output-dir]" >&2
  exit 2
fi
root_repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
binary="$(realpath "$1")"
out="$(realpath -m "${2:-$root_repo/dist}")"
version="$(sed -n 's/^version = "\([^"]*\)"$/\1/p' "$root_repo/Cargo.toml" | head -1)"
case "$version" in
  *[!0-9.]*|'') echo "Invalid version in Cargo.toml" >&2; exit 1 ;;
esac
case "$(dpkg --print-architecture)" in
  amd64) arch=amd64 ;;
  arm64) arch=arm64 ;;
  *) echo "Only amd64 and arm64 Debian packages are supported" >&2; exit 1 ;;
esac
test -x "$binary"
test -f "$root_repo/LICENSE"
test -f "$root_repo/README.md"
"$binary" --version | grep -Fx "whydown $version"
stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT
pkg="$stage/pkg"
install -Dm755 "$binary" "$pkg/usr/bin/whydown"
install -Dm644 "$root_repo/README.md" "$pkg/usr/share/doc/whydown/README.md"
install -Dm644 "$root_repo/LICENSE" "$pkg/usr/share/doc/whydown/copyright"
install -d "$pkg/usr/share/icons/hicolor/256x256/apps" "$pkg/usr/share/applications" "$pkg/DEBIAN"
"$binary" icon > "$pkg/usr/share/icons/hicolor/256x256/apps/whydown.png"
cat > "$pkg/usr/share/applications/whydown.desktop" <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=WhyDown
Comment=Evidence-based, read-only Linux troubleshooting
Exec=whydown gui
TryExec=whydown
Icon=whydown
Terminal=false
Categories=System;Utility;
Keywords=systemd;troubleshooting;diagnostics;logs;
DESKTOP
cat > "$pkg/DEBIAN/control" <<CONTROL
Package: whydown
Version: ${version}-1
Section: utils
Priority: optional
Architecture: ${arch}
Maintainer: Hamdy Abou El Anein <hamdyaea@users.noreply.github.com>
Recommends: python3, python3-pyqt6
Homepage: https://github.com/hamdyaea/WhyDown
Description: Read-only, evidence-based Linux diagnostics
 WhyDown analyzes local systemd journal events, networking, storage,
 services and resource usage without changing the machine or sending
 telemetry. Includes a terminal CLI/TUI and an optional PyQt6 GUI.
CONTROL
mkdir -p "$out"
deb="$out/whydown_${version}-1_${arch}.deb"
dpkg-deb --build --root-owner-group "$pkg" "$deb"
test "$(dpkg-deb -f "$deb" Architecture)" = "$arch"
test "$(dpkg-deb -f "$deb" Package)" = whydown
mkdir -p "$stage/unpack"
dpkg-deb -x "$deb" "$stage/unpack"
"$stage/unpack/usr/bin/whydown" --version | grep -Fx "whydown $version"
"$stage/unpack/usr/bin/whydown" demo --format json > "$stage/demo.json"
python3 -m json.tool "$stage/demo.json" >/dev/null
test -s "$stage/unpack/usr/share/icons/hicolor/256x256/apps/whydown.png"
test -f "$stage/unpack/usr/share/applications/whydown.desktop"
( cd "$out" && sha256sum "$(basename "$deb")" > "$(basename "$deb").sha256" )
# Release assets must remain verifiable after download or relocation.
( cd "$out" && sha256sum -c "$(basename "$deb").sha256" )
printf 'Built and verified %s\n' "$deb"
