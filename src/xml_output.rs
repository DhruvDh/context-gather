use anyhow::Result;
use path_slash::PathBufExt;
use quick_xml::{
    Writer,
    events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event},
};

use super::gather::FileContents;

/// Builds an XML-like structure grouping files by folder using a streaming
/// writer.
pub fn build_xml(files: &[FileContents]) -> Result<String> {
    // Initialize writer with indentation
    let mut writer = Writer::new_with_indent(Vec::new(), b' ', 2);
    // XML declaration
    writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;
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
        // Write file contents as text (escaped)
        writer.write_event(Event::Text(BytesText::from_escaped(file.contents.as_str())))?;
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
