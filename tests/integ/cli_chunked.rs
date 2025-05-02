use assert_cmd::Command;
use assert_fs::prelude::*;
use predicates::str::contains;

#[test]
fn chunk_size_splits_and_summarizes() {
    let dir = assert_fs::TempDir::new().unwrap();
    for i in 0..10 {
        dir.child(format!("f{i}.txt")).write_str("tok ".repeat(100)).unwrap();
    }

    Command::cargo_bin("context-gather").unwrap()
        .current_dir(&dir)
        .args(["--stdout", "--no-clipboard", "-c", "50", "."])
        .assert()
        .success()
        .stdout(contains("<context-part"))
        .stdout(contains("<more/>"))        // the marker printed between chunks
        .stdout(contains("✔"))             // summary line
        .stderr(predicates::str::is_empty());
}
