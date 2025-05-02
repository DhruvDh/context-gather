use context_gather::{header::make_header, chunker::FileMeta};
use std::path::PathBuf;

#[test]
fn header_reports_totals_correctly() {
    let metas = vec![
        FileMeta { id:0, path:PathBuf::from("a.rs"), tokens:10, parts:1 },
        FileMeta { id:1, path:PathBuf::from("b.rs"), tokens:20, parts:2 },
    ];
    let hdr = make_header(5, 40000, &metas);
    assert!(hdr.contains(r#"total-chunks="5""#));
    assert!(hdr.contains(r#"total-files="2""#));
    assert!(hdr.contains(r#"id="1" path="b.rs" tokens="20" parts="2""#));
}
