#![cfg(feature = "integration")]

use std::process::Command;
use std::sync::Mutex;

static RUN_LOCK: Mutex<()> = Mutex::new(());

fn hello_elf() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/test/generated_elfs/hello.elf")
}

fn hello_puts_elf() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/test/generated_elfs/hello_puts.elf")
}

fn run_ps5rs(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_ps5rs"))
        .args(args)
        .output()
        .expect("ps5rs binary should run")
}

#[test]
fn run_hello_reports_exit_zero() {
    let _guard = RUN_LOCK.lock().unwrap();
    let output = run_ps5rs(&["run", hello_elf().to_str().unwrap()]);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("exited with code 0"), "{stdout}");
    assert!(stdout.contains("(no import calls)"), "{stdout}");
}

#[test]
fn run_hello_json_serializes_report() {
    let _guard = RUN_LOCK.lock().unwrap();
    let output = run_ps5rs(&["run", "--json", hello_elf().to_str().unwrap()]);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("JSON report");
    assert_eq!(report["version"], 1);
    assert_eq!(report["exit_code"], 0);
    assert_eq!(report["module_name"], "hello.elf");
    assert!(report["import_calls"].as_array().unwrap().is_empty());
}

#[test]
fn run_hello_puts_prints_message() {
    let _guard = RUN_LOCK.lock().unwrap();
    let output = run_ps5rs(&["run", hello_puts_elf().to_str().unwrap()]);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Hello from ps5rs!"), "{stdout}");
    assert!(stdout.contains("libkernel::puts"), "{stdout}");
    assert!(stdout.contains("exited with code 0"), "{stdout}");
    assert!(!stdout.contains("(no import calls)"), "{stdout}");
}

#[test]
fn run_hello_puts_json_reports_import_call() {
    let _guard = RUN_LOCK.lock().unwrap();
    let output = run_ps5rs(&["run", "--json", hello_puts_elf().to_str().unwrap()]);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Hello from ps5rs!"), "{stdout}");
    let json_start = stdout.find('{').expect("JSON object in stdout");
    let report: serde_json::Value =
        serde_json::from_str(&stdout[json_start..]).expect("JSON report");
    assert_eq!(report["exit_code"], 0);
    assert_eq!(report["module_name"], "hello_puts.elf");
    let calls = report["import_calls"].as_array().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0]["library"], "libkernel");
    assert_eq!(calls[0]["name"], "puts");
    assert_eq!(calls[0]["return_value"], 0);
}
