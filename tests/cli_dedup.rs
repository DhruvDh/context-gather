use assert_cmd::Command;
use assert_fs::prelude::*;
use predicates::str::{contains, is_empty};

#[test]
fn dir_and_file_are_deduped() {
    let dir = assert_fs::TempDir::new().unwrap();
    dir.child("foo.txt").write_str("hello").unwrap();

    Command::cargo_bin("context-gather")
        .unwrap()
        .current_dir(&dir)
        .args(["--stdout", "--no-clipboard", ".", "foo.txt"])
        .assert()
        .success()
        .stdout(contains("✔ 1 files"))
        .stderr(is_empty());
}
