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
XDG_DATA_HOME="${TMPDIR:-/tmp}/whydown-nonexistent-ci-launcher" sh scripts/install.sh --dry-run --desktop |
    grep -q 'Dry run: nothing downloaded or changed'
case "$(sh scripts/install.sh --print-target)" in
    *-unknown-linux-musl) ;;
    *) echo "Auto target must be static musl" >&2; exit 1 ;;
esac
case "$(sh scripts/install.sh --print-target --libc gnu)" in
    *-unknown-linux-gnu) ;;
    *) echo "Explicit GNU target selection failed" >&2; exit 1 ;;
esac
prefix=$(mktemp -d "${TMPDIR:-/tmp}/whydown-installer-ci.XXXXXXXX")
trap 'rm -rf -- "$prefix"' 0
XDG_DATA_HOME="$prefix/xdg" sh scripts/install.sh --from-source --desktop --prefix "$prefix"
test -f "$prefix/xdg/applications/whydown.desktop"
grep -F "Exec=$prefix/bin/whydown gui" "$prefix/xdg/applications/whydown.desktop"
grep -q 'Terminal=false' "$prefix/xdg/applications/whydown.desktop"
version=$(sed -n 's/^version = "\([^"]*\)"$/\1/p' Cargo.toml | head -1)
"$prefix/bin/whydown" --version | grep -Fx "whydown $version"

# Mock GitHub downloads to validate successful install and safe failure locally.
mock_dir=$(mktemp -d "${TMPDIR:-/tmp}/whydown-mock-ci.XXXXXXXX")
trap 'rm -rf -- "$prefix" "$mock_dir"' 0
mkdir -p "$mock_dir/bin" "$mock_dir/files"
cat > "$mock_dir/bin/curl" <<'CURL'
#!/bin/sh
set -eu
output=
url=
while [ "$#" -gt 0 ]; do
    case "$1" in
        -o|--output) output="$2"; shift 2 ;;
        -*) shift ;;
        *) url="$1"; shift ;;
    esac
done
[ -n "$output" ] && [ -n "$url" ]
cp "$TEST_DOWNLOAD_DIR/$(basename "$url")" "$output"
CURL
chmod 755 "$mock_dir/bin/curl"
asset="whydown-v${version}-${target}.tar.gz"
cat > "$mock_dir/files/whydown" <<EOF
#!/bin/sh
printf 'whydown ${version}\n'
EOF
chmod 755 "$mock_dir/files/whydown"
tar -czf "$mock_dir/files/$asset" -C "$mock_dir/files" whydown
( cd "$mock_dir/files" && sha256sum "$asset" > "$asset.sha256" )
PATH="$mock_dir/bin:$PATH" TEST_DOWNLOAD_DIR="$mock_dir/files" \
    sh scripts/install.sh --prefix "$prefix"
"$prefix/bin/whydown" --version | grep -Fx "whydown $version"
# Simulate a fully checksummed but incompatible executable. Existing binary
# must remain intact; the install should fail with a clear compatibility hint.
cat > "$mock_dir/files/whydown" <<'BAD'
#!/bin/sh
echo "GLIBC_2.39 not found" >&2
exit 127
BAD
chmod 755 "$mock_dir/files/whydown"
tar -czf "$mock_dir/files/$asset" -C "$mock_dir/files" whydown
( cd "$mock_dir/files" && sha256sum "$asset" > "$asset.sha256" )
if PATH="$mock_dir/bin:$PATH" TEST_DOWNLOAD_DIR="$mock_dir/files" \
    sh scripts/install.sh --prefix "$prefix" > "$mock_dir/failure.out" 2>&1; then
    echo "ERROR: incompatible candidate installation unexpectedly succeeded" >&2
    exit 1
fi
grep -q "existing installation preserved" "$mock_dir/failure.out"
"$prefix/bin/whydown" --version | grep -Fx "whydown $version"
