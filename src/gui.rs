//! Optional offline desktop interface built on the existing Rust diagnostics.
//!
//! The Python/Tkinter GUI is embedded directly into each WhyDown binary.
//! Releasing the CLI still requires no Python dependency; desktop users opt in
//! to Python 3 + Tk at runtime. No shell invocation, file write or network use.

use crate::cli::Cli;
use std::process::Command;

fn lookback(seconds: u64) -> String {
    if seconds % 86_400 == 0 {
        format!("{}d", seconds / 86_400)
    } else if seconds % 3_600 == 0 {
        format!("{}h", seconds / 3_600)
    } else {
        format!("{}m", seconds / 60)
    }
}

pub fn run(cli: Cli) -> Result<u8, String> {
    let binary = std::env::current_exe()
        .map_err(|e| format!("Cannot locate WhyDown executable: {e}"))?;
    let mut launcher = Command::new("python3");
    launcher
        .arg("-B") // do not write bytecode caches
        .arg("-c")
        .arg(include_str!("../scripts/whydown_gui.py"))
        .arg("--whydown-bin")
        .arg(binary)
        .arg("--focus")
        .arg(cli.focus.as_str())
        .arg("--since")
        .arg(lookback(cli.since_seconds))
        .arg("--max-events")
        .arg(cli.max_events.to_string());
    if cli.tui_demo {
        launcher.arg("--demo");
    }
    if cli.current_boot {
        launcher.arg("--current-boot");
    }
    if cli.disk_details {
        launcher.arg("--disk-details");
    }
    if cli.include_remote_mounts {
        launcher.arg("--include-remote-mounts");
    }
    let status = launcher.status().map_err(|e| {
        format!("Cannot launch graphical WhyDown: {e}. \
                 Install Python 3 with Tkinter, or use 'whydown tui'.")
    })?;
    Ok(if status.success() { 0 } else { 2 })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_precise_validated_lookback() {
        assert_eq!(lookback(3_600), "1h");
        assert_eq!(lookback(14_400), "4h");
        assert_eq!(lookback(5_400), "90m");
        assert_eq!(lookback(86_400), "1d");
        assert_eq!(lookback(604_800), "7d");
    }

    #[test]
    fn script_is_embedded_and_has_no_external_fetch() {
        let script = include_str!("../scripts/whydown_gui.py");
        assert!(script.contains("def build_command"));
        assert!(script.contains("subprocess.Popen"));
        assert!(script.contains("class WhyDownGUI"));
    }
}
