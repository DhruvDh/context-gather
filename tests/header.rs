use context_gather::{chunker::FileMeta, header::make_header};
use std::path::PathBuf;

#[test]
fn header_reports_totals_correctly() {
    let metas = vec![
        FileMeta {
            id: 0,
            path: PathBuf::from("a.rs"),
            tokens: 10,
            parts: 1,
        },
        FileMeta {
            id: 1,
            path: PathBuf::from("b.rs"),
            tokens: 20,
            parts: 2,
        },
    ];
    let hdr = make_header(5, 40000, &metas);
    assert!(hdr.contains(r#"total-chunks="5""#));
    assert!(hdr.contains(r#"total-files="2""#));
    assert!(hdr.contains(r#"id="1" path="b.rs" tokens="20" parts="2""#));
}

// Test that git-info section is included with at least one commit
#[test]
fn test_git_info_included() {
    let metas = vec![FileMeta {
        id: 0,
        path: PathBuf::from("a.rs"),
        tokens: 10,
        parts: 1,
    }];
    let hdr = make_header(1, 100, &metas);
    // Should contain git-info opening and closing tags
    assert!(
        hdr.contains("<git-info branch=\""),
        "git-info branch tag missing"
    );
    assert!(hdr.contains("</git-info>"), "git-info closing tag missing");
    // Should have at least one commit entry
    let commit_count = hdr.matches("<commit>").count();
    assert!(
        commit_count >= 1,
        "Expected at least one <commit> entry, got {}",
        commit_count
    );
}
