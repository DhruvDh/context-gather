use assert_cmd::Command;
use assert_fs::prelude::*;
use predicates::str::{contains, starts_with};

#[test]
fn stdout_only_basic() {
    let dir = assert_fs::TempDir::new().unwrap();
    dir.child("foo.txt").write_str("hello world").unwrap();

    Command::cargo_bin("context-gather").unwrap()
        .current_dir(&dir)
        .args(["--stdout", "--no-clipboard", "foo.txt"])
        .assert()
        .success()
        .stdout(contains("<context-gather"))
        .stderr(predicates::str::is_empty());
}

#[test]
fn chunk_size_splits_and_summarises() {
    let dir = assert_fs::TempDir::new().unwrap();
    for i in 0..10 {
        dir.child(format!("f{i}.txt")).write_str("tok ".repeat(100)).unwrap();
    }

    Command::cargo_bin("context-gather").unwrap()
        .current_dir(&dir)
        .args(["--stdout", "--no-clipboard", "-c", "50", "."])
        .assert()
        .success()
        .stdout(contains("<context-header"))
        .stdout(contains("<more/>"))        // the marker printed between chunks
        .stdout(contains("✔"))             // summary line
        .stderr(predicates::str::is_empty());
}
