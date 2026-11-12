#!/usr/bin/env python3
"""Exercise the actual WhyDown interactive TUI on a Linux pseudo-terminal.

This checks keyboard input and output at the terminal boundary; unit tests
that use in-memory BufRead alone cannot catch swallowed real menu input.
"""

import os
import pty
import select
import subprocess
import sys
import time


def exercise(binary):
    master, slave = pty.openpty()
    env = dict(os.environ)
    env["TERM"] = "dumb"  # plain transcript avoids ANSI erasure
    proc = subprocess.Popen(
        [binary, "tui", "--demo"],
        stdin=slave,
        stdout=slave,
        stderr=slave,
        env=env,
        start_new_session=True,
    )
    os.close(slave)
    transcript = b""

    def wait_for(needle, timeout=12):
        nonlocal transcript
        expected = needle.encode("utf-8")
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            if expected in transcript:
                return
            available, _, _ = select.select([master], [], [], 0.2)
            if available:
                try:
                    chunk = os.read(master, 65536)
                except OSError:
                    break
                if not chunk:
                    break
                transcript += chunk
        raise AssertionError(
            f"Missing terminal response {needle!r}; recent output:\n"
            + transcript[-3000:].decode("utf-8", errors="replace")
        )

    def send(action):
        os.write(master, (action + "\n").encode("utf-8"))

    try:
        wait_for("Selection (number + Enter) >")
        send("2")
        wait_for("Focus changed to network")
        send("5")
        wait_for("Journal cap changed to 10000")
        send("1")
        wait_for("Diagnostic completed:")
        send("6")  # MUST be interpreted immediately after diagnostic
        wait_for("Finding number")
        send("1")
        wait_for("Finding evidence")
        send("7")  # entered at detail page, forwarded to dashboard
        wait_for("Recent classified events")
        send("")
        wait_for("Returned from timeline")
        send("0")
        wait_for("TUI closed; no system changes made.")
        proc.wait(timeout=5)
        assert proc.returncode == 0, f'TUI exit code: {proc.returncode}'
        assert b"DEMO (fictional)" in transcript
        print("PTY TUI demo navigation smoke test: passed")
    finally:
        if proc.poll() is None:
            proc.terminate()
            try:
                proc.wait(timeout=3)
            except subprocess.TimeoutExpired:
                proc.kill()
                proc.wait(timeout=3)
        os.close(master)


if __name__ == "__main__":
    if len(sys.argv) != 2:
        raise SystemExit("Usage: tui_pty_smoke.py /path/to/whydown")
    exercise(sys.argv[1])
