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
import subprocess
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

def load_brand_pixmap(binary):
    """Read the embedded project icon from the same trusted WhyDown binary.

    No separate asset download, writes, privileged access or shell execution.
    Returns a null pixmap when testing a non-WhyDown fixture.
    """
    pixmap = QtGui.QPixmap()
    try:
        result = subprocess.run(
            [os.fspath(binary), "icon"], stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL, timeout=3, check=True)
        if len(result.stdout) <= 100_000:
            pixmap.loadFromData(result.stdout, "PNG")
    except (OSError, ValueError, subprocess.SubprocessError):
        pass
    return pixmap


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


# Matching uses rule IDs rather than keywords from potentially sensitive logs.
# Snapshot-only rules deliberately have no historical events attached.
FINDING_EVENT_KINDS = {
    "LINK_DISCONNECTS": ("link_disconnected",),
    "DHCP_FAILURES": ("dhcp_problem",),
    "DNS_FAILURES": ("dns_failure",),
    "NETWORK_UNREACHABLE": ("network_unreachable",),
    "LINK_BEFORE_DNS": ("link_disconnected", "dns_failure"),
    "DHCP_BEFORE_DNS": ("dhcp_problem", "dns_failure"),
    "WIFI_LOCAL_ROAM_OBSERVED": ("wifi_roam_start", "wifi_associated"),
    "SERVICE_FAILURE_LOGS": ("service_failure",),
    "SERVICE_RESTART_OBSERVED": ("service_restart_scheduled", "service_failure"),
    "OOM_EVENTS": ("out_of_memory",),
    "STORAGE_ERRORS": ("storage_error",),
    "FD_EXHAUSTION_LOGS": ("file_handle_exhaustion",),
    "USB_DISCONNECT_BURST": ("usb_disconnect",),
    "USB_TRANSPORT_STATUS": ("usb_transport_error",),
}


def related_events(report, finding):
    kinds = FINDING_EVENT_KINDS.get(finding.get("id"), ())
    return [event for event in report.get("timeline", [])
            if isinstance(event, dict) and event.get("kind") in kinds]


def affected_targets(report, finding, events=None):
    events = related_events(report, finding) if events is None else events
    targets = sorted({str(e["subject"]) for e in events if e.get("subject")})
    system = report.get("system") or {}
    if finding.get("id") == "FAILED_SYSTEMD_UNITS":
        targets = [str(x) for x in system.get("failed_units", [])]
    elif finding.get("id") == "FAILED_USER_SYSTEMD_UNITS":
        targets = [str(x) for x in system.get("failed_user_units") or []]
    if len(targets) > 3:
        return ", ".join(targets[:3]) + f" (+{len(targets)-3})"
    return ", ".join(targets) if targets else "Not identified"


def overview_row(report, finding):
    events = related_events(report, finding)
    first = events[0].get("timestamp", "—") if events else (
        "Current state" if finding.get("scope") == "current_snapshot" else "—")
    summary = (finding.get("evidence") or [finding.get("explanation", "")])[0]
    return {
        "severity": str(finding.get("severity", "?")).upper(),
        "title": str(finding.get("title", "Finding")),
        "affected": affected_targets(report, finding, events),
        "observed": str(first),
        "events": str(len(events)) if events else "—",
        "preview": str(summary),
    }


