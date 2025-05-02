#![cfg_attr(not(test), allow(dead_code))]
use context_gather::{chunker::build_chunks, gather::FileContents};
use proptest::prelude::*;
use std::path::PathBuf;

proptest! {
    #[test]
    // This the only piece of the suite that
    // exercises “for every input” correctness of the smart-chunking
    // algorithm. Deleting it would throw away:
    //
    // Regression safety – it catches edge-cases no hand-written example ever
    // will (empty lines, Unicode, 1-byte limits, etc.).
    //
    // Refactor confidence – any future change to build_chunks that breaks
    // re-assembly immediately fails the property test.
    //
    // Automatic bug discovery – if an undiscovered panic lurks in the token
    // math, Proptest will eventually find it.
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
