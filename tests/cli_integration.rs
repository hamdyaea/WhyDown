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
    assert!(output.contains("\"boot_relation\""));
    assert!(output.contains("\"timeline\""));
    assert!(output.contains("\"historical_journal\""));
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
