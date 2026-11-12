"""Smoke checks for the concise project README and its screenshot.

The screenshot is a real WhyDown GUI captured with the fictional demo:
it must be a committed JPEG, not an external link or developer placeholder.
"""
import pathlib
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
README = ROOT / "README.md"
IMAGE = ROOT / "docs/images/whydown-gui-dark.jpg"


class ReadmeQuickStartTests(unittest.TestCase):
    def test_readme_is_short_and_has_usable_entrypoints(self):
        text = README.read_text(encoding="utf-8")
        self.assertLessEqual(len(text.splitlines()), 170)
        for expected in (
            "git clone https://github.com/hamdyaea/WhyDown.git",
            "sh scripts/install.sh",
            "sh scripts/install.sh --desktop",
            "~/.local/bin/whydown gui",
            "~/.local/bin/whydown tui",
            "~/.local/bin/whydown diagnose",
            "sudo pacman -S python python-pyqt6",
            "sudo apt install python3 python3-pyqt6",
            "docs/images/whydown-gui-dark.jpg",
            "docs/GUI.md",
            "docs/TUI.md",
            "docs/PROJECT_HISTORY.md",
        ):
            with self.subTest(expected=expected):
                self.assertIn(expected, text)

    def test_screenshot_is_in_repo_and_is_a_real_jpeg(self):
        self.assertTrue(IMAGE.is_file(), "README screenshot is missing from Git")
        data = IMAGE.read_bytes()
        self.assertGreater(len(data), 10_000)
        self.assertLess(len(data), 1_000_000)
        self.assertTrue(data.startswith(b"\xff\xd8\xff"), "Expected JPEG")
        self.assertTrue(data.endswith(b"\xff\xd9"), "JPEG appears truncated")

    def test_archived_detail_preserved(self):
        archive = ROOT / "docs/PROJECT_HISTORY.md"
        self.assertTrue(archive.is_file())
        self.assertGreater(len(archive.read_text(encoding="utf-8")), 20_000)


if __name__ == "__main__":
    unittest.main()
