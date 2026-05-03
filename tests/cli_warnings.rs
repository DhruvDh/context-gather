use assert_fs::prelude::*;
use predicates::prelude::*;
use predicates::str::contains;
use std::fs;

#[test]
fn missing_literal_warns_and_reports_skipped() {
    let dir = assert_fs::TempDir::new().unwrap();

    assert_cmd::cargo::cargo_bin_cmd!("context-gather")
        .current_dir(&dir)
        .args(["--stdout", "--no-clipboard", "missing.txt"])
        .assert()
        .success()
        .stdout(contains(r#"total-files="0""#))
        .stderr(contains("No such file or directory"))
        .stderr(contains("no files were included in output"))
        .stderr(contains("skipped=1"));
}

#[test]
fn oversize_only_warns_and_reports_skipped() {
    let dir = assert_fs::TempDir::new().unwrap();
    dir.child("big.txt").write_str("too big").unwrap();

    assert_cmd::cargo::cargo_bin_cmd!("context-gather")
        .current_dir(&dir)
        .args(["--stdout", "--no-clipboard", "--max-size", "1", "big.txt"])
        .assert()
        .success()
        .stdout(contains(r#"total-files="0""#))
        .stderr(contains("exceeds 1 bytes"))
        .stderr(contains("skipped=1"));
}

#[test]
fn binary_only_warns_and_reports_skipped() {
    let dir = assert_fs::TempDir::new().unwrap();
    fs::write(dir.path().join("bin.dat"), [0_u8, 255, 0, 128]).unwrap();

    assert_cmd::cargo::cargo_bin_cmd!("context-gather")
        .current_dir(&dir)
        .args(["--stdout", "--no-clipboard", "bin.dat"])
        .assert()
        .success()
        .stdout(contains(r#"total-files="0""#))
        .stderr(contains("appears to be a binary file"))
        .stderr(contains("skipped=1"));
}

#[test]
fn exclude_all_warns_without_skipped_count() {
    let dir = assert_fs::TempDir::new().unwrap();
    dir.child("a.rs").write_str("fn main() {}\n").unwrap();

    assert_cmd::cargo::cargo_bin_cmd!("context-gather")
        .current_dir(&dir)
        .args([
            "--stdout",
            "--no-clipboard",
            "--exclude-paths",
            "**/*.rs",
            ".",
        ])
        .assert()
        .success()
        .stdout(contains(r#"total-files="0""#))
        .stderr(contains("no files were included in output"))
        .stderr(contains("skipped=").not());
}

#[test]
fn mixed_success_and_skip_reports_skipped_count() {
    let dir = assert_fs::TempDir::new().unwrap();
    dir.child("small.txt").write_str("ok").unwrap();
    dir.child("big.txt").write_str("too big").unwrap();

    assert_cmd::cargo::cargo_bin_cmd!("context-gather")
        .current_dir(&dir)
        .args([
            "--stdout",
            "--no-clipboard",
            "--max-size",
            "3",
            "small.txt",
            "big.txt",
        ])
        .assert()
        .success()
        .stdout(contains(r#"total-files="1""#))
        .stdout(contains("ok"))
        .stderr(contains("exceeds 3 bytes"))
        .stderr(contains("OK 1 files"))
        .stderr(contains("skipped=1"));
}
