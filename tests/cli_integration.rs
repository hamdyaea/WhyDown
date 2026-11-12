use std::process::Command;
use std::fs;

fn whydown(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_whydown"))
        .args(args)
        .output()
        .expect("Whydown binary should launch")
}

#[test]
fn demo_has_evidence_and_no_real_system_data() {
    let result = whydown(&["demo", "--focus", "network"]);
    assert!(result.status.success());
    let output = String::from_utf8(result.stdout).unwrap();
    assert!(output.contains("[DEMO]"));
    assert!(output.contains("LINK_BEFORE_DNS"));
    assert!(output.contains("Demo Linux 1.0"));
    assert!(!output.contains("example-backup.service"));
}

#[test]
fn json_output_has_schema_fields() {
    let result = whydown(&["demo", "--format", "json"]);
    assert!(result.status.success());
    let output = String::from_utf8(result.stdout).unwrap();
    assert!(output.starts_with('{'));
    assert!(output.contains("\"schema_version\": 2"));
    assert!(output.contains("\"summary\""));
    assert!(output.contains("\"subject\""));
    assert!(output.contains("\"source\""));
    assert!(output.contains("\"operstate\""));
    assert!(output.contains("\"service_event_windows\""));
    assert!(output.contains("\"failed_user_units\""));
    assert!(output.contains("\"system_units_checked\""));
    assert!(output.contains("\"dpkg_checked\": true"));
    assert!(output.contains("\"dpkg_problem_count\": 1"));
    assert!(output.contains("\"memory\""));
    assert!(output.contains("\"total_kib\": 4194304"));
    assert!(output.contains("\"pressure\""));
    assert!(output.contains("\"some_avg60_percent_x100\": 15"));
    assert!(output.contains("\"file_handles\""));
    assert!(output.contains("\"inodes\""));
    assert!(output.contains("\"md_arrays\""));
    assert!(output.contains("\"DPKG_INCOMPLETE_PACKAGES\""));
    assert!(output.contains("\"boot_relation\""));
    assert!(output.contains("\"timeline\""));
    assert!(output.contains("\"historical_journal\""));
}

#[test]
fn warning_exit_code_is_opt_in_and_json_output_is_preserved() {
    let normal = whydown(&["demo", "--format", "json"]);
    assert_eq!(normal.status.code(), Some(0));
    let monitored = whydown(&["demo", "--format", "json", "--exit-on-warning"]);
    assert_eq!(monitored.status.code(), Some(1));
    let json = String::from_utf8(monitored.stdout).unwrap();
    assert!(json.contains("\"schema_version\": 2"));
    assert!(json.contains("\"warning_count\""));
    assert!(json.contains("\"DPKG_INCOMPLETE_PACKAGES\""));
}

#[test]
fn current_boot_scope_is_machine_readable() {
    let result = whydown(&["diagnose", "--focus", "services", "--current-boot", "--format", "json"]);
    assert_eq!(result.status.code(), Some(0));
    let json = String::from_utf8(result.stdout).unwrap();
    assert!(json.contains("\"journal_current_boot_only\": true"));
    assert!(json.contains("\"schema_version\": 2"));
}

#[test]
fn overwriting_report_replaces_it_and_keeps_private_permissions() {
    let path = std::env::temp_dir().join(format!(
        "whydown-test-{}-atomic.json", std::process::id()
    ));
    fs::write(&path, "old report").unwrap();
    let result = whydown(&["demo", "--format", "json",
        "--output", path.to_str().unwrap(), "--overwrite"]);
    assert!(result.status.success());
    let saved = fs::read_to_string(&path).unwrap();
    assert!(saved.contains("\"schema_version\": 2"));
    assert_ne!(saved, "old report");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }
    fs::remove_file(path).unwrap();
}

#[test]
fn collector_timings_are_json_safe_and_remote_mounts_are_opt_in() {
    let result = whydown(&["demo", "--format", "json", "--include-remote-mounts"]);
    assert!(result.status.success());
    let report = String::from_utf8(result.stdout).unwrap();
    assert!(report.contains("\"collector_timings\""));
    assert!(report.contains("\"elapsed_ms\": 0"));
    assert!(report.contains("\"name\": \"demo fixture\""));
}

#[test]
fn forced_progress_uses_stderr_without_corrupting_json_stdout() {
    let result = whydown(&["demo", "--format", "json", "--progress"]);
    assert!(result.status.success());
    let report = String::from_utf8(result.stdout).unwrap();
    let stderr = String::from_utf8(result.stderr).unwrap();
    assert!(report.starts_with('{'));
    assert!(report.contains("\"schema_version\": 2"));
    assert!(!report.contains("Diagnostic complete"));
    assert!(stderr.contains("100% (3/3) Diagnostic complete"));
    assert_eq!(stderr.matches("100% (3/3) Diagnostic complete").count(), 1,
        "successful progress must not be printed twice on drop");
    assert!(stderr.contains("[####################]"));
    assert!(!stderr.contains("\u{1b}"), "non-TTY forced progress should not contain ANSI");
}

#[test]
fn saved_report_progress_lines_do_not_overlap_notifications() {
    let path = std::env::temp_dir().join(format!(
        "whydown-test-{}-progress.json", std::process::id()
    ));
    let run = whydown(&["demo", "--format", "json", "--progress",
        "--output", path.to_str().unwrap(), "--overwrite"]);
    assert!(run.status.success());
    assert!(run.stdout.is_empty(), "saved JSON should not appear on stdout");
    let err = String::from_utf8(run.stderr).unwrap();
    assert!(err.contains("Report saved to "));
    assert!(err.contains("100% (3/3) Diagnostic complete"));
    assert_eq!(err.matches("100% (3/3) Diagnostic complete").count(), 1);
    assert!(!err.contains("Report saved to ["),
        "progress must not collide with saved-file notification");
    let data = fs::read_to_string(&path).unwrap();
    assert!(data.starts_with('{') && data.contains("\"schema_version\": 2"));
    fs::remove_file(path).unwrap();
}

#[test]
fn silent_progress_does_not_pollute_stderr() {
    let result = whydown(&["demo", "--format", "json", "--no-progress"]);
    assert!(result.status.success());
    assert!(result.stderr.is_empty());
    assert!(String::from_utf8(result.stdout).unwrap().starts_with('{'));
}

#[test]
fn rejects_unknown_arguments() {
    let result = whydown(&["diagnose", "--since", "100d"]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("Duration"));
}

#[test]
fn refuses_to_overwrite_existing_output() {
    let path = std::env::temp_dir().join(format!("whydown-test-{}-{}.json", std::process::id(), "overwrite"));
    fs::write(&path, "original").unwrap();
    let result = whydown(&["demo", "--format", "json", "--output", path.to_str().unwrap()]);
    assert!(!result.status.success());
    assert_eq!(fs::read_to_string(&path).unwrap(), "original");
    fs::remove_file(&path).unwrap();
}
