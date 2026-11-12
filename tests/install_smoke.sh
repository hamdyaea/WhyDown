#!/bin/sh
set -eu
sh -n scripts/install.sh
sh scripts/install.sh --help | grep -q 'Rust is NOT required'
sh scripts/install.sh --check-build-prereqs
target=$(sh scripts/install.sh --print-target)
case "$target" in
 x86_64-unknown-linux-gnu|aarch64-unknown-linux-gnu|x86_64-unknown-linux-musl|aarch64-unknown-linux-musl) ;;
 *) printf 'Unexpected install target: %s\n' "$target" >&2; exit 1 ;;
esac
sh scripts/install.sh --dry-run | grep -q 'Dry run: nothing downloaded or changed'
prefix=$(mktemp -d "${TMPDIR:-/tmp}/whydown-installer-ci.XXXXXXXX")
trap 'rm -rf -- "$prefix"' 0
sh scripts/install.sh --from-source --prefix "$prefix"
version=$(sed -n 's/^version = "\([^"]*\)"$/\1/p' Cargo.toml | head -1)
"$prefix/bin/whydown" --version | grep -Fx "whydown $version"
