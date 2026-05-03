use assert_fs::prelude::*;
use predicates::str::{contains, is_empty};
use std::time::Duration;

#[test]
fn multi_step_without_stdout_is_quiet() {
    let dir = assert_fs::TempDir::new().unwrap();
    dir.child("a.txt").write_str("hello").unwrap();

    // Run with multi-step only, without --stdout: should not print header snippet
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("context-gather");
    cmd.current_dir(&dir).args(["-m", "--no-clipboard", "."]); // multi-step mode
    // Send 'q' to exit REPL immediately
    cmd.write_stdin("q\n")
        .assert()
        .success()
        .stdout(is_empty())
        .stderr(contains("Request file id or glob"));
}

#[test]
fn multi_step_with_stdout_prints_header() {
    let dir = assert_fs::TempDir::new().unwrap();
    dir.child("b.txt").write_str("world").unwrap();

    // Run with --stdout and multi-step
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("context-gather");
    cmd.current_dir(&dir).args(["--stdout", "-m", "."]);
    cmd.write_stdin("q\n")
        .assert()
        .success()
        .stdout(contains("<shared-context>"))
        .stderr(contains("Request file id or glob"));
}

#[test]
fn multi_step_exits_on_stdin_eof() {
    let dir = assert_fs::TempDir::new().unwrap();
    dir.child("c.txt").write_str("done").unwrap();

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("context-gather");
    cmd.current_dir(&dir)
        .args(["--stdout", "--no-clipboard", "-m", "."])
        .write_stdin("")
        .timeout(Duration::from_secs(2));
    cmd.assert()
        .success()
        .stdout(contains("<shared-context>"))
        .stderr(contains("stdin closed; leaving multi-step mode."));
}
