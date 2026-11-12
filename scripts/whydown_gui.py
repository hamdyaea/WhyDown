#!/usr/bin/env python3
"""WhyDown Qt6 desktop. GPL-3.0-only, offline and read-only.

PyQt6 is needed only for GUI mode; the existing CLI/TUI are independent.
Evidence uses an ENABLED read-only QPlainTextEdit for real mouse selections.
"""
from __future__ import annotations
import argparse
import json
import os
import re
import sys

try:
    from PyQt6 import QtCore, QtGui, QtWidgets
except ImportError:
    QtCore = QtGui = QtWidgets = None

FOCUSES = ("all", "network", "services", "system")
LOOKBACKS = ("1h", "4h", "24h", "7d")
MAX_EVENTS = ("5000", "10000")
MAX_REPORT_BYTES = 8 * 1024 * 1024
THEMES = ("system", "light", "dark")
THEME_SETTINGS_KEY = "appearance/theme"

# The light palette is our existing visual design. A complete dark override
# keeps menus, controls, results and selected evidence readable throughout.
LIGHT_STYLE = """
QMainWindow, QWidget#Root { background: #f2f6fb; color: #1c2b3e; }
QFrame#Sidebar { background: #142136; border: 0; }
QLabel#Brand { color: white; font-size: 23px; font-weight: 800; }
QLabel#SideCaption { color: #91a9c6; font-size: 10px; font-weight: 600; }
QPushButton#Nav { background: transparent; color: #bdd0e1; text-align: left;
  border: none; border-radius: 10px; padding: 12px 17px; font-weight: 600; }
QPushButton#Nav:hover { background: #253951; }
QPushButton#Nav:checked { background: #265263; color: #9ff2df; }
QLabel#Title { color: #1b2b40; font-size: 26px; font-weight: 800; }
QLabel#Muted { color: #6c7f96; font-size: 11px; }
QLabel#CardTitle { color: #263950; font-size: 13px; font-weight: 800; }
QLabel#CardValue { color: #192b42; font-size: 26px; font-weight: 800; }
QLabel#CardCaption { color: #74869b; font-size: 10px; }
QLabel#Pill { color: #0a8073; background: #d7f5ee; border-radius: 10px;
  padding: 7px 11px; font-size: 10px; font-weight: 800; }
QFrame#Card { background: white; border: 1px solid #e1e9f2; border-radius: 13px; }
QComboBox { background: #f8fafc; color: #29384c; border: 1px solid #dce4ee;
  border-radius: 8px; padding: 8px; min-width: 94px; }
QComboBox:hover { border: 1px solid #148c85; }
QCheckBox { color: #40546a; font-size: 11px; }
QPushButton#Primary { color: white; background: #118d84; border: none;
  border-radius: 9px; font-size: 12px; font-weight: 800; padding: 12px 20px; }
QPushButton#Primary:hover { background: #08776e; }
QPushButton#Primary:disabled { background: #99c6c0; }
QPushButton#Secondary { background: #fff; color: #30455e;
  border: 1px solid #dae5ed; border-radius: 9px;
  padding: 10px 15px; font-size: 11px; font-weight: 700; }
QPushButton#Secondary:hover { background: #eaf5f3; }
QTabWidget::pane { background: #fff; border: 1px solid #e1e9f2;
  border-radius: 10px; }
QTabBar::tab { background: transparent; color: #647b92;
  padding: 12px 19px; font-weight: 700; font-size: 11px; }
QTabBar::tab:selected { color: #0b887e; border-bottom: 3px solid #12ac9d; }
QTreeWidget { background: white; color: #293b50; border: 0;
  alternate-background-color: #f5f9fc; }
QTreeWidget::item { min-height: 30px; padding: 3px; }
QTreeWidget::item:selected { color: #124439; background: #d4f1e8; }
QHeaderView::section { background: #f4f7fb; color: #64798d; border: 0;
  border-bottom: 1px solid #e0e8f1; padding: 9px; font-weight: 700; }
QPlainTextEdit { color: #1d354a; background: #fff; border: 0;
  selection-color: #103d39; selection-background-color: #a3e4d9;
  font-family: monospace; font-size: 12px; padding: 12px; }
QSplitter::handle { background: #e5edf5; width: 2px; }
QProgressBar { border: 0; background: #e1e9f0; border-radius: 3px; height: 5px; }
QProgressBar::chunk { background: #10a99b; border-radius: 3px; }
"""

