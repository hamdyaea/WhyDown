"""Real Tkinter GUI smoke test under Xvfb; requires Python Tk + a display.

Exercises the desktop window and the same released Rust diagnostic binary,
but uses *fictional demo evidence* and never reads host logs or touches files.
"""
import importlib.util
import pathlib
import sys
import time
import tkinter as tk

rootdir = pathlib.Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location(
    "whydown_gui", rootdir / "scripts" / "whydown_gui.py")
mod = importlib.util.module_from_spec(spec)
spec.loader.exec_module(mod)

binary = pathlib.Path(sys.argv[1]).resolve()
assert binary.is_file(), f"Missing executable: {binary}"
root = tk.Tk()
root.withdraw()
args = mod.parse_args(["--whydown-bin", str(binary), "--demo"])
gui = mod.WhyDownGUI(root, args)
try:
    assert gui.focus.get() == "all"
    assert gui.since.get() == "4h"
    assert gui.max_events.get() == "10000"
    assert "disabled" in gui.save_button.state()
    gui.analyze()
    deadline = time.monotonic() + 20
    while gui.report is None and time.monotonic() < deadline:
        root.update()
        time.sleep(0.05)
    assert gui.report is not None, "GUI did not render fictional report"
    assert gui.report["schema_version"] == 2
    assert gui.report["demo"] is True
    assert "Analysis complete" in gui.status.get()
    assert "warning(s)" in gui.summary.get()
    assert "disabled" not in gui.save_button.state()
    assert gui.tabs.tab(gui.tabs.select(), "text") == "Findings & evidence"
    assert gui.coverage.get("1.0", "end").find("COLLECTION COVERAGE") != -1
    print("Real Tk GUI smoke test (fictional demo only): passed")
finally:
    gui.close()
