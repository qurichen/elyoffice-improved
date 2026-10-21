use std::{fs, path::PathBuf, process::Command};

use ely_platform::{AppId, logging, startup};

const CHILD: &str = "ELY_STARTUP_TEST_DIR";
const NO_LOG: &str = "ELY_STARTUP_TEST_NO_LOG";

fn rerun(test: &str, dir: &std::path::Path, no_log: bool) -> std::process::Output {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", test, "--nocapture"])
        .env(CHILD, dir)
        .env("ELY_NO_DIALOG", "1");
    if no_log {
        command.env(NO_LOG, "1");
    }
    command.output().unwrap()
}

#[test]
fn a_startup_failure_is_logged_printed_and_exits_with_one() {
    const TEST: &str = "a_startup_failure_is_logged_printed_and_exits_with_one";
    if let Some(dir) = std::env::var_os(CHILD) {
        logging::init_in(AppId::Doc, &PathBuf::from(dir)).unwrap();
        startup::fail(AppId::Doc, "no usable GPU");
    }

    let dir = tempfile::tempdir().unwrap();
    let output = rerun(TEST, dir.path(), false);

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    let log_path = dir.path().join("ely.log");
    assert!(
        stderr.contains("ElyDoc could not start") && stderr.contains("no usable GPU"),
        "{stderr}"
    );
    assert!(
        stderr.contains(&log_path.display().to_string()),
        "the message does not name the real log: {stderr}"
    );
    let log = fs::read_to_string(log_path).unwrap();
    assert!(log.contains("ElyDoc cannot start: no usable GPU"), "{log}");
}

#[test]
fn without_a_logger_the_message_says_there_is_no_log() {
    const TEST: &str = "without_a_logger_the_message_says_there_is_no_log";
    if std::env::var_os(NO_LOG).is_some() {
        startup::fail(AppId::Doc, "logging did not start: read-only home");
    }

    let dir = tempfile::tempdir().unwrap();
    let output = rerun(TEST, dir.path(), true);

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("none, logging did not start"), "{stderr}");
    assert!(!stderr.contains("ely.log"), "{stderr}");
}
