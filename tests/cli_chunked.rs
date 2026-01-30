use assert_fs::prelude::*;
use context_gather::tokenizer::count as count_tokens;
use predicates::prelude::*;
use predicates::str::{contains, is_empty};

#[test]
fn chunk_size_splits_and_summarizes() {
    let dir = assert_fs::TempDir::new().unwrap();
    for i in 0..10 {
        dir.child(format!("f{i}.txt"))
            .write_str(&"tok\n".repeat(100))
            .unwrap();
    }

    let output = assert_cmd::cargo::cargo_bin_cmd!("context-gather")
        .current_dir(&dir)
        .args(["--stdout", "--no-clipboard", "-c", "80", "."])
        .assert()
        .success()
        .stdout(contains("<context-chunk id=\""))
        .stdout(contains("<more remaining=\"")) // the marker printed between chunks
        .stderr(contains("OK"))
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8_lossy(&output);
    let mut snippets = Vec::new();
    let mut iter = stdout.split("<context-chunk id=\"");
    if let Some(header) = iter.next()
        && !header.is_empty()
    {
        snippets.push(header.to_string());
    }
    for part in iter {
        snippets.push(format!("<context-chunk id=\"{}", part));
    }
    let limit = 80;
    for snippet in snippets
        .into_iter()
        .filter(|s| s.starts_with("<context-chunk"))
    {
        let tokens = count_tokens(&snippet);
        assert!(
            tokens <= limit,
            "chunk snippet exceeded limit: {tokens} > {limit}"
        );
    }
}

#[test]
fn zero_files_outputs_header_with_closing_tag() {
    let dir = assert_fs::TempDir::new().unwrap();

    assert_cmd::cargo::cargo_bin_cmd!("context-gather")
        .current_dir(&dir)
        .args(["--stdout", "--no-clipboard", "-c", "50", "."])
        .assert()
        .success()
        .stdout(contains("</shared-context>"))
        .stdout(contains("<context-chunk id=").not())
        .stdout(predicates::str::is_empty().not())
        .stderr(contains("OK"));
}

#[test]
fn chunk_index_selects_single_stdout_chunk() {
    let dir = assert_fs::TempDir::new().unwrap();
    for i in 0..10 {
        dir.child(format!("f{i}.txt"))
            .write_str(&"tok\n".repeat(100))
            .unwrap();
    }

    let output = assert_cmd::cargo::cargo_bin_cmd!("context-gather")
        .current_dir(&dir)
        .args(["--stdout", "--no-clipboard", "-c", "50", "-k", "2", "."])
        .assert()
        .success()
        .stderr(contains("OK"))
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8_lossy(&output);
    let count = stdout.matches("<context-chunk id=").count();
    assert_eq!(count, 1, "expected exactly one chunk in stdout");
    assert!(
        stdout.contains("<context-chunk id=\"2/"),
        "expected chunk 2 in stdout"
    );
    assert!(
        !stdout.contains("<shared-context>"),
        "expected no header when printing a specific chunk"
    );
}

#[test]
fn chunk_index_none_suppresses_stdout() {
    let dir = assert_fs::TempDir::new().unwrap();
    for i in 0..5 {
        dir.child(format!("f{i}.txt"))
            .write_str(&"tok\n".repeat(100))
            .unwrap();
    }

    assert_cmd::cargo::cargo_bin_cmd!("context-gather")
        .current_dir(&dir)
        .args(["--stdout", "--no-clipboard", "-c", "50", "-k", "-1", "."])
        .assert()
        .success()
        .stdout(is_empty())
        .stderr(contains("OK"));
}