def detailed_incident(report, finding):
    """Human-readable dossier from actual collected data, never inferred causes.

    All entries are already privacy-filtered by the Rust collector. A timeline
    sample is not an exhaustive journal transcript, and current snapshots are
    not evidence that a historical error persists.
    """
    events = related_events(report, finding)
    row = overview_row(report, finding)
    sysinfo = report.get("system") or {}
    lines = [
        str(finding.get("title", "Finding")), "",
        "INCIDENT IDENTIFICATION",
        f"Rule: {finding.get('id', 'unknown')}",
        f"Severity: {row['severity']}",
        f"Confidence: {finding.get('confidence', 'unknown')}",
        f"Scope: {finding.get('scope', 'unknown')}",
        f"Affected: {row['affected']}",
        f"First matching event in retained timeline: {row['observed']}",
        f"Matching events in retained timeline: {len(events)}",
    ]
    if events:
        lines.append(f"Last matching event in retained timeline: {events[-1].get('timestamp', '?')}")
    lines.extend(["", "WHAT WHYDOWN OBSERVED",
                  str(finding.get("explanation", "")), "",
                  "CLASSIFIED JOURNAL EVENTS (time / source / affected / safe diagnostic code)"])
    if events:
        for i, event in enumerate(events[-80:], 1):
            lines.append(
                f"{i:02d}  {event.get('timestamp', '?')}  "
                f"{event.get('kind', '?')}  source={event.get('source') or 'unknown'}  "
                f"subject={event.get('subject') or 'unknown'}")
            signal = event.get("diagnostic_hint")
            if signal:
                lines.append(f"    Indicator: {signal}")
        if len(events) > 80:
            lines.append(f"Only the newest 80 matching timeline entries are displayed here (of {len(events)} retained).")
    else:
        lines.append("No event of this class is present in the retained timeline; "
                     "this rule may be snapshot-only or older events may be outside the timeline.")
    lines.extend(["", "SPECIFIC COLLECTED EVIDENCE"])
    for evidence in finding.get("evidence") or []:
        lines.append(" • " + str(evidence))
    if not finding.get("evidence"):
        lines.append(" No additional evidence recorded")

    if finding.get("id") in (
        "LINK_DISCONNECTS", "DHCP_FAILURES", "DNS_FAILURES",
        "NETWORK_UNREACHABLE", "LINK_BEFORE_DNS", "DHCP_BEFORE_DNS",
        "WIFI_LOCAL_ROAM_OBSERVED", "RECENT_INTERFACE_DROPS", "IFACE_ERROR_COUNTERS",
        "ROUTE_MISSING", "IPV4_ROUTE_MISSING", "DNS_UNCONFIGURED", "DNS_STUB_INACTIVE",
    ):
        lines.extend(["", "CURRENT NETWORK SNAPSHOT (NOT VERIFIED AT INCIDENT TIME)"])
        lines.extend([
            f"IPv4 default route: {sysinfo.get('ipv4_default_route', 'unverified')}",
            f"IPv6 default route: {sysinfo.get('ipv6_default_route', 'unverified')}",
            f"Configured DNS nameserver count: {sysinfo.get('dns_nameservers', 'unverified')}",
            f"systemd-resolved active: {sysinfo.get('resolved_active', 'unverified')}",
        ])
        known = {e.get("subject") for e in events if e.get("subject")}
        for interface in sysinfo.get("interfaces") or []:
            if not known or interface.get("name") in known:
                lines.append(
                    f" • {interface.get('name', '?')}: operstate={interface.get('operstate', '?')}, "
                    f"rx_errors={interface.get('rx_errors', '?')}, "
                    f"tx_errors={interface.get('tx_errors', '?')}, "
                    f"rx_dropped={interface.get('rx_dropped', '?')}, "
                    f"tx_dropped={interface.get('tx_dropped', '?')}")
        for incident in (report.get("network_incidents") or [])[-12:]:
            if known and incident.get("subject") not in known:
                continue
            lines.append(
                f" • Incident cluster: {incident.get('subject') or 'unknown interface'} "
                f"{incident.get('start', '?')} → {incident.get('end', '?')}: "
                f"link={incident.get('link_events', 0)}, dhcp={incident.get('dhcp_events', 0)}, "
                f"dns={incident.get('dns_events', 0)}, unreachable={incident.get('unreachable_events', 0)}")
    elif finding.get("id") in (
        "FAILED_SYSTEMD_UNITS", "FAILED_USER_SYSTEMD_UNITS",
        "SERVICE_FAILURE_LOGS", "SERVICE_RESTART_OBSERVED",
    ):
        lines.extend(["", "CURRENT SYSTEMD STATE (SEPARATE FROM HISTORICAL ERRORS)"])
        if sysinfo.get("system_units_checked"):
            failed = sysinfo.get("failed_units") or []
            lines.append("Currently failed system units: " + (", ".join(map(str, failed)) or "none"))
        else:
            lines.append("Current system-unit query: unavailable")
        user_failed = sysinfo.get("failed_user_units")
        lines.append("Currently failed user units: "
                     + (", ".join(map(str, user_failed)) if user_failed is not None else "not verified"))
        for state in sysinfo.get("service_statuses") or []:
            lines.append(
                f" • {state.get('unit', '?')}: "
                f"load={state.get('load_state', '?')}, "
                f"active={state.get('active_state', '?')}, "
                f"sub={state.get('sub_state', '?')}, "
                f"result={state.get('result', '?')}, "
                f"restarts={state.get('n_restarts', 'unknown')}")
        for group in (report.get("service_event_windows") or [])[-12:]:
            lines.append(
                f" • Historical failure cluster: {group.get('start', '?')} → {group.get('end', '?')}, "
                f"messages={group.get('entry_count', '?')}, "
                f"units={', '.join(group.get('units') or []) or 'unattributed'}")

    lines.extend(["", "SUGGESTED READ-ONLY CHECKS"])
    for check in finding.get("checks") or []:
        lines.append(" • " + str(check))
    if not finding.get("checks"):
        lines.append(" No specific check supplied")
    lines.extend(["", "CONFIDENCE AND LIMITATIONS",
                  "This is a local snapshot plus classified historical events, not a confirmed root cause.",
                  "Current status may have changed since a historical incident.",
                  "No raw journal messages or credentials are retained in this report.",
                  "The timeline contains at most 300 recent classified events, not necessarily all incidents."])
    if report.get("journal_truncated"):
        lines.append("PARTIAL JOURNAL: older records were excluded by the event limit.")
    if not report.get("journal_checked"):
        lines.append("UNVERIFIED JOURNAL: journalctl was inaccessible or unavailable.")
    return "\n".join(lines)


