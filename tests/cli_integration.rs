use std::process::Command;
use std::fs;
use std::io::Write;

fn whydown(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_whydown"))
        .args(args)
        .output()
        .expect("Whydown binary should launch")
}

#[test]
fn nagios_checks_report_all_four_states_with_standards_compliant_perfdata() {
    let ok = whydown(&["nagios","check","--metric","memory","--demo"]);
    assert_eq!(ok.status.code(), Some(0));
    let o = String::from_utf8(ok.stdout).unwrap();
    assert!(o.starts_with("WHYDOWN OK DEMO - memory 62.50%"));
    assert!(o.contains("| 'mem_used_pct'=62.50%;80;95;0;100"));
    assert!(ok.stderr.is_empty());

    let warning = whydown(&["nagios","--demo"]);
    assert_eq!(warning.status.code(), Some(1));
    let w = String::from_utf8(warning.stdout).unwrap();
    assert!(w.starts_with("WHYDOWN WARNING DEMO -"));
    assert!(w.contains("'failed_units'=1.00;0;0:2;0;"));

    let critical = whydown(&["nagios","check","--metric","memory","--critical","50","--demo"]);
    assert_eq!(critical.status.code(), Some(2));
    assert!(String::from_utf8(critical.stdout).unwrap().starts_with("WHYDOWN CRITICAL DEMO -"));

    let service = whydown(&["nagios","check","--metric","service",
        "--unit","example-backup.service","--demo"]);
    assert_eq!(service.status.code(), Some(0));
    assert!(String::from_utf8(service.stdout).unwrap().contains("'service_active'=1.00;~:;1:;0;1"));
    for invalid in [
        vec!["nagios","--metric","service","--demo"],
        vec!["nagios","--metric","service","--unit","example.service;rm","--demo"],
        vec!["nagios","--metric","memory","--unit","example.service","--demo"],
    ] {
        assert_eq!(whydown(&invalid).status.code(), Some(3));
    }

    let unknown = whydown(&["nagios","check","--metric","not-a-check","--demo"]);
    assert_eq!(unknown.status.code(), Some(3));
    assert!(String::from_utf8(unknown.stdout).unwrap().starts_with("WHYDOWN UNKNOWN -"));
    assert!(unknown.stderr.is_empty());
    let bad = whydown(&["nagios","--metric","all","--warning","70","--demo"]);
    assert_eq!(bad.status.code(), Some(3));
}

#[test]
fn nagios_recording_svg_dashboard_and_private_file_safety() {
    let base = std::env::temp_dir().join(format!("whydown-nagios-test-{}", std::process::id()));
    fs::create_dir_all(&base).unwrap();
    let history = base.join("metric.tsv");
    let svg = base.join("memory.svg");
    let html = base.join("dashboard.html");
    let _ = fs::remove_file(&history);
    let _ = fs::remove_file(&svg);
    let _ = fs::remove_file(&html);
    let h = history.to_str().unwrap();
    let g = svg.to_str().unwrap();
    let d = html.to_str().unwrap();

    for _ in 0..2 {
        let recorded = whydown(&["nagios","check","--demo","--record",h]);
        assert_eq!(recorded.status.code(), Some(1));
    }
    let lines: Vec<_> = fs::read_to_string(&history).unwrap().lines()
        .map(str::to_string).collect();
    assert_eq!(lines.len(), (whydown::nagios::SPECS.len() - 1) * 2); // named service is excluded from all
    assert!(lines.iter().any(|l| l.contains("\tmem_used_pct\t62.5000\tOK\tdemo")));
    // Repeated samples in the same second are not a trend. Simulate a
    // distinct later timestamp to exercise an actual time-series line.
    let first_ts: u64 = lines[0].split('\t').next().unwrap().parse().unwrap();
    let later = format!("{}\tmem_used_pct\t75.0000\tOK\tlive\t80\t95\n",
        first_ts + 300);
    std::fs::OpenOptions::new().append(true).open(&history).unwrap()
        .write_all(later.as_bytes()).unwrap();

    let graphed = whydown(&["nagios","graph","--history",h,"--metric","memory",
        "--output",g]);
    assert_eq!(graphed.status.code(), Some(0), "{:?}", graphed.stderr);
    let plot = fs::read_to_string(&svg).unwrap();
    assert!(plot.starts_with("<svg "));
    assert!(plot.contains("<polyline"));
    assert!(plot.contains("75.00%")); // newest real timestamp, not the older demo reading
    let existing = whydown(&["nagios","graph","--history",h,"--metric","memory",
        "--output",g]);
    assert_eq!(existing.status.code(), Some(3));
    let replaced = whydown(&["nagios","graph","--history",h,"--metric","memory",
        "--output",g,"--overwrite"]);
    assert_eq!(replaced.status.code(), Some(0));

    let dashboard = whydown(&["nagios","dashboard","--history",h,"--output",d]);
    assert_eq!(dashboard.status.code(), Some(0));
    let page = fs::read_to_string(&html).unwrap();
    assert!(page.contains("WhyDown"));
    assert!(page.contains("DEMO DATA PRESENT"));
    assert!(page.contains("mem_used_pct"));
    assert!(page.contains("<svg "));
    let protect = whydown(&["nagios","graph","--history",h,"--metric","memory",
        "--output",h,"--overwrite"]);
    assert_eq!(protect.status.code(), Some(3));

    #[cfg(unix)] {
        use std::os::unix::fs::PermissionsExt;
        for file in [&history, &svg, &html] {
            assert_eq!(fs::metadata(file).unwrap().permissions().mode() & 0o077, 0);
        }
    }
    fs::remove_dir_all(base).unwrap();
}

#[test]
fn embedded_icon_and_ascii_mark_do_not_pollute_json_or_script_stdout() {
    let icon = whydown(&["icon"]);
    assert!(icon.status.success());
    assert!(icon.stdout.starts_with(b"\x89PNG\r\n\x1a\n"));
    assert!(icon.stdout.len() > 1_000);
    assert!(icon.stderr.is_empty());

    let brand = whydown(&["logo"]);
    assert!(brand.status.success());
    let ascii = String::from_utf8(brand.stdout).unwrap();
    assert!(ascii.contains("WhyDown"));
    assert!(ascii.is_ascii());

    // Piped JSON must still be exactly a JSON document, not a logo header.
    let json = whydown(&["demo", "--format", "json"]);
    assert!(json.status.success());
    assert_eq!(json.stdout.first(), Some(&b'{'));
}

#[test]
fn interactive_tui_rejects_pipes_without_hanging_or_reading_system_data() {
    let result = whydown(&["tui", "--demo"]);
    assert_eq!(result.status.code(), Some(2));
    assert!(result.stdout.is_empty());
    let stderr = String::from_utf8(result.stderr).unwrap();
    assert!(stderr.contains("requires stdin and stdout to be terminals"));
}

#[test]
fn interactive_tui_rejects_report_output_flags() {
    for invalid in [
        vec!["tui", "--output", "report.json"],
        vec!["tui", "--format", "json"],
        vec!["tui", "--exit-on-warning"],
    ] {
        let result = whydown(&invalid);
        assert_eq!(result.status.code(), Some(2));
        assert!(String::from_utf8(result.stderr).unwrap().contains("not supported"));
    }
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
    assert!(output.contains("\"journal_checked\": true"));
    assert!(output.contains("\"journal_truncated\": false"));
    assert!(output.contains("\"journal_entries_examined\": 4"));
    assert!(output.contains("\"interface_drop_deltas\": null"));
    assert!(output.contains("\"disk_usage_checked\": true"));
    assert!(output.contains("\"root_filesystem_read_only\": false"));
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
