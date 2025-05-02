mod common;
use assert_fs::prelude::*;
use common::basic_fs;
use context_gather::gather::*;
use std::fs;

#[test]
fn expand_paths_glob_and_literal() {
    let td = basic_fs();
    let _src = td.child("src");

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
    let all = paths
        .iter()
        .map(|p| p.strip_prefix(td.path()).unwrap())
        .collect::<Vec<_>>();

    assert!(all.contains(&std::path::Path::new("bin/binary.dat")));
    assert!(!all.contains(&std::path::Path::new("deep/ignore.me")));
}

#[test]
fn read_file_skips_binary_and_too_large() {
    let dir = assert_fs::TempDir::new().unwrap();
    // Write a binary file
    let bin_child = dir.child("bin.dat");
    let bin_path = bin_child.path().to_path_buf();
    fs::write(&bin_path, [0u8, 255u8, 0u8, 128u8]).unwrap();
    // Read and detect binary
    let err = read_file(&bin_path, 1024).unwrap_err();

    let huge_child = dir.child("huge.txt");
    let huge_path = huge_child.path().to_path_buf();
    fs::write(&huge_path, "x".repeat(2048).into_bytes()).unwrap();
    let err2 = read_file(&huge_path, 1000).unwrap_err();

    assert!(format!("{err}").contains("binary"), "{err}");
    assert!(format!("{err2}").contains("exceeds 1000"), "{err2}");
}

#[test]
fn token_count_is_stable() {
    let n = count_tokens("hello world");
    assert_eq!(n, 2);
}
