use assert_fs::prelude::*;
use predicates::str::contains;

#[test]
fn unsupported_tokenizer_model_errors() {
    let dir = assert_fs::TempDir::new().unwrap();
    dir.child("foo.txt").write_str("hello world").unwrap();

    assert_cmd::cargo::cargo_bin_cmd!("context-gather")
        .current_dir(&dir)
        .env_remove("CG_TOKENIZER_MODEL")
        .args([
            "--stdout",
            "--no-clipboard",
            "--tokenizer-model",
            "definitely-not-real",
            "foo.txt",
        ])
        .assert()
        .failure()
        .stderr(contains("unsupported tokenizer model"));
}

#[test]
fn documented_gpt5_tokenizer_model_succeeds() {
    let dir = assert_fs::TempDir::new().unwrap();
    dir.child("foo.txt").write_str("hello world").unwrap();

    assert_cmd::cargo::cargo_bin_cmd!("context-gather")
        .current_dir(&dir)
        .env_remove("CG_TOKENIZER_MODEL")
        .args([
            "--stdout",
            "--no-clipboard",
            "--tokenizer-model",
            "gpt-5.2",
            "foo.txt",
        ])
        .assert()
        .success()
        .stderr(contains("OK"));
}

#[test]
fn unsupported_env_tokenizer_model_errors() {
    let dir = assert_fs::TempDir::new().unwrap();
    dir.child("foo.txt").write_str("hello world").unwrap();

    assert_cmd::cargo::cargo_bin_cmd!("context-gather")
        .current_dir(&dir)
        .env("CG_TOKENIZER_MODEL", "definitely-not-real")
        .args(["--stdout", "--no-clipboard", "foo.txt"])
        .assert()
        .failure()
        .stderr(contains("unsupported tokenizer model"));
}
