use assert_fs::prelude::*;
use context_gather::tokenizer::count as count_tokens;
use predicates::prelude::*;
use predicates::str::contains;

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
        .args(["--stdout", "--no-clipboard", "-c", "50", "."])
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
    if let Some(header) = iter.next() {
        if !header.is_empty() {
            snippets.push(header.to_string());
        }
    }
    for part in iter {
        snippets.push(format!("<context-chunk id=\"{}", part));
    }
    let limit = 50;
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
