use assert_cmd::Command;
use assert_fs::prelude::*;
use predicates::str::contains;

#[test]
fn chunk_size_splits_and_summarizes() {
    let dir = assert_fs::TempDir::new().unwrap();
    for i in 0..10 {
        dir.child(format!("f{i}.txt"))
            .write_str(&"tok ".repeat(100))
            .unwrap();
    }

    Command::cargo_bin("context-gather")
        .unwrap()
        .current_dir(&dir)
        .args(["--stdout", "--no-clipboard", "-c", "50", "."])
        .assert()
        .success()
        .stdout(contains("<context-chunk id=\""))
        .stdout(contains("<more remaining=\"")) // the marker printed between chunks
        .stdout(contains("OK")) // summary line
        .stderr(predicates::str::is_empty());
}

#[test]
fn zero_files_outputs_header_with_closing_tag() {
    let dir = assert_fs::TempDir::new().unwrap();

    Command::cargo_bin("context-gather")
        .unwrap()
        .current_dir(&dir)
        .args(["--stdout", "--no-clipboard", "-c", "50", "."])
        .assert()
        .success()
        .stdout(contains("</shared-context>"))
        .stdout(contains("<context-chunk id=").not())
        .stdout(predicates::str::is_empty().not())
        .stderr(predicates::str::is_empty());
}
