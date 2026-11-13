"""Real Qt6/Xvfb GUI smoke test with a fictional WhyDown diagnostic only.

Crucially verifies MOUSE selection on a read-only evidence pane, and that
copying does not include unselected diagnostic details. No network or host
journal access; uses the same embedded GUI module as the shipped binary.
"""
import importlib.util
import os
import pathlib
import sys
import tempfile
import time

from PyQt6 import QtCore, QtGui, QtTest, QtWidgets

rootdir = pathlib.Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location(
    "whydown_gui", rootdir / "scripts" / "whydown_gui.py")
mod = importlib.util.module_from_spec(spec)
spec.loader.exec_module(mod)
binary = pathlib.Path(sys.argv[1]).resolve()
assert binary.is_file()
# Never read/write the runner's persistent desktop preferences.
test_settings = tempfile.TemporaryDirectory(prefix="whydown-qt-theme-")
os.environ["XDG_CONFIG_HOME"] = test_settings.name
app = QtWidgets.QApplication(["WhyDown GUI test"])
args = mod.parse_args(["--whydown-bin", str(binary), "--demo", "--current-boot"])
preferences = QtCore.QSettings("WhyDown", "WhyDown")
assert preferences.value("appearance/theme") is None, "Fresh install must have no saved preference"
gui = mod.WhyDownGUI(args)
gui.show()
app.processEvents()
try:
    assert gui.theme_box.currentText() == "Dark"
    initial_theme = gui.effective_theme
    assert initial_theme == "dark"
    assert preferences.value("appearance/theme") is None, (
        "Opening a GUI must not persist a setting before user choice")


    # Manual choice changes every themed widget and persists across launch.
    gui.theme_box.setCurrentText("Light")
    app.processEvents()
    gui.theme_box.setCurrentText("Dark")
    app.processEvents()
    assert gui.theme_preference == "dark"
    assert gui.effective_theme == "dark"
    assert "#152338" in gui.styleSheet()
    assert gui.detail.isReadOnly() and gui.detail.isEnabled()
    assert QtCore.QSettings("WhyDown", "WhyDown").value("appearance/theme") == "dark"

    reopened = mod.WhyDownGUI(args)
    assert reopened.theme_box.currentText() == "Dark"
    assert reopened.effective_theme == "dark"
    reopened.close()
    app.processEvents()

    gui.theme_box.setCurrentText("Light")
    app.processEvents()
    assert gui.theme_preference == "light"
    assert gui.effective_theme == "light"
    assert "#f2f6fb" in gui.styleSheet()
    assert "#152338" not in gui.styleSheet()

    # System setting follows platform changes without overwriting preference.
    gui.theme_box.setCurrentText("System")
    app.processEvents()
    assert gui.theme_preference == "system"
    detector = mod.system_prefers_dark
    try:
        mod.system_prefers_dark = lambda _app: True
        gui._system_theme_changed()
        assert gui.effective_theme == "dark"
        mod.system_prefers_dark = lambda _app: False
        gui._system_theme_changed()
        assert gui.effective_theme == "light"
        assert gui.theme_preference == "system"
        assert QtCore.QSettings("WhyDown", "WhyDown").value("appearance/theme") == "system"
    finally:
        mod.system_prefers_dark = detector
    gui._system_theme_changed()
    assert gui.effective_theme in ("light", "dark")

    # Explicitly chosen System is remembered, not silently replaced by Dark.
    system_reopened = mod.WhyDownGUI(args)
    assert system_reopened.theme_box.currentText() == "System"
    system_reopened.close()
    app.processEvents()

    # The actual release executable exports the same official icon as the GUI.
    assert not gui.brand_pixmap.isNull(), "Official embedded WD PNG is unreadable"
    assert not gui.windowIcon().isNull(), "Desktop window must have a WhyDown icon"
    assert gui.brandmark.pixmap() is not None
    assert not gui.brandmark.pixmap().isNull()

    assert gui.focus.currentText() == "all"
    assert gui.since.currentText() == "4h"
    assert gui.cap.currentText() == "10000"
    assert not gui.save_button.isEnabled()
    gui.analyze()
    deadline = time.monotonic() + 20
    while gui.report is None and time.monotonic() < deadline:
        app.processEvents()
        time.sleep(0.04)
    assert gui.report is not None, "GUI did not render the fictional diagnostic"
    assert gui.report["schema_version"] == 2
    assert gui.report.get("demo") is True
    assert "Analysis complete" in gui.status.text()
    assert gui.save_button.isEnabled()
    assert gui.tabs.currentIndex() == 0
    assert gui.findings.topLevelItemCount() >= 1
    assert gui.tabs.tabText(0) == "Incident overview"
    assert gui.tabs.tabText(1) == "Incident details"
    first = gui.findings.topLevelItem(0)
    assert first.text(0) in ("WARNING", "INFO")
    assert first.text(1)
    assert "Double-click" in gui.overview_preview.toPlainText()
    # Simulate an actual double-click in the Qt incident table.
    gui.findings.scrollToItem(first)
    rect = gui.findings.visualItemRect(first)
    # Qt's item-view double-click requires a preceding press on the item.
    QtTest.QTest.mouseClick(
        gui.findings.viewport(), QtCore.Qt.MouseButton.LeftButton,
        pos=rect.center())
    QtTest.QTest.mouseDClick(
        gui.findings.viewport(), QtCore.Qt.MouseButton.LeftButton,
        pos=rect.center())
    app.processEvents()
    assert gui.tabs.currentIndex() == 1, "Double-click must open the second page"
    assert "INCIDENT IDENTIFICATION" in gui.detail.toPlainText()
    assert "CLASSIFIED JOURNAL EVENTS" in gui.detail.toPlainText()
    assert "SUGGESTED READ-ONLY CHECKS" in gui.detail.toPlainText()
    assert "COLLECTION COVERAGE" in gui.coverage.toPlainText()

    # Qt native text editor MUST be enabled/read-only, not disabled.
    editor = gui.detail
    assert editor.isEnabled()
    assert editor.isReadOnly()
    assert editor.textInteractionFlags() & QtCore.Qt.TextInteractionFlag.TextSelectableByMouse
    assert editor.textInteractionFlags() & QtCore.Qt.TextInteractionFlag.TextSelectableByKeyboard
    editor.setFocus()
    app.processEvents()

    # Real mouse double-click selects exactly a word in the text viewport.
    cursor = QtGui.QTextCursor(editor.document())
    cursor.setPosition(3)
    editor.setTextCursor(cursor)
    app.processEvents()
    pos = editor.cursorRect(cursor).center()
    QtTest.QTest.mouseDClick(editor.viewport(), QtCore.Qt.MouseButton.LeftButton, pos=pos)
    app.processEvents()
    mouse_selected = editor.textCursor().selectedText()
    assert mouse_selected, "Cannot select evidence text with the mouse"
    assert len(mouse_selected) < len(editor.toPlainText()), "Selected entire report, not a passage"
    assert gui.copy_selection()
    assert app.clipboard().text() == mouse_selected

    # Ctrl+C on the currently selected text uses native Qt clipboard handling.
    app.clipboard().clear()
    QtTest.QTest.keyClick(editor, QtCore.Qt.Key.Key_C, QtCore.Qt.KeyboardModifier.ControlModifier)
    app.processEvents()
    assert app.clipboard().text() == mouse_selected

    # An empty selection does not silently export the full report.
    cursor.clearSelection()
    editor.setTextCursor(cursor)
    app.clipboard().clear()
    assert gui.copy_selection() is False
    assert app.clipboard().text() == ""
    assert "Select a text passage" in gui.status.text()

    # Coverage can be selected and copied independently of findings.
    gui.tabs.setCurrentIndex(3)
    app.processEvents()
    cover_cursor = gui.coverage.textCursor()
    cover_cursor.setPosition(0)
    cover_cursor.setPosition(12, QtGui.QTextCursor.MoveMode.KeepAnchor)
    gui.coverage.setTextCursor(cover_cursor)
    expected = cover_cursor.selectedText()
    assert gui.copy_selection() is True
    assert app.clipboard().text() == expected

    # Timeline copying includes only user-selected rows as tab-separated text.
    gui.tabs.setCurrentIndex(2)
    app.processEvents()
    assert gui.events.topLevelItemCount() >= 1
    gui.events.setCurrentItem(gui.events.topLevelItem(0))
    assert gui.copy_selection()
    assert "\t" in app.clipboard().text()
    # Double-click a classified timeline event for its individual safe indicators.
    first_event = gui.events.topLevelItem(0)
    gui.open_event(first_event)
    assert gui.tabs.currentIndex() == 1
    assert "CLASSIFIED JOURNAL EVENT" in gui.detail.toPlainText()
    gui.back_button.click()
    assert gui.tabs.currentIndex() == 0

    print("Qt6 GUI Xvfb smoke test: real mouse selection / Ctrl+C / toolbar copy passed")
finally:
    gui.close()
    app.processEvents()
    test_settings.cleanup()
