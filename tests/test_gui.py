"""Headless model tests for the optional WhyDown Tkinter desktop interface.

Only test pure presentation, CLI argument safety and schema validation here.
No X display, host collectors, network requests or file writes are needed.
"""
import importlib.util
import json
import pathlib
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("whydown_gui", ROOT / "scripts" / "whydown_gui.py")
gui = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(gui)


class TestWhyDownGUI(unittest.TestCase):
    def test_default_command_is_all_read_only_and_current_boot(self):
        cmd = gui.build_command("/opt/bin/whydown")
        self.assertEqual(cmd[:2], ["/opt/bin/whydown", "diagnose"])
        self.assertIn("all", cmd)
        self.assertIn("4h", cmd)
        self.assertIn("--current-boot", cmd)
        self.assertIn("10000", cmd)
        self.assertEqual(cmd[-2:], ["--no-progress", "--current-boot"])
        self.assertNotIn("--output", cmd)
        self.assertNotIn("--exit-on-warning", cmd)

    def test_demo_does_not_collect_host_and_no_network_probes(self):
        cmd = gui.build_command("/opt/bin/whydown", demo=True)
        self.assertEqual(cmd[1], "demo")
        self.assertNotIn("--current-boot", cmd)
        self.assertNotIn("--disk-details", cmd)
        self.assertNotIn("--include-remote-mounts", cmd)

    def test_opt_in_remote_mounts_and_custom_lookback(self):
        cmd = gui.build_command(
            "/opt/bin/whydown", since="90m", focus="system", max_events=5000,
            disk_details=True, include_remote_mounts=True, current_boot=False)
        self.assertIn("90m", cmd)
        self.assertIn("--disk-details", cmd)
        self.assertIn("--include-remote-mounts", cmd)
        self.assertNotIn("--current-boot", cmd)

    def test_reject_unsafe_or_out_of_bounds_options(self):
        for value in ("0h", "31d", "1s", "4h;rm", "1h\nbad", "bad"):
            with self.subTest(since=value):
                with self.assertRaises(ValueError):
                    gui.build_command("whydown", since=value)
        for value in ("foo", "network;rm"):
            with self.assertRaises(ValueError):
                gui.build_command("whydown", focus=value)
        with self.assertRaises(ValueError):
            gui.build_command("whydown", max_events=5001)

    def test_theme_preferences_and_palette_selection(self):
        self.assertEqual(gui.THEMES, ("system", "light", "dark"))
        self.assertEqual(gui.normalize_theme("garbage"), "dark")
        self.assertEqual(gui.normalize_theme(None), "dark")
        self.assertEqual(gui.resolve_theme("system", True), "dark")
        self.assertEqual(gui.resolve_theme("system", False), "light")
        self.assertEqual(gui.resolve_theme("dark", False), "dark")
        self.assertEqual(gui.resolve_theme("light", True), "light")
        self.assertEqual(gui.resolve_theme("invalid", False), "dark")
        self.assertIn("QPlainTextEdit", gui.stylesheet_for("dark", False))
        self.assertIn("#152338", gui.stylesheet_for("dark", False))
        self.assertIn("#f2f6fb", gui.stylesheet_for("light", True))
        self.assertEqual(gui.stylesheet_for("system", True), gui.DARK_STYLE)
        self.assertEqual(gui.stylesheet_for("system", False), gui.LIGHT_STYLE)

    @staticmethod
    def fixture():
        return {
            "schema_version": 2, "version": "0.9.0",
            "journal_checked": True, "journal_truncated": True,
            "journal_entries_examined": 10000,
            "journal_current_boot_only": True,
            "summary": {"warning_count": 1, "info_count": 2},
            "system": {
                "system_units_checked": True, "disk_usage_checked": True,
                "service_statuses": [{
                    "unit": "lightdm.service", "active_state": "active",
                    "sub_state": "running", "result": "success", "n_restarts": 1
                }]
            },
            "collection_notes": ["Journal truncated; narrower window advised"],
            "findings": [{
                "id": "SERVICE_RESTART_OBSERVED", "severity": "info",
                "confidence": "observed", "scope": "historical_journal",
                "title": "Systemd restart scheduled",
                "explanation": "A journal message was observed, not proof of readiness.",
                "evidence": ["systemd showed active"], "checks": ["systemctl status lightdm"]
            }],
            "timeline": [{
                "timestamp": "2026-11-12T15:01:42Z", "label": "USB detach",
                "subject": "1-1.2", "source": "kernel"
            }]
        }

    def test_parse_and_display_json_v2(self):
        original = self.fixture()
        parsed = gui.parse_report(json.dumps(original).encode("utf-8"))
        self.assertEqual(parsed, original)
        self.assertIn("1 warnings", gui.headline(parsed))
        self.assertIn("PARTIAL", gui.headline(parsed))
        details = gui.finding_details(parsed["findings"][0])
        self.assertIn("Systemd restart scheduled", details)
        self.assertIn("systemd showed active", details)
        self.assertIn("systemctl status lightdm", details)
        coverage = gui.coverage_details(parsed)
        self.assertIn("lightdm.service", coverage)
        self.assertIn("Journal window partial: True", coverage)
        self.assertIn("connectivity", coverage)

    def test_detailed_incident_has_specific_wifi_and_systemd_evidence(self):
        data = self.fixture()
        data["journal_truncated"] = False
        data["system"].update({
            "interfaces": [{"name": "wlan0", "operstate": "up", "rx_errors": 0,
                            "tx_errors": 0, "rx_dropped": 2, "tx_dropped": 0}],
            "ipv4_default_route": True, "ipv6_default_route": False,
            "dns_nameservers": 1, "resolved_active": True,
            "failed_units": [],
        })
        wifi = {"id": "LINK_DISCONNECTS", "title": "Network link interruption on wlan0",
                "severity": "warning", "scope": "historical_journal",
                "confidence": "observed", "explanation": "Carrier loss was observed",
                "evidence": ["wlan0 affected"], "checks": ["ip -s link show dev wlan0"]}
        svc = {"id": "SERVICE_FAILURE_LOGS", "title": "Historical failure: sshd.service",
               "severity": "warning", "scope": "historical_journal",
               "confidence": "observed", "explanation": "Process failed",
               "evidence": ["sshd.service"], "checks": ["systemctl status sshd.service"]}
        data["findings"] = [wifi, svc]
        data["timeline"] = [
            {"timestamp": "2026-11-12T15:01:42Z", "kind": "link_disconnected",
             "subject": "wlan0", "source": "NetworkManager",
             "diagnostic_hint": "reason=2; transition=activated_to_disconnected"},
            {"timestamp": "2026-11-12T15:02:02Z", "kind": "service_failure",
             "subject": "sshd.service", "source": "systemd",
             "diagnostic_hint": "exit_status=1/FAILURE"},
        ]
        self.assertEqual(len(gui.related_events(data, wifi)), 1)
        row = gui.overview_row(data, wifi)
        self.assertEqual(row["affected"], "wlan0")
        self.assertEqual(row["events"], "1")
        wifi_dossier = gui.detailed_incident(data, wifi)
        for term in ("wlan0", "NetworkManager", "2026-11-12T15:01:42Z",
                     "reason=2", "operstate=up", "ip -s link show dev wlan0",
                     "not a confirmed root cause"):
            self.assertIn(term, wifi_dossier)
        self.assertNotIn("exit_status=1/FAILURE", wifi_dossier)
        self.assertIn("ip -s link show dev wlan0", wifi_dossier)
        self.assertIn("nmcli device show wlan0", wifi_dossier)
        svc_dossier = gui.detailed_incident(data, svc)
        self.assertIn("sshd.service", svc_dossier)
        self.assertIn("exit_status=1/FAILURE", svc_dossier)
        self.assertIn("CURRENT SYSTEMD STATE", svc_dossier)
        self.assertNotIn("reason=2", svc_dossier)
        self.assertIn("systemctl status sshd.service --no-pager", svc_dossier)
        self.assertIn("journalctl -u sshd.service --no-pager -n 100", svc_dossier)
        forged = [dict(data["timeline"][0], subject="wlan0;rm -rf /")]
        self.assertFalse(any("rm" in cmd for cmd in
                             gui.targeted_readonly_commands(data, wifi, forged)))
        standalone = gui.detailed_incident(
            data, {"id": "FAILED_SYSTEMD_UNITS", "title": "Current units failed",
                   "severity": "warning", "scope": "current_snapshot",
                   "explanation": "Failure observed", "evidence": [], "checks": []})
        self.assertIn("Current state", standalone)
        self.assertIn("No event of this class", standalone)
        self.assertIn("No specific check supplied", standalone)

    def test_historical_opera_exit_24_has_user_scope_and_safe_dossier_commands(self):
        data = self.fixture()
        unit = "app-opera@d824a1805db343d680ca5e5932b22b70.service"
        data["system"]["system_units_checked"] = True
        data["system"]["failed_units"] = []
        data["system"]["failed_user_units"] = []
        data["system"]["service_statuses"] = [{
            "unit": unit, "manager": "user", "transient": True,
            "desktop_entry": True, "exec_main_status": 24,
            "load_state": "loaded", "active_state": "inactive",
            "sub_state": "dead", "result": "exit-code", "n_restarts": 0,
        }]
        data["timeline"] = [
            {"timestamp": "2026-11-13T10:18:50Z",
             "kind": "service_failure", "subject": unit,
             "source": "systemd",
             "diagnostic_hint": "exit_status=24; process_code=exited; systemd_result=exit-code"},
            {"timestamp": "2026-11-13T10:14:48Z",
             "kind": "service_failure", "subject": None,
             "source": "systemd", "diagnostic_hint": None},
        ]
        desktop = {
            "id": "DESKTOP_LAUNCH_EXIT_LOGS",
            "title": "Historical desktop launcher exit",
            "severity": "info", "confidence": "moderate",
            "scope": "historical_journal",
            "explanation": "User-session GUI launcher exited",
            "evidence": [f"Desktop app unit(s): {unit}", "exit_status=24"],
            "checks": ["Verify intended document opened"],
        }
        data["findings"] = [desktop]
        matching = gui.related_events(data, desktop)
        self.assertEqual(len(matching), 1)
        self.assertEqual(matching[0]["subject"], unit)
        info = gui.detailed_incident(data, desktop)
        self.assertIn("exit_status=24", info)
        self.assertIn("[user manager]", info)
        self.assertIn("transient=True", info)
        self.assertIn("desktop_launcher=True", info)
        self.assertIn("systemctl --user status", info)
        self.assertIn(f"journalctl --user -u {unit}", info)
        self.assertNotIn("systemctl status " + unit, info)
        self.assertNotIn("2026-11-13T10:14:48Z", info.split(
            "CLASSIFIED JOURNAL EVENTS")[1].split("SPECIFIC COLLECTED EVIDENCE")[0])

    def test_unit_scoped_restart_and_temporal_correlation_do_not_mix_events(self):
        data = self.fixture()
        data["timeline"] = [
            {"timestamp": "2026-11-12T15:00:00Z", "kind": "service_restart_scheduled",
             "subject": "alpha.service", "source": "systemd"},
            {"timestamp": "2026-11-12T15:00:01Z", "kind": "service_restart_scheduled",
             "subject": "beta.service", "source": "systemd"},
            {"timestamp": "2026-11-12T15:00:02Z", "kind": "link_disconnected",
             "subject": "wlan0", "source": "kernel"},
            {"timestamp": "2026-11-12T15:00:03Z", "kind": "dns_failure",
             "subject": None, "source": "resolved"},
            {"timestamp": "2026-11-12T16:00:00Z", "kind": "dns_failure",
             "subject": None, "source": "resolved"},
        ]
        restart = {
            "id": "SERVICE_RESTART_OBSERVED",
            "evidence": ["beta.service: restart scheduled at 2026-11-12T15:00:01Z"],
        }
        self.assertEqual(
            [e["subject"] for e in gui.related_events(data, restart)],
            ["beta.service"])
        pair = {
            "id": "LINK_BEFORE_DNS",
            "evidence": ["Link event: 2026-11-12T15:00:02Z",
                         "DNS event: 2026-11-12T15:00:03Z (1 s later)"],
        }
        self.assertEqual(
            [e["timestamp"] for e in gui.related_events(data, pair)],
            ["2026-11-12T15:00:02Z", "2026-11-12T15:00:03Z"])

    def test_dossier_events_are_bounded_and_not_claimed_complete(self):
        data = self.fixture()
        finding = {"id": "DNS_FAILURES", "severity": "warning", "title": "DNS", "evidence": []}
        data["timeline"] = [
            {"timestamp": "2026-11-12T15:00:00Z", "kind": "dns_failure",
             "subject": None, "source": "systemd-resolved", "diagnostic_hint": "dns_reply=SERVFAIL"}
            for _ in range(100)
        ]
        dossier = gui.detailed_incident(data, finding)
        self.assertIn("Only the newest 80", dossier)
        self.assertIn("PARTIAL JOURNAL", dossier)
        self.assertIn("matching events in retained timeline: 100", dossier.lower())

    def test_report_validation_does_not_guess(self):
        for bad in ({}, {"schema_version": 4}, {"schema_version": 2, "system": {}}):
            with self.subTest(data=bad):
                with self.assertRaises(ValueError):
                    gui.parse_report(json.dumps(bad))
        with self.assertRaises(ValueError):
            gui.parse_report("not-json")
        with self.assertRaises(ValueError):
            gui.parse_report(b"x" * (gui.MAX_REPORT_BYTES + 1))

    def test_cli_defaults(self):
        args = gui.parse_args(["--whydown-bin", "/tmp/whydown"])
        self.assertEqual((args.focus, args.since, args.max_events),
                         ("all", "4h", 10000))
        self.assertFalse(args.demo)
        args = gui.parse_args(["--whydown-bin", "/tmp/whydown",
                               "--focus", "network", "--since", "90m", "--demo"])
        self.assertTrue(args.demo)
        self.assertEqual(args.since, "90m")


if __name__ == "__main__":
    unittest.main()
