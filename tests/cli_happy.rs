use assert_fs::prelude::*;
use predicates::str::contains;

#[test]
fn stdout_only_basic() {
    let dir = assert_fs::TempDir::new().unwrap();
    dir.child("foo.txt").write_str("hello world").unwrap();

    assert_cmd::cargo::cargo_bin_cmd!("context-gather")
        .current_dir(&dir)
        .args(["--stdout", "--no-clipboard", "foo.txt"])
        .assert()
        .success()
        .stdout(contains("<shared-context>"))
        .stderr(predicates::str::is_empty());
}

#[test]
fn chunk_size_splits_and_summarises() {
    let dir = assert_fs::TempDir::new().unwrap();
    for i in 0..10 {
        dir.child(format!("f{i}.txt"))
            .write_str(&"tok ".repeat(100))
            .unwrap();
    }

    assert_cmd::cargo::cargo_bin_cmd!("context-gather")
        .current_dir(&dir)
        .args(["--stdout", "--no-clipboard", "-c", "50", "."])
        .assert()
        .success()
        .stdout(contains("<context-chunk id="))
        .stdout(contains("<more remaining=\""))
        .stdout(contains("OK")) // summary line
        .stderr(predicates::str::is_empty());
}
