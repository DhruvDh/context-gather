use assert_fs::prelude::*;

#[test]
fn chunk_index_requires_chunk_size() {
    let dir = assert_fs::TempDir::new().unwrap();
    dir.child("foo.txt").write_str("hello world").unwrap();

    assert_cmd::cargo::cargo_bin_cmd!("context-gather")
        .current_dir(&dir)
        .args([
            "--chunk-index",
            "0",
            "--stdout",
            "--no-clipboard",
            "foo.txt",
        ])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn chunk_size_zero_is_invalid() {
    let dir = assert_fs::TempDir::new().unwrap();
    dir.child("foo.txt").write_str("hello world").unwrap();

    assert_cmd::cargo::cargo_bin_cmd!("context-gather")
        .current_dir(&dir)
        .args(["--chunk-size", "0", "--stdout", "--no-clipboard", "foo.txt"])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn invalid_exclude_pattern_errors() {
    let dir = assert_fs::TempDir::new().unwrap();
    dir.child("foo.txt").write_str("hello world").unwrap();

    assert_cmd::cargo::cargo_bin_cmd!("context-gather")
        .current_dir(&dir)
        .args([
            "--exclude-paths",
            "[",
            "--stdout",
            "--no-clipboard",
            "foo.txt",
        ])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn chunk_index_out_of_range_errors() {
    let dir = assert_fs::TempDir::new().unwrap();
    dir.child("foo.txt").write_str("hello world").unwrap();

    assert_cmd::cargo::cargo_bin_cmd!("context-gather")
        .current_dir(&dir)
        .args([
            "--chunk-size",
            "10",
            "--chunk-index",
            "99",
            "--stdout",
            "--no-clipboard",
            "foo.txt",
        ])
        .assert()
        .failure()
        .code(3);
}
