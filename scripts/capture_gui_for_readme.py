#!/usr/bin/env python3
"""Generate a privacy-safe README screenshot of the REAL Qt6 GUI.

The program is run in fictional demo mode under Xvfb. No host journal,
network access, user identifiers or secrets are included in the image.
Run with: xvfb-run -a python3 scripts/capture_gui_for_readme.py /path/to/whydown

Write docs/images/whydown-gui-dark.jpg locally; this is a developer tool.
"""
from __future__ import annotations
import argparse
import importlib.util
import os
import pathlib
import sys
import tempfile
import time

from PyQt6 import QtCore, QtWidgets

ROOT = pathlib.Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location(
    "whydown_gui", ROOT / "scripts" / "whydown_gui.py")
MOD = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MOD)


def capture(binary: pathlib.Path, output: pathlib.Path) -> pathlib.Path:
    if not binary.is_file():
        raise FileNotFoundError(f"WhyDown binary not found: {binary}")
    # Always start without any existing user appearance settings.
    with tempfile.TemporaryDirectory(prefix="whydown-readme-demo-") as settings_home:
        os.environ["XDG_CONFIG_HOME"] = settings_home
        app = QtWidgets.QApplication(["WhyDown README screenshot"])
        app.setStyle("Fusion")
        args = MOD.parse_args(["--whydown-bin", str(binary), "--demo",
                               "--current-boot"])
        gui = MOD.WhyDownGUI(args)
        gui.resize(1250, 820)
        gui.show()
        app.processEvents()
        assert gui.theme_preference == "dark"
        assert gui.effective_theme == "dark"
        gui.analyze()
        deadline = time.monotonic() + 20
        while gui.report is None and time.monotonic() < deadline:
            app.processEvents()
            time.sleep(0.05)
        if gui.report is None or gui.report.get("demo") is not True:
            raise RuntimeError("Fictional demo report did not render")
        if gui.findings.topLevelItemCount() == 0:
            raise RuntimeError("Fictional report unexpectedly has no findings")
        app.processEvents()
        image = gui.grab().toImage()
        image = image.scaledToWidth(
            1100, QtCore.Qt.TransformationMode.SmoothTransformation)
        output.parent.mkdir(parents=True, exist_ok=True)
        if not image.save(str(output), "JPG", 78):
            raise RuntimeError("Could not encode screenshot as JPEG")
        gui.close()
        app.processEvents()
        return output


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("binary", type=pathlib.Path)
    parser.add_argument("--output", type=pathlib.Path,
                        default=ROOT / "docs/images/whydown-gui-dark.jpg")
    args = parser.parse_args()
    path = capture(args.binary.resolve(), args.output)
    print(f"Generated fictional Qt6 screenshot: {path}")


if __name__ == "__main__":
    sys.exit(main())