DARK_STYLE = LIGHT_STYLE + """
QMainWindow, QWidget#Root { background: #0b1421; color: #e3edf8; }
QFrame#Sidebar { background: #080f1c; border: 0; }
QLabel#Brand { color: #f4f8ff; }
QLabel#SideCaption { color: #9fb4ce; }
QPushButton#Nav { background: transparent; color: #b8cbe2; }
QPushButton#Nav:hover { background: #24354b; color: #ffffff; }
QPushButton#Nav:checked { background: #1b504f; color: #b8fff1; }
QLabel#Title { color: #f0f5fc; }
QLabel#Muted { color: #afc1d6; }
QLabel#CardTitle { color: #e7f0fa; }
QLabel#CardValue { color: #f0f6ff; }
QLabel#CardCaption { color: #a7bcd0; }
QLabel#Pill { color: #a6f6e5; background: #1a4947; }
QFrame#Card { background: #152338; border: 1px solid #293b52; }
QComboBox { background: #1b2a40; color: #edf4ff;
  border: 1px solid #40546a; }
QComboBox:hover { border: 1px solid #49c8bd; }
QComboBox QAbstractItemView { background: #18283c; color: #edf6ff;
  selection-background-color: #23685e; selection-color: #ffffff; }
QCheckBox { color: #d8e6f4; }
QPushButton#Primary { background: #008d83; color: #ffffff; }
QPushButton#Primary:hover { background: #13a99c; }
QPushButton#Primary:disabled { background: #375d60; color: #bad5d4; }
QPushButton#Secondary { background: #1d3047; color: #e4edf8;
  border: 1px solid #3d5068; }
QPushButton#Secondary:hover { background: #294358; }
QTabWidget::pane { background: #152338; border: 1px solid #293b52; }
QTabBar::tab { background: transparent; color: #b1c3da; }
QTabBar::tab:selected { color: #7af4d9; border-bottom: 3px solid #40d2b9; }
QTreeWidget { background: #152338; color: #eaf3ff;
  alternate-background-color: #1b2c42; }
QTreeWidget::item:selected { color: #eafff6; background: #1e655e; }
QHeaderView::section { background: #1e3047; color: #d9e7f6;
  border-bottom: 1px solid #34485f; }
QPlainTextEdit { background: #152338; color: #edf5ff;
  selection-color: #ffffff; selection-background-color: #146e69; }
QSplitter::handle { background: #34475f; }
QProgressBar { background: #304259; }
QProgressBar::chunk { background: #3ad2bf; }
"""

def normalize_theme(value):
    """Unknown values, including absent/invalid preferences, default to Dark."""
    return value if isinstance(value, str) and value in THEMES else "dark"


def resolve_theme(preference, system_dark):
    """Pure theme rule, also testable on Linux/Qt versions without colorScheme."""
    preference = normalize_theme(preference)
    if preference != "system":
        return preference
    return "dark" if system_dark else "light"


def system_prefers_dark(app):
    """Qt >= 6.5: platform color scheme; earlier Qt: original system palette.

    A CSS-styled window's background must not be used to infer the desktop
    theme: this function only examines Qt's platform/style hints or app palette.
    """
    hints = app.styleHints()
    if hasattr(hints, "colorScheme") and hasattr(QtCore.Qt, "ColorScheme"):
        scheme = hints.colorScheme()
        if scheme == QtCore.Qt.ColorScheme.Dark:
            return True
        if scheme == QtCore.Qt.ColorScheme.Light:
            return False
    return app.palette().color(QtGui.QPalette.ColorRole.Window).lightness() < 128


def stylesheet_for(preference, system_dark):
    return DARK_STYLE if resolve_theme(preference, system_dark) == "dark" else LIGHT_STYLE


