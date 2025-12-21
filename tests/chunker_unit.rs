use context_gather::{chunker::*, gather::FileContents, tokenizer::count as count_tokens};
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
    let (chunks, meta) = build_chunks(&files, 0, false);
    assert_eq!(chunks.len(), 1);
    assert_eq!(meta.len(), 2);
    assert!(chunks[0].tokens >= 15);
}

#[test]
fn split_across_two_chunks() {
    let path = PathBuf::from("file0.txt");
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let contents = "x\n".repeat(200);
    let file_block = format!(
        "    <file-contents path=\"{}\" name=\"{}\">\n{}\n    </file-contents>\n",
        path.display(),
        name,
        contents
    );
    let file_tok = count_tokens(&file_block);
    let limit = (file_tok / 4).max(1);
    let file = FileContents {
        folder: PathBuf::from("."),
        path,
        contents,
    };
    let (chunks, meta) = build_chunks(&[file], limit, false);
    assert!(chunks.len() >= 2);
    assert!(meta[0].parts >= 2);
}

#[test]
fn oversize_file_line_split_keeps_order() {
    let content = (1..=30).map(|n| format!("line{n}\n")).collect::<String>();
    let f = FileContents {
        folder: PathBuf::from("."),
        path: PathBuf::from("big.txt"),
        contents: content.clone(),
    };
    let (chunks, _) = build_chunks(&[f], 50, false); // tiny token limit
    // Re-assemble lines from all chunks and compare
    let joined: String = chunks.iter().map(|c| c.xml.clone()).collect();
    for n in 1..=30 {
        assert!(joined.contains(&format!("line{n}")), "missing line{n}");
    }
}

#[test]
fn part_counts_match_output() {
    let content = "line\n".repeat(200);
    let f = FileContents {
        folder: PathBuf::from("."),
        path: PathBuf::from("big.txt"),
        contents: content,
    };
    let (chunks, meta) = build_chunks(&[f], 50, false);
    let joined: String = chunks.iter().map(|c| c.xml.clone()).collect();
    let mut parts: Vec<(usize, usize)> = Vec::new();
    for segment in joined.split("part=\"").skip(1) {
        if let Some(raw) = segment.split('"').next() {
            let mut iter = raw.split('/');
            if let (Some(idx), Some(total)) = (iter.next(), iter.next())
                && let (Ok(idx), Ok(total)) = (idx.parse::<usize>(), total.parse::<usize>())
            {
                parts.push((idx, total));
            }
        }
    }
    assert!(!parts.is_empty(), "expected split parts to be present");
    let total = parts[0].1;
    assert!(parts.iter().all(|(_, t)| *t == total));
    let max_idx = parts.iter().map(|(i, _)| *i).max().unwrap();
    assert_eq!(max_idx, total);
    assert_eq!(meta[0].parts, total);
}
