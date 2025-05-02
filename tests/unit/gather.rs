use context_gather::gather::*;
use assert_fs::prelude::*;
use predicates::prelude::*;
use tests::common::basic_fs;

#[test]
fn expand_paths_glob_and_literal() {
    let td = basic_fs();
    let src = td.child("src");

    let glob_pat = format!("{}/src/*.rs", td.path().display());
    let mut paths = expand_paths(vec![glob_pat.clone(), "README.md".into()]).unwrap();

    paths.sort();
    assert_eq!(paths.len(), 2, "one .rs + the literal README");
    assert!(paths.iter().any(|p| p.ends_with("hello.rs")));
    assert!(paths.iter().any(|p| p.ends_with("README.md")));
}

#[test]
fn gather_all_file_paths_respects_gitignore() {
    let td = basic_fs();
    let paths = gather_all_file_paths(&[td.path().into()]).unwrap();
    let all = paths.iter().map(|p| p.strip_prefix(td.path()).unwrap()).collect::<Vec<_>>();

    assert!(all.contains(&std::path::Path::new("bin/binary.dat")));
    assert!(!all.contains(&std::path::Path::new("deep/ignore.me")));
}

#[test]
fn read_file_skips_binary_and_too_large() {
    use std::io::Write;
    let dir = assert_fs::TempDir::new().unwrap();

    let mut bin = dir.child("bin.dat").create_file().unwrap();
    bin.write_all(&[0u8, 255u8, 0u8, 128u8]).unwrap();

    let err = super::read_file(bin.path(), 1024).unwrap_err();
    assert!(format!("{err}").contains("binary"), "{err}");

    let huge = dir.child("huge.txt");
    huge.write_str(&"x".repeat(2048)).unwrap();
    let err2 = super::read_file(huge.path(), 1000).unwrap_err();
    assert!(format!("{err2}").contains("exceeds 1000"), "{err2}");
}

#[test]
fn token_count_is_stable() {
    let n = count_tokens("hello world");
    assert_eq!(n, 2);
}
