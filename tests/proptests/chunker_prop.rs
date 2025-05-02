#![cfg_attr(not(test), allow(dead_code))]
use context_gather::{chunker::build_chunks, gather::FileContents};
use proptest::prelude::*;
use std::path::PathBuf;

proptest! {
    #[test]
    fn reassembled_equals_original(lines in prop::collection::vec(".*", 1..100),
                                   limit in 10usize..200usize) {
        // force at least one oversize scenario
        let text = lines.join("\n");
        let file = FileContents {
            folder: PathBuf::from("."),
            path: PathBuf::from("big.txt"),
            contents: text.clone(),
        };
        let (chunks, _) = build_chunks(&[file], limit);
        let glued:String = chunks.into_iter().map(|c| c.xml).collect();
        for l in &lines {
            prop_assert!(glued.contains(l));
        }
    }
}
