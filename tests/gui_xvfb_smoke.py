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
    assert len(gui.context_menus) == 4
    assert gui.detail.cget("exportselection") == 0
    assert gui.detail.cget("state") == "disabled"

    # Select only a short fragment, never the complete diagnostic report.
    gui.detail.tag_add("sel", "1.0", "1.8")
    expected = gui.detail.get("1.0", "1.8")
    gui.copy_active_selection()
    assert root.clipboard_get() == expected, "Copy selection button copied unrelated text"
    assert "Copied 8 character(s)" in gui.status.get()
    assert gui.detail.get("1.0", "end").count("OBSERVED EVIDENCE") == 1

    # Verify Ctrl+C handler, including partial text from a disabled widget.
    gui.detail.tag_remove("sel", "1.0", "end")
    gui.detail.tag_add("sel", "3.0", "3.8")
    expected = gui.detail.get("3.0", "3.8")
    assert gui.on_copy_text(type("Event", (), {"widget": gui.detail})()) == "break"
    assert root.clipboard_get() == expected

    # A toolbar click should preserve selection even when focus moves.
    assert gui.detail.cget("exportselection") == 0
    gui.detail.tag_remove("sel", "1.0", "end")
    root.clipboard_clear()
    assert gui.copy_text_selection(gui.detail) is False
    assert "Select some text" in gui.status.get()
    try:
        root.clipboard_get()
    except tk.TclError:
        pass
    else:
        raise AssertionError("Empty selection must not leak the full report")

    # Coverage supports arbitrary substring selection, not only whole reports.
    gui.tabs.select(2)
    gui.coverage.tag_add("sel", "1.0", "1.10")
    expected = gui.coverage.get("1.0", "1.10")
    assert gui.copy_active_selection()
    assert root.clipboard_get() == expected

    # Timeline: selecting an entry copies one row (not all journal entries).
    gui.tabs.select(1)
    rows = gui.event_tree.get_children()
    assert rows, "Demo report should include a classified event"
    gui.event_tree.selection_set(rows[0])
    expected = gui.selected_rows(gui.event_tree)
    assert gui.copy_active_selection()
    assert root.clipboard_get() == expected
    assert "\\t" in expected, "Timeline rows should have tab-separated columns"

    # Context menu and Ctrl+C bindings are explicitly installed.
    assert gui.detail.bind("<Button-3>")
    assert gui.detail.bind("<Control-c>")
    assert gui.coverage.bind("<Control-c>")
    assert gui.event_tree.bind("<Control-c>")
    print("Real Tk GUI smoke test (fictional demo only): passed")
finally:
    gui.close()