def event_dossier(event):
    lines = ["CLASSIFIED JOURNAL EVENT", "",
             f"Time (UTC): {event.get('timestamp', 'unknown')}",
             f"Category: {event.get('label', event.get('kind', 'unknown'))}",
             f"Source: {event.get('source') or 'not identified'}",
             f"Affected subject: {event.get('subject') or 'not identified'}",
             f"Safe diagnostic indicator: {event.get('diagnostic_hint') or 'not available'}",
             "", "The journal message itself was not retained for privacy.",
             "A journal event does not prove that the fault is still present."]
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
        self.brand_pixmap = load_brand_pixmap(self.args.whydown_bin)
        if not self.brand_pixmap.isNull():
            self.setWindowIcon(QtGui.QIcon(self.brand_pixmap))
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
        brand_row = QtWidgets.QHBoxLayout()
        brand_row.setSpacing(8)
        self.brandmark = QtWidgets.QLabel()
        self.brandmark.setFixedSize(42, 42)
        if not self.brand_pixmap.isNull():
            self.brandmark.setPixmap(self.brand_pixmap.scaled(
                42, 42, QtCore.Qt.AspectRatioMode.KeepAspectRatio,
                QtCore.Qt.TransformationMode.SmoothTransformation))
        brand_row.addWidget(self.brandmark)
        brand_row.addWidget(self.label("WhyDown", "Brand"), 1)
        nav.addLayout(brand_row)
        nav.addWidget(self.label("LOCAL SYSTEM DIAGNOSTICS", "SideCaption"))
        nav.addSpacing(46)
        nav.addWidget(self.label("WORKSPACE", "SideCaption"))
        self.navbuttons = []
        for n, name in enumerate(("◉  Overview", "▣  Incident details", "◷  Timeline", "▤  Coverage")):
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
        summary.addWidget(self.label("02   Incident overview", "CardTitle"))
        summary.addStretch()
        summary.addWidget(self.label("Select an alert to preview · Double-click for full details", "Muted"))
        main.addLayout(summary)

        self.tabs = QtWidgets.QTabWidget()
        self.tabs.currentChanged.connect(self._navigate)
        main.addWidget(self.tabs, 1)
        self.tabs.setMinimumHeight(265)
        finds = QtWidgets.QWidget()
        layout = QtWidgets.QVBoxLayout(finds)
        layout.setContentsMargins(5, 5, 5, 5)
        self.findings = QtWidgets.QTreeWidget()
        self.findings.setHeaderLabels(
            ["Severity", "Incident / observation", "Affected", "First event (UTC)", "Events"])
        self.findings.setRootIsDecorated(False)
        self.findings.setAlternatingRowColors(True)
        self.findings.setSelectionMode(QtWidgets.QAbstractItemView.SelectionMode.SingleSelection)
        self.findings.setMinimumHeight(190)
        self.findings.currentItemChanged.connect(self._finding_changed)
        self.findings.itemDoubleClicked.connect(self.open_finding)
        self.findings.setToolTip("Double-click an incident for its evidence, timestamps and affected units")
        layout.addWidget(self.findings, 2)
        layout.addWidget(self.label("SELECTED INCIDENT PREVIEW  ·  Double-click the row for all details",
                                    "Muted"))
        self.overview_preview = self.readonly_text("Run diagnostics to view the incident list.")
        self.overview_preview.setMaximumHeight(125)
        layout.addWidget(self.overview_preview, 1)
        self.tabs.addTab(finds, "Incident overview")

        details_page = QtWidgets.QWidget()
        details_layout = QtWidgets.QVBoxLayout(details_page)
        details_layout.setContentsMargins(8, 8, 8, 8)
        top = QtWidgets.QHBoxLayout()
        self.back_button = QtWidgets.QPushButton("← Back to overview")
        self.back_button.setObjectName("Secondary")
        self.back_button.clicked.connect(lambda: self.tabs.setCurrentIndex(0))
        top.addWidget(self.back_button)
        top.addStretch()
        top.addWidget(self.label("Read-only evidence · Ctrl+C to copy", "Muted"))
        details_layout.addLayout(top)
        self.detail = self.readonly_text("Double-click an alert in the overview to see its full dossier.")
        details_layout.addWidget(self.detail)
        self.tabs.addTab(details_page, "Incident details")

        timeline = QtWidgets.QWidget()
        lv = QtWidgets.QVBoxLayout(timeline)
        lv.setContentsMargins(5, 5, 5, 5)
        self.events = QtWidgets.QTreeWidget()
        self.events.setHeaderLabels(["Time (UTC)", "Event", "Subject", "Source"])
        self.events.setRootIsDecorated(False)
        self.events.setAlternatingRowColors(True)
        self.events.setSelectionMode(QtWidgets.QAbstractItemView.SelectionMode.ExtendedSelection)
        self.events.itemDoubleClicked.connect(self.open_event)
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
        for i, finding in sorted(
                enumerate(r["findings"]),
                key=lambda indexed: (indexed[1].get("severity") != "warning", indexed[0])):
            info = overview_row(r, finding)
            item = QtWidgets.QTreeWidgetItem([
                info["severity"], info["title"], info["affected"],
                info["observed"], info["events"]])
            item.setData(0, QtCore.Qt.ItemDataRole.UserRole, i)
            item.setToolTip(1, info["title"] + "\n" + info["preview"])
            if finding.get("severity") == "warning":
                item.setForeground(0, QtGui.QBrush(QtGui.QColor("#e98d4f")))
            self.findings.addTopLevelItem(item)
        for column in range(5):
            self.findings.resizeColumnToContents(column)
        self.findings.setColumnWidth(1, min(440, max(240, self.findings.columnWidth(1))))
        self.detail.setPlainText(
            "Double-click an incident in the overview for its detailed dossier.")
        if r["findings"]:
            self.findings.setCurrentItem(self.findings.topLevelItem(0))
        else:
            self.overview_preview.setPlainText("No classified findings. Not a health guarantee.")
        self.events.clear()
        for e in reversed(r["timeline"][-300:]):
            item = QtWidgets.QTreeWidgetItem([
                str(e.get("timestamp", "")),
                str(e.get("label", e.get("kind", ""))),
                str(e.get("subject") or ""), str(e.get("source") or "")])
            item.setData(0, QtCore.Qt.ItemDataRole.UserRole, e)
            self.events.addTopLevelItem(item)
        for i in range(4): self.events.resizeColumnToContents(i)
        self.coverage.setPlainText(coverage_details(r))
        self.tabs.setCurrentIndex(0)
        self.save_button.setEnabled(True)
        suffix = " · Partial journal window" if r.get("journal_truncated") else ""
        self.status.setText("Analysis complete. Select an alert, then double-click for the full incident dossier." + suffix)

    def _finding_changed(self, current, previous):
        del previous
        if current is None or self.report is None:
            return
        i = current.data(0, QtCore.Qt.ItemDataRole.UserRole)
        if isinstance(i, int) and 0 <= i < len(self.report["findings"]):
            finding = self.report["findings"][i]
            info = overview_row(self.report, finding)
            preview = (
                f"{info['severity']}  •  {info['title']}\n"
                f"Affected: {info['affected']}  |  First event: {info['observed']}\n"
                f"{info['preview']}\n"
                "Double-click this row to inspect matching event timestamps, "
                "safe diagnostic codes, current system state and suggested checks.")
            self.overview_preview.setPlainText(preview)

    def open_finding(self, item, _column=0):
        if item is None or self.report is None:
            return
        index = item.data(0, QtCore.Qt.ItemDataRole.UserRole)
        if not isinstance(index, int) or not 0 <= index < len(self.report["findings"]):
            return
        self.detail.setPlainText(detailed_incident(self.report, self.report["findings"][index]))
        self.detail.moveCursor(QtGui.QTextCursor.MoveOperation.Start)
        self.tabs.setCurrentIndex(1)

    def open_event(self, item, _column=0):
        if item is None or self.report is None:
            return
        event = item.data(0, QtCore.Qt.ItemDataRole.UserRole)
        if not isinstance(event, dict):
            return
        self.detail.setPlainText(event_dossier(event))
        self.detail.moveCursor(QtGui.QTextCursor.MoveOperation.Start)
        self.tabs.setCurrentIndex(1)

    def copy_selection(self):
        index = self.tabs.currentIndex()
        if index != 2:
            editor = {0: self.overview_preview, 1: self.detail, 3: self.coverage}[index]
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
