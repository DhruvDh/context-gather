use assert_fs::{fixture::PathChild, TempDir};
use std::fs;

/// Builds a fixture tree:
/// root/
///   src/hello.rs
///   README.md
///   bin/binary.dat  (non-UTF8)
///   deep/ignore.me     (# ignored via .gitignore)
pub fn basic_fs() -> TempDir {
    let td = TempDir::new().unwrap();
    let src = td.child("src");
    src.create_dir_all().unwrap();
    src.child("hello.rs").write_str("fn main() { println!(\"hello\"); }\n").unwrap();

    td.child("README.md").write_str("# readme\n").unwrap();

    // binary file (lots of zeroes)
    let mut bin = fs::File::create(td.child("bin").child("binary.dat")).unwrap();
    bin.write_all(&[0u8; 4096]).unwrap();

    // deep ignored file
    let deep = td.child("deep");
    deep.create_dir_all().unwrap();
    deep.child("ignore.me").write_str("should be ignored\n").unwrap();

    // .gitignore
    td.child(".gitignore").write_str("/deep\n").unwrap();
    td
}
