use assert_fs::prelude::*;
use predicates::str::{contains, is_empty};

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
