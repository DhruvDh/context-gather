use assert_fs::prelude::*;
use predicates::str::contains;

#[test]
fn dir_and_file_are_deduped() {
    let dir = assert_fs::TempDir::new().unwrap();
    dir.child("foo.txt").write_str("hello").unwrap();

    assert_cmd::cargo::cargo_bin_cmd!("context-gather")
        .current_dir(&dir)
        .args(["--stdout", "--no-clipboard", ".", "foo.txt"])
        .assert()
        .success()
        .stdout(contains("<shared-context"))
        .stderr(contains("OK 1 files"));
}
