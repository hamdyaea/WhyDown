#!/bin/sh
# Standalone GUI prerequisite check: never downloads or installs anything.
set -eu

temp=$(mktemp -d "${TMPDIR:-/tmp}/whydown-gui-check.XXXXXXXX") ||
    { echo "Cannot create test directory" >&2; exit 1; }
trap 'rm -rf -- "$temp"' 0

mkdir -p "$temp/bin"
ln -s "$(command -v uname)" "$temp/bin/uname"
# Only uname is on PATH: simulate a machine without Python.
if PATH="$temp/bin" /bin/sh scripts/install.sh --check-gui-prereqs >"$temp/no-python.txt" 2>&1; then
    echo "ERROR: missing python3 unexpectedly passed" >&2
    exit 1
fi
grep -q 'GUI dependency MISSING: python3' "$temp/no-python.txt"
grep -q 'CLI and TUI remain fully usable' "$temp/no-python.txt"
grep -q 'No packages were installed or changed' "$temp/no-python.txt"

cat >"$temp/bin/python3" <<'PYTHON'
#!/bin/sh
case "$1" in
    --version) printf 'Python 3.13.0\n'; exit 0 ;;
    -c)
        case "${FAKE_TK_MODE:-missing}" in
            ok) exit 0 ;;
            missing) printf 'ModuleNotFoundError: tkinter\n' >&2; exit 1 ;;
        esac ;;
esac
exit 3
PYTHON
chmod +x "$temp/bin/python3"
if FAKE_TK_MODE=missing PATH="$temp/bin" /bin/sh scripts/install.sh --check-gui-prereqs >"$temp/no-tk.txt" 2>&1; then
    echo "ERROR: missing Tkinter unexpectedly passed" >&2
    exit 1
fi
grep -q 'GUI dependency OK: Python 3 available' "$temp/no-tk.txt"
grep -q 'GUI dependency MISSING: Tkinter' "$temp/no-tk.txt"
grep -q 'No packages were installed or changed' "$temp/no-tk.txt"

FAKE_TK_MODE=ok PATH="$temp/bin" /bin/sh scripts/install.sh --check-gui-prereqs >"$temp/ok.txt"
grep -q 'GUI dependency OK: Python 3 available' "$temp/ok.txt"
grep -q 'GUI dependency OK: Tkinter import succeeded' "$temp/ok.txt"
grep -q 'GUI dependencies available' "$temp/ok.txt"

# Missing GUI dependencies cannot prevent an ordinary, read-only dry-run.
FAKE_TK_MODE=missing PATH="$temp/bin" /bin/sh scripts/install.sh --dry-run >"$temp/dry-run.txt"
grep -q 'Dry run: nothing downloaded or changed' "$temp/dry-run.txt"
grep -q 'GUI dependency MISSING: Tkinter' "$temp/dry-run.txt"

# In --desktop mode the GUI advice is visible but the binary itself remains
# optional; dry-run must not create an application launcher.
prefix="$temp/prefix"
FAKE_TK_MODE=missing HOME="$temp" PATH="$temp/bin" \
    /bin/sh scripts/install.sh --dry-run --desktop --prefix "$prefix" >"$temp/desktop-dry-run.txt"
grep -q 'Dry run: nothing downloaded or changed' "$temp/desktop-dry-run.txt"
test ! -e "$temp/.local/share/applications/whydown.desktop"

printf 'GUI prerequisite smoke tests: passed\n'
