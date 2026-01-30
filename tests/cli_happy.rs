use assert_fs::prelude::*;
use predicates::prelude::*;
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
        .stderr(contains("OK"));
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
        .stderr(contains("OK"));
}

#[test]
fn no_model_context_suppresses_token_summary() {
    let dir = assert_fs::TempDir::new().unwrap();
    dir.child("foo.txt").write_str("hello world").unwrap();

    assert_cmd::cargo::cargo_bin_cmd!("context-gather")
        .current_dir(&dir)
        .args([
            "--stdout",
            "--no-clipboard",
            "--no-model-context",
            "foo.txt",
        ])
        .assert()
        .success()
        .stdout(contains("<shared-context>"))
        .stderr(contains("OK"))
        .stderr(contains("tokens").not());
}

#[test]
fn default_outputs_raw_contents() {
    let dir = assert_fs::TempDir::new().unwrap();
    dir.child("foo.txt").write_str("MAGIC_<tag>&MAGIC").unwrap();

    assert_cmd::cargo::cargo_bin_cmd!("context-gather")
        .current_dir(&dir)
        .args(["--stdout", "--no-clipboard", "foo.txt"])
        .assert()
        .success()
        .stdout(contains("MAGIC_<tag>&MAGIC"));
}

#[test]
fn escape_xml_escapes_file_contents() {
    let dir = assert_fs::TempDir::new().unwrap();
    dir.child("foo.txt").write_str("MAGIC_<tag>&MAGIC").unwrap();

    assert_cmd::cargo::cargo_bin_cmd!("context-gather")
        .current_dir(&dir)
        .args(["--stdout", "--no-clipboard", "--escape-xml", "foo.txt"])
        .assert()
        .success()
        .stdout(contains("MAGIC_&lt;tag&gt;&amp;MAGIC"));
}
