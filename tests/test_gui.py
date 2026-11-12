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
