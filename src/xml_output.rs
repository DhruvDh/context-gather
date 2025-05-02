use anyhow::Result;
use path_slash::PathBufExt;
use quick_xml::{
    Writer,
    events::{BytesEnd, BytesStart, BytesText, Event},
};

use super::gather::FileContents;

/// Builds an XML-like structure grouping files by folder using a streaming
/// writer.
pub fn build_xml(files: &[FileContents]) -> Result<String> {
    // Initialize writer with indentation
    let mut writer = Writer::new_with_indent(Vec::new(), b' ', 2);
    // Root element
    writer.write_event(Event::Start(BytesStart::new("context-gather")))?;

    let mut current_folder: Option<String> = None;
    for file in files {
        let folder = file.folder.to_slash_lossy().to_string();
        // Open new folder tag if needed
        if current_folder.as_ref() != Some(&folder) {
            if current_folder.is_some() {
                writer.write_event(Event::End(BytesEnd::new("folder")))?;
            }
            current_folder = Some(folder.clone());
            let mut fld = BytesStart::new("folder");
            fld.push_attribute(("path", folder.as_str()));
            writer.write_event(Event::Start(fld))?;
        }
        // File element
        let path = file.path.to_slash_lossy().to_string();
        let name = file
            .path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let mut f = BytesStart::new("file-contents");
        f.push_attribute(("path", path.as_str()));
        f.push_attribute(("name", name.as_str()));
        writer.write_event(Event::Start(f))?;
        // Write file contents with trailing newline without escaping
        let mut text_buf = file.contents.clone();
        text_buf.push('\n');
        writer.write_event(Event::Text(BytesText::new(&text_buf)))?;
        writer.write_event(Event::End(BytesEnd::new("file-contents")))?;
    }
    // Close last folder
    if current_folder.is_some() {
        writer.write_event(Event::End(BytesEnd::new("folder")))?;
    }
    // Close root
    writer.write_event(Event::End(BytesEnd::new("context-gather")))?;

    // Convert buffer to String
    let buf = writer.into_inner();
    let s = String::from_utf8(buf)?;
    Ok(s)
}

// Round-trip test: ensure build_xml output can be parsed back to original content
#[cfg(test)]
mod tests {
    use super::*;
    use crate::gather::FileContents;
    use quick_xml::Reader;
    use quick_xml::events::Event;
    use std::path::PathBuf;

    #[test]
    fn roundtrip_xml_plain_text() {
        let files = vec![FileContents {
            folder: PathBuf::from("f"),
            path: PathBuf::from("f/t.txt"),
            contents: "Hello & <world>".to_string(),
        }];
        let xml = build_xml(&files).unwrap();
        let mut reader = Reader::from_str(&xml);
        reader.config_mut().trim_text(false);
        let mut buf = Vec::new();
        let extracted = loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) if e.name().as_ref() == b"file-contents" => {
                    buf.clear();
                    match reader.read_event_into(&mut buf) {
                        Ok(Event::Text(e)) => break e.unescape().unwrap().to_string(),
                        Ok(Event::Eof) => panic!("Unexpected EOF in file-contents"),
                        other => panic!("Expected text but got {other:?}"),
                    }
                }
                Ok(Event::Eof) => panic!("No <file-contents> start found"),
                _ => buf.clear(),
            }
        };
        assert_eq!(extracted, "Hello & <world>\n");
    }
}
