use assert_fs::prelude::*;
use predicates::str::contains;

#[test]
fn exclude_relative_paths_matches_cwd() {
    let dir = assert_fs::TempDir::new().unwrap();
    dir.child("src").create_dir_all().unwrap();
    dir.child("src/a.rs").write_str("fn a() {}\n").unwrap();
    dir.child("b.rs").write_str("fn b() {}\n").unwrap();

    let output = assert_cmd::cargo::cargo_bin_cmd!("context-gather")
        .current_dir(&dir)
        .args([
            "--exclude-paths",
            "src/**",
            "--stdout",
            "--no-clipboard",
            ".",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(stdout.contains("b.rs"));
    assert!(!stdout.contains("a.rs"));
}

#[test]
fn invalid_exclude_patterns_warn_but_valid_apply() {
    let dir = assert_fs::TempDir::new().unwrap();
    dir.child("src").create_dir_all().unwrap();
    dir.child("src/a.rs").write_str("fn a() {}\n").unwrap();
    dir.child("b.rs").write_str("fn b() {}\n").unwrap();

    let output = assert_cmd::cargo::cargo_bin_cmd!("context-gather")
        .current_dir(&dir)
        .args([
            "--exclude-paths",
            "[",
            "--exclude-paths",
            "src/**",
            "--stdout",
            "--no-clipboard",
            ".",
        ])
        .assert()
        .success()
        .stderr(contains("invalid --exclude-paths pattern: ["))
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(stdout.contains("b.rs"));
    assert!(!stdout.contains("a.rs"));
}
