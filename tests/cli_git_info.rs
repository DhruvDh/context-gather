use assert_fs::prelude::*;
use predicates::prelude::*;
use predicates::str::contains;
use std::process::Command;

fn git(
    dir: &assert_fs::TempDir,
    args: &[&str],
) {
    let output = Command::new("git")
        .current_dir(dir.path())
        .args(args)
        .output()
        .unwrap_or_else(|err| panic!("failed to run git {args:?}: {err}"));
    assert!(
        output.status.success(),
        "git {args:?} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn git_info_uses_local_main_when_origin_main_is_absent() {
    let dir = assert_fs::TempDir::new().unwrap();
    git(&dir, &["init"]);
    git(&dir, &["checkout", "-B", "main"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "Context Gather Test"]);
    dir.child("a.txt").write_str("before\n").unwrap();
    git(&dir, &["add", "a.txt"]);
    git(&dir, &["commit", "-m", "Initial commit"]);
    git(&dir, &["checkout", "-b", "feature"]);
    dir.child("a.txt").write_str("after\n").unwrap();

    assert_cmd::cargo::cargo_bin_cmd!("context-gather")
        .current_dir(&dir)
        .env_remove("CG_TOKENIZER_MODEL")
        .args([
            "--stdout",
            "--no-clipboard",
            "--chunk-size",
            "10000",
            "--git-info",
            ".",
        ])
        .assert()
        .success()
        .stdout(contains(r#"<changed-files diffed-against="main">"#))
        .stdout(contains("<file>a.txt</file>"))
        .stdout(contains("origin/main").not());
}
