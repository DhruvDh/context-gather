use assert_fs::TempDir;
use std::fs;

/// Builds a fixture tree:
/// root/
///   src/hello.rs
///   README.md
///   bin/binary.dat  (non-UTF8)
///   deep/ignore.me     (# ignored via .gitignore)
pub fn basic_fs() -> TempDir {
    let td = TempDir::new().unwrap();
    let root = td.path();

    // src/hello.rs
    let src_dir = root.join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(
        src_dir.join("hello.rs"),
        "fn main() { println!(\"hello\"); }\n",
    )
    .unwrap();

    // README.md
    fs::write(root.join("README.md"), "# readme\n").unwrap();

    // binary file
    let bin_dir = root.join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    fs::write(bin_dir.join("binary.dat"), vec![0u8; 4096]).unwrap();

    // deep ignored file
    let deep_dir = root.join("deep");
    fs::create_dir_all(&deep_dir).unwrap();
    fs::write(deep_dir.join("ignore.me"), "should be ignored\n").unwrap();

    // .gitignore
    fs::write(root.join(".gitignore"), "/deep\n").unwrap();
    td
}