def build_command(binary, *, demo=False, focus="all", since="4h",
                  max_events=10000, current_boot=True, disk_details=False,
                  include_remote_mounts=False):
    if focus not in FOCUSES:
        raise ValueError("Invalid diagnostic scope")
    match = re.fullmatch(r"([0-9]+)([mhd])", since)
    if match is None:
        raise ValueError("Invalid lookback")
    seconds = int(match.group(1)) * {"m": 60, "h": 3600, "d": 86400}[match.group(2)]
    if not 60 <= seconds <= 30 * 86400:
        raise ValueError("Lookback outside safe range")
    if str(max_events) not in MAX_EVENTS:
        raise ValueError("Invalid journal limit")
    cmd = [os.fspath(binary), "demo" if demo else "diagnose",
           "--focus", focus, "--since", since, "--max-events", str(max_events),
           "--format", "json", "--no-progress"]
    if current_boot and not demo: cmd.append("--current-boot")
    if disk_details and not demo: cmd.append("--disk-details")
    if include_remote_mounts and not demo: cmd.append("--include-remote-mounts")
    return cmd

def parse_report(raw):
    if isinstance(raw, bytes):
        if len(raw) > MAX_REPORT_BYTES: raise ValueError("Report exceeds 8 MiB")
        raw = raw.decode("utf-8")
    elif len(raw.encode("utf-8")) > MAX_REPORT_BYTES:
        raise ValueError("Report exceeds 8 MiB")
    report = json.loads(raw)
    if not isinstance(report, dict) or report.get("schema_version") != 2:
        raise ValueError("Expected a WhyDown JSON schema v2 report")
    if not (isinstance(report.get("findings"), list) and
            isinstance(report.get("timeline"), list) and
            isinstance(report.get("system"), dict)):
        raise ValueError("Report is missing structured fields")
    return report

def headline(report):
    summary = report.get("summary") or {}
    extra = " | PARTIAL" if report.get("journal_truncated") else ""
    return (f"{summary.get('warning_count', 0)} warnings, "
            f"{summary.get('info_count', 0)} info, "
            f"{report.get('journal_entries_examined', 0)} events{extra}")

def finding_details(f):
    lines = [str(f.get("title", "Finding")), "",
             f"SEVERITY: {f.get('severity', '?')}",
             f"CONFIDENCE: {f.get('confidence', '?')}",
             f"SCOPE: {f.get('scope', '?')}", "",
             "EXPLANATION", str(f.get("explanation", "")), "",
             "OBSERVED EVIDENCE"]
    lines.extend(" • " + str(e) for e in (f.get("evidence") or []))
    if not f.get("evidence"): lines.append(" No evidence supplied")
    lines.extend(["", "SUGGESTED READ-ONLY CHECKS"])
    lines.extend(" • " + str(c) for c in (f.get("checks") or []))
    if not f.get("checks"): lines.append(" No specific check supplied")
    lines.extend(["", "Observations are not proof of root cause or application readiness."])
    return "\n".join(lines)

def coverage_details(r):
    sysinfo = r.get("system") or {}
    lines = ["COLLECTION COVERAGE AND LIMITATIONS", "",
             f"Journal collected: {r.get('journal_checked', False)}",
             f"Journal entries examined: {r.get('journal_entries_examined', 0)}",
             f"Journal window partial: {r.get('journal_truncated', False)}",
             f"Current boot only: {r.get('journal_current_boot_only', False)}",
             f"Systemd checked: {sysinfo.get('system_units_checked', False)}",
             f"Disk usage checked: {sysinfo.get('disk_usage_checked', False)}",
             "", "CURRENT SERVICE SNAPSHOTS"]
    for v in (sysinfo.get("service_statuses") or [])[:20]:
        lines.append(f" • {v.get('unit', '?')}: {v.get('active_state', '?')}/"
                     f"{v.get('sub_state', '?')}; result={v.get('result', '?')}")
    lines.extend(["", "COLLECTOR LIMITATIONS"])
    notes = r.get("collection_notes") or []
    lines.extend(" • " + str(e) for e in notes[:80])
    if not notes: lines.append(" No additional notes")
    if r.get("journal_truncated"):
        lines.extend(["", "Journal partial: reduce lookback or inspect more entries."])
    lines.extend(["", "Active services and associated Wi-Fi do not prove connectivity."])
    return "\n".join(lines)

