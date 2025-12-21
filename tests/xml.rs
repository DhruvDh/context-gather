use context_gather::{
    gather::FileContents, xml_output::build_xml, xml_output::build_xml_with_escape,
};
use std::path::PathBuf;

#[test]
fn groups_by_folder_and_contains_contents() {
    let files = vec![
        FileContents {
            folder: PathBuf::from("src"),
            path: PathBuf::from("src/main.rs"),
            contents: "fn main(){}".into(),
        },
        FileContents {
            folder: PathBuf::from("tests"),
            path: PathBuf::from("tests/foo.rs"),
            contents: "assert!(true);".into(),
        },
    ];
    let xml = build_xml(&files).unwrap();
    assert!(xml.contains(r#"<folder path="src">"#));
    assert!(xml.contains(r#"<folder path="tests">"#));
    assert!(xml.contains("fn main(){}"));
    assert!(xml.contains("assert!(true);"));
}

#[test]
fn escape_xml_rewrites_special_chars() {
    let files = vec![FileContents {
        folder: PathBuf::from("src"),
        path: PathBuf::from("src/main.rs"),
        contents: "if a < b && b > c { println!(\"&\"); }".into(),
    }];
    let xml = build_xml_with_escape(&files, true).unwrap();
    assert!(xml.contains("&lt;"));
    assert!(xml.contains("&gt;"));
    assert!(xml.contains("&amp;"));
    assert!(!xml.contains("if a < b && b > c"));
}
