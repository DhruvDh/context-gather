use assert_fs::TempDir;
use context_gather::{chunker::*, gather::FileContents};
use std::path::PathBuf;

fn make_file(
    id: usize,
    repeat: usize,
) -> FileContents {
    FileContents {
        folder: PathBuf::from("."),
        path: PathBuf::from(format!("file{id}.txt")),
        contents: "tok ".repeat(repeat), // 1 token ~= "tok"
    }
}

#[test]
fn no_limit_yields_single_chunk() {
    let files = vec![make_file(0, 10), make_file(1, 5)];
    let (chunks, meta) = build_chunks(&files, 0);
    assert_eq!(chunks.len(), 1);
    assert_eq!(meta.len(), 2);
    assert!(chunks[0].tokens >= 15);
}

#[test]
fn split_across_two_chunks() {
    // 15 tokens, limit 10 – should give two chunks 10/5
    let files = vec![make_file(0, 15)];
    let (chunks, meta) = build_chunks(&files, 10);
    assert_eq!(chunks.len(), 2);
    assert_eq!(meta[0].parts, 2); // file got split
    assert!(chunks[0].tokens <= 10);
}

#[test]
fn oversize_file_line_split_keeps_order() {
    let content = (1..=30).map(|n| format!("line{n}\n")).collect::<String>();
    let f = FileContents {
        folder: PathBuf::from("."),
        path: PathBuf::from("big.txt"),
        contents: content.clone(),
    };
    let (chunks, _) = build_chunks(&[f], 50); // tiny token limit
    // Re-assemble lines from all chunks and compare
    let joined: String = chunks.iter().map(|c| c.xml.clone()).collect();
    for n in 1..=30 {
        assert!(joined.contains(&format!("line{n}")), "missing line{n}");
    }
}