def parse_args(argv=None):
    parser = argparse.ArgumentParser(description="WhyDown Qt6 GUI")
    parser.add_argument("--whydown-bin", required=True)
    parser.add_argument("--demo", action="store_true")
    parser.add_argument("--focus", choices=FOCUSES, default="all")
    parser.add_argument("--since", default="4h")
    parser.add_argument("--max-events", choices=(5000, 10000), type=int, default=10000)
    parser.add_argument("--current-boot", action="store_true")
    parser.add_argument("--disk-details", action="store_true")
    parser.add_argument("--include-remote-mounts", action="store_true")
    return parser.parse_args(argv)


class WhyDownGUI(QtWidgets.QMainWindow if QtWidgets else object):
    """Qt widgets are modern-looking but all evidence fields are native/selectable."""
    def __init__(self, args):
        super().__init__()
        self.args = args
        self.report = self.raw_report = None
        self.running = False
        self.out = bytearray()
        self.err = bytearray()
        self.settings = QtCore.QSettings("WhyDown", "WhyDown")
        self.theme_preference = normalize_theme(
            self.settings.value(THEME_SETTINGS_KEY, "dark", type=str))
        self.effective_theme = None
        self.setWindowTitle("WhyDown  |  Linux system diagnostics")
        self.setMinimumSize(980, 640)
        self.resize(1250, 810)
        self._ui()
        self._apply_theme()
        hints = QtWidgets.QApplication.instance().styleHints()
        if hasattr(hints, "colorSchemeChanged"):
            hints.colorSchemeChanged.connect(self._system_theme_changed)
        app = QtWidgets.QApplication.instance()
        if hasattr(app, "paletteChanged"):
            app.paletteChanged.connect(self._system_theme_changed)
        self.process = QtCore.QProcess(self)
        self.process.setProcessChannelMode(QtCore.QProcess.ProcessChannelMode.SeparateChannels)
        self.process.readyReadStandardOutput.connect(self._stdout)
        self.process.readyReadStandardError.connect(self._stderr)
        self.process.finished.connect(self._finished)
        self.process.errorOccurred.connect(self._process_error)
        self.timer = QtCore.QTimer(self)
        self.timer.setSingleShot(True)
        self.timer.timeout.connect(lambda: self._abort("Diagnostic timeout (240s)."))

    @staticmethod
    def card():
        c = QtWidgets.QFrame()
        c.setObjectName("Card")
        return c

    @staticmethod
    def label(text, role):
        l = QtWidgets.QLabel(text)
        l.setObjectName(role)
        return l

    @staticmethod
    def readonly_text(placeholder):
        text = QtWidgets.QPlainTextEdit()
        text.setReadOnly(True)
        text.setTextInteractionFlags(
            QtCore.Qt.TextInteractionFlag.TextSelectableByMouse |
            QtCore.Qt.TextInteractionFlag.TextSelectableByKeyboard)
        text.setPlainText(placeholder)
        return text

    @staticmethod
    def dropdown(group, title, items, default):
        v = QtWidgets.QVBoxLayout()
        v.addWidget(WhyDownGUI.label(title, "Muted"))
        combo = QtWidgets.QComboBox()
        combo.addItems(items)
        combo.setCurrentText(default)
        v.addWidget(combo)
        group.addLayout(v)
        return combo

    def _ui(self):
        root = QtWidgets.QWidget()
        root.setObjectName("Root")
        self.setCentralWidget(root)
        row = QtWidgets.QHBoxLayout(root)
        row.setContentsMargins(0, 0, 0, 0)
        row.setSpacing(0)

        sidebar = QtWidgets.QFrame()
        sidebar.setObjectName("Sidebar")
        sidebar.setFixedWidth(220)
        nav = QtWidgets.QVBoxLayout(sidebar)
        nav.setContentsMargins(22, 30, 22, 24)
        nav.setSpacing(9)
        nav.addWidget(self.label("●  WhyDown", "Brand"))
        nav.addWidget(self.label("LOCAL SYSTEM DIAGNOSTICS", "SideCaption"))
        nav.addSpacing(46)
        nav.addWidget(self.label("WORKSPACE", "SideCaption"))
        self.navbuttons = []
        for n, name in enumerate(("◉  Findings", "◷  Timeline", "▤  Coverage")):
            btn = QtWidgets.QPushButton(name)
            btn.setObjectName("Nav")
            btn.setCheckable(True)
            btn.clicked.connect(lambda checked=False, i=n: self.tabs.setCurrentIndex(i))
            nav.addWidget(btn)
            self.navbuttons.append(btn)
        nav.addStretch()
        nav.addWidget(self.label("●  OFFLINE · READ-ONLY\n\nNo telemetry. No repairs.", "SideCaption"))
        row.addWidget(sidebar)

        scroll = QtWidgets.QScrollArea()
        scroll.setWidgetResizable(True)
        scroll.setFrameShape(QtWidgets.QFrame.Shape.NoFrame)
        scroll.setHorizontalScrollBarPolicy(QtCore.Qt.ScrollBarPolicy.ScrollBarAlwaysOff)
        row.addWidget(scroll, 1)
        body = QtWidgets.QWidget()
        body.setObjectName("Root")
        scroll.setWidget(body)
        main = QtWidgets.QVBoxLayout(body)
        main.setContentsMargins(29, 27, 29, 22)
        main.setSpacing(14)

        header = QtWidgets.QHBoxLayout()
        headings = QtWidgets.QVBoxLayout()
        headings.addWidget(self.label("System diagnostics", "Title"))
        headings.addWidget(self.label("Understand what happened, backed by local evidence.", "Muted"))
        header.addLayout(headings)
        header.addStretch()
        header.addWidget(self.label("APPEARANCE", "Muted"))
        self.theme_box = QtWidgets.QComboBox()
        self.theme_box.setObjectName("ThemeSelector")
        self.theme_box.addItems(("System", "Light", "Dark"))
        self.theme_box.setCurrentText(self.theme_preference.capitalize())
        self.theme_box.setAccessibleName("Appearance: System, Light or Dark")
        self.theme_box.setToolTip(
            "System follows your desktop appearance when Qt supports it. "
            "Your choice is saved locally.")
        self.theme_box.setMinimumWidth(105)
        self.theme_box.currentTextChanged.connect(self._theme_selected)
        header.addWidget(self.theme_box)
        header.addSpacing(10)
        header.addWidget(self.label("●  PRIVATE & LOCAL", "Pill"))
        main.addLayout(header)

        form = self.card()
        fl = QtWidgets.QVBoxLayout(form)
        fl.setContentsMargins(18, 17, 18, 17)
        fl.addWidget(self.label("01   Configure & analyze", "CardTitle"))
        opts = QtWidgets.QHBoxLayout()
        opts.setSpacing(15)
        self.focus = self.dropdown(opts, "FOCUS", FOCUSES, self.args.focus)
        ranges = list(LOOKBACKS)
        if self.args.since not in ranges: ranges.append(self.args.since)
        self.since = self.dropdown(opts, "LOOKBACK", ranges, self.args.since)
        self.cap = self.dropdown(opts, "MAX EVENTS", MAX_EVENTS, str(self.args.max_events))
        self.boot = QtWidgets.QCheckBox("Current boot only")
        self.boot.setChecked(self.args.current_boot)
        opts.addWidget(self.boot)
        opts.addStretch(1)
        self.analyze_button = QtWidgets.QPushButton(
            "Explore sample" if self.args.demo else "Run diagnostics")
        self.analyze_button.setObjectName("Primary")
        self.analyze_button.clicked.connect(self.analyze)
        opts.addWidget(self.analyze_button)
        fl.addLayout(opts)
        advanced = QtWidgets.QHBoxLayout()
        self.disk = QtWidgets.QCheckBox("Detailed disk usage (slower)")
        self.disk.setChecked(self.args.disk_details)
        self.remote = QtWidgets.QCheckBox("Include remote mounts (may block)")
        self.remote.setChecked(self.args.include_remote_mounts)
        advanced.addWidget(self.disk)
        advanced.addWidget(self.remote)
        advanced.addStretch()
        fl.addLayout(advanced)
        main.addWidget(form)

        strip = QtWidgets.QHBoxLayout()
        strip.setSpacing(14)
        self.metrics = {}
        for key, name, subtitle in (("warnings", "WARNINGS", "Need investigation"),
                                     ("info", "OBSERVATIONS", "Informational signals"),
                                     ("events", "JOURNAL ENTRIES", "Scoped evidence")):
            card = self.card()
            v = QtWidgets.QVBoxLayout(card)
            v.setContentsMargins(16, 12, 16, 12)
            v.addWidget(self.label(name, "CardCaption"))
            value = self.label("—", "CardValue")
            self.metrics[key] = value
            v.addWidget(value)
            v.addWidget(self.label(subtitle, "CardCaption"))
            strip.addWidget(card, 1)
        main.addLayout(strip)
        summary = QtWidgets.QHBoxLayout()
        summary.addWidget(self.label("02   Results & evidence", "CardTitle"))
        summary.addStretch()
        summary.addWidget(self.label("Drag to select text, then Ctrl+C or Copy selection", "Muted"))
        main.addLayout(summary)

        self.tabs = QtWidgets.QTabWidget()
        self.tabs.currentChanged.connect(self._navigate)
        main.addWidget(self.tabs, 1)
        self.tabs.setMinimumHeight(265)
        finds = QtWidgets.QWidget()
        layout = QtWidgets.QVBoxLayout(finds)
        layout.setContentsMargins(5, 5, 5, 5)
        split = QtWidgets.QSplitter(QtCore.Qt.Orientation.Horizontal)
        self.findings = QtWidgets.QTreeWidget()
        self.findings.setHeaderLabels(["Finding", "Severity"])
        self.findings.setRootIsDecorated(False)
        self.findings.setAlternatingRowColors(True)
        self.findings.currentItemChanged.connect(self._finding_changed)
        split.addWidget(self.findings)
        self.detail = self.readonly_text("Start an analysis to review the evidence.")
        split.addWidget(self.detail)
        split.setSizes([280, 630])
        layout.addWidget(split)
        self.tabs.addTab(finds, "Findings & evidence")

        timeline = QtWidgets.QWidget()
        lv = QtWidgets.QVBoxLayout(timeline)
        lv.setContentsMargins(5, 5, 5, 5)
        self.events = QtWidgets.QTreeWidget()
        self.events.setHeaderLabels(["Time (UTC)", "Event", "Subject", "Source"])
        self.events.setRootIsDecorated(False)
        self.events.setAlternatingRowColors(True)
        self.events.setSelectionMode(QtWidgets.QAbstractItemView.SelectionMode.ExtendedSelection)
        lv.addWidget(self.events)
        self.tabs.addTab(timeline, "Event timeline")

        covered = QtWidgets.QWidget()
        cv = QtWidgets.QVBoxLayout(covered)
        cv.setContentsMargins(5, 5, 5, 5)
        self.coverage = self.readonly_text("Collection limits will appear after analysis.")
        cv.addWidget(self.coverage)
        self.tabs.addTab(covered, "Coverage & limits")
        self._navigate(0)

        footer = QtWidgets.QHBoxLayout()
        self.progress = QtWidgets.QProgressBar()
        self.progress.setRange(0, 0)
        self.progress.setFixedWidth(100)
        self.progress.hide()
        footer.addWidget(self.progress)
        self.status = self.label("Ready. Run diagnostics to get started.", "Muted")
        footer.addWidget(self.status, 1)
        self.copy_button = QtWidgets.QPushButton("Copy selection")
        self.copy_button.setObjectName("Secondary")
        self.copy_button.clicked.connect(self.copy_selection)
        footer.addWidget(self.copy_button)
        self.save_button = QtWidgets.QPushButton("Export JSON…")
        self.save_button.setObjectName("Secondary")
        self.save_button.setEnabled(False)
        self.save_button.clicked.connect(self.export_json)
        footer.addWidget(self.save_button)
        main.addLayout(footer)

    def _theme_selected(self, label):
        # Only an explicit user change is persisted. Opening the GUI or
        # receiving a system theme update never changes the user's setting.
        choice = normalize_theme(label.lower())
        if choice == self.theme_preference:
            return
        self.theme_preference = choice
        self.settings.setValue(THEME_SETTINGS_KEY, choice)
        self._apply_theme()

    def _apply_theme(self):
        app = QtWidgets.QApplication.instance()
        resolved = resolve_theme(self.theme_preference, system_prefers_dark(app))
        if resolved == self.effective_theme:
            return
        self.effective_theme = resolved
        self.setStyleSheet(DARK_STYLE if resolved == "dark" else LIGHT_STYLE)

    def _system_theme_changed(self, *_args):
        if self.theme_preference == "system":
            self._apply_theme()

    def _navigate(self, index):
        for i, button in enumerate(self.navbuttons):
            button.setChecked(i == index)

    def analyze(self):
        if self.running: return
        try:
            cmd = build_command(
                self.args.whydown_bin, demo=self.args.demo,
                focus=self.focus.currentText(), since=self.since.currentText(),
                max_events=int(self.cap.currentText()), current_boot=self.boot.isChecked(),
                disk_details=self.disk.isChecked(), include_remote_mounts=self.remote.isChecked())
        except ValueError as err:
            QtWidgets.QMessageBox.warning(self, "Invalid scope", str(err))
            return
        self.report = self.raw_report = None
        self.out = bytearray()
        self.err = bytearray()
        self.running = True
        self.analyze_button.setEnabled(False)
        self.save_button.setEnabled(False)
        self.progress.show()
        self.status.setText("Collecting local evidence…")
        self.process.start(cmd[0], cmd[1:])
        self.timer.start(240_000)

    def _stdout(self):
        self.out.extend(bytes(self.process.readAllStandardOutput()))
        if len(self.out) > MAX_REPORT_BYTES:
            self._abort("Diagnostic report exceeded the 8 MiB safety limit.")

    def _stderr(self):
        if len(self.err) < 4096:
            self.err.extend(bytes(self.process.readAllStandardError())[:4096-len(self.err)])
        else:
            self.process.readAllStandardError()

    def _abort(self, why):
        if not self.running: return
        self.running = False
        self.timer.stop()
        if self.process.state() != QtCore.QProcess.ProcessState.NotRunning:
            self.process.kill()
        self.analyze_button.setEnabled(True)
        self.progress.hide()
        self.status.setText(why)
        QtWidgets.QMessageBox.warning(self, "Diagnostic unavailable", why)

    def _process_error(self, error):
        if self.running and error == QtCore.QProcess.ProcessError.FailedToStart:
            self._abort("Cannot start WhyDown diagnostic executable.")

    def _finished(self, code, status):
        self._stdout()
        self._stderr()
        if not self.running: return
        self.running = False
        self.timer.stop()
        self.analyze_button.setEnabled(True)
        self.progress.hide()
        if code or status != QtCore.QProcess.ExitStatus.NormalExit:
            self.status.setText("Diagnostic failed; no report was saved.")
            QtWidgets.QMessageBox.warning(
                self, "Diagnostic failed",
                f"WhyDown exit {code}: {self.err.decode('utf-8', errors='replace')[:500]}")
            return
        try:
            self.report = parse_report(bytes(self.out))
        except (ValueError, UnicodeError, json.JSONDecodeError) as err:
            self.status.setText(f"Cannot parse report: {err}")
            return
        self.raw_report = bytes(self.out)
        self._show_report()

    def _show_report(self):
        r = self.report
        summary = r.get("summary") or {}
        self.metrics["warnings"].setText(str(summary.get("warning_count", 0)))
        self.metrics["info"].setText(str(summary.get("info_count", 0)))
        self.metrics["events"].setText(str(r.get("journal_entries_examined", 0)))
        self.findings.clear()
        for i, finding in enumerate(r["findings"]):
            item = QtWidgets.QTreeWidgetItem([
                str(finding.get("title", "Finding")),
                str(finding.get("severity", ""))])
            item.setData(0, QtCore.Qt.ItemDataRole.UserRole, i)
            if finding.get("severity") == "warning":
                item.setForeground(1, QtGui.QBrush(QtGui.QColor("#b85f28")))
            self.findings.addTopLevelItem(item)
        if r["findings"]:
            self.findings.setCurrentItem(self.findings.topLevelItem(0))
        else:
            self.detail.setPlainText("No classified findings. Not a health guarantee.")
        self.events.clear()
        for e in reversed(r["timeline"][-300:]):
            item = QtWidgets.QTreeWidgetItem([
                str(e.get("timestamp", "")),
                str(e.get("label", e.get("kind", ""))),
                str(e.get("subject") or ""), str(e.get("source") or "")])
            self.events.addTopLevelItem(item)
        for i in range(4): self.events.resizeColumnToContents(i)
        self.coverage.setPlainText(coverage_details(r))
        self.tabs.setCurrentIndex(0)
        self.save_button.setEnabled(True)
        suffix = " · Partial journal window" if r.get("journal_truncated") else ""
        self.status.setText("Analysis complete. Select and copy any evidence passage." + suffix)

    def _finding_changed(self, current, previous):
        del previous
        if current is None or self.report is None: return
        i = current.data(0, QtCore.Qt.ItemDataRole.UserRole)
        if isinstance(i, int) and i < len(self.report["findings"]):
            self.detail.setPlainText(finding_details(self.report["findings"][i]))

    def copy_selection(self):
        index = self.tabs.currentIndex()
        if index != 1:
            editor = self.detail if index == 0 else self.coverage
            selected = editor.textCursor().selectedText().replace("\u2029", "\n")
        else:
            selected = "\n".join(
                "\t".join(item.text(i) for i in range(4))
                for item in self.events.selectedItems())
        if not selected:
            self.status.setText("Select a text passage or timeline row before copying.")
            return False
        QtWidgets.QApplication.clipboard().setText(selected)
        self.status.setText(f"Copied {len(selected)} characters to clipboard.")
        return True

    def export_json(self):
        if self.raw_report is None: return
        path, _ = QtWidgets.QFileDialog.getSaveFileName(
            self, "Save private JSON report", "whydown-report.json", "JSON (*.json)")
        if not path: return
        created = False
        try:
            fd = os.open(path, os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o600)
            created = True
            with os.fdopen(fd, "wb") as out:
                out.write(self.raw_report)
                out.flush()
                os.fsync(out.fileno())
            self.status.setText("Private report saved; inspect before sharing.")
        except OSError as err:
            if created:
                try: os.unlink(path)
                except OSError: pass
            QtWidgets.QMessageBox.warning(self, "Save failed",
                                          f"{err}\nExisting files are never overwritten.")

    def closeEvent(self, event):
        self.timer.stop()
        if self.process.state() != QtCore.QProcess.ProcessState.NotRunning:
            self.process.kill()
            self.process.waitForFinished(1500)
        super().closeEvent(event)


def main(argv=None):
    args = parse_args(argv)
    if QtWidgets is None:
        print("WhyDown Qt GUI requires Python 3 and PyQt6. "
              "Manjaro: sudo pacman -S python python-pyqt6; "
              "Debian/Ubuntu: sudo apt install python3 python3-pyqt6. "
              "The terminal CLI/TUI do not require Qt.", file=sys.stderr)
        return 2
    if not (os.environ.get("DISPLAY") or os.environ.get("WAYLAND_DISPLAY")
            or os.environ.get("QT_QPA_PLATFORM")):
        print("WhyDown Qt GUI needs a graphical desktop. Use whydown tui over SSH.",
              file=sys.stderr)
        return 2
    app = QtWidgets.QApplication(["WhyDown"])
    app.setApplicationName("WhyDown")
    app.setStyle("Fusion")
    gui = WhyDownGUI(args)
    gui.show()
    return app.exec()


if __name__ == "__main__":
    sys.exit(main())
