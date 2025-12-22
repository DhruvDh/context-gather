use crate::context::types::FileContents;
use crate::tokenizer::count as count_tokens;
use anyhow::Result;
use path_slash::PathBufExt;
use std::borrow::Cow;

fn escape_xml_inner(
    s: &str,
    escape_quotes: bool,
) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' if escape_quotes => out.push_str("&quot;"),
            '\'' if escape_quotes => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
    out
}

fn needs_escape_attr(s: &str) -> bool {
    s.bytes()
        .any(|b| matches!(b, b'&' | b'<' | b'>' | b'"' | b'\''))
}

pub(crate) fn maybe_escape_text<'a>(
    s: &'a str,
    escape_xml: bool,
) -> Cow<'a, str> {
    if escape_xml {
        Cow::Owned(escape_xml_inner(s, false))
    } else {
        Cow::Borrowed(s)
    }
}

pub(crate) fn maybe_escape_attr<'a>(
    s: &'a str,
    escape_xml: bool,
) -> Cow<'a, str> {
    if escape_xml || needs_escape_attr(s) {
        Cow::Owned(escape_xml_inner(s, true))
    } else {
        Cow::Borrowed(s)
    }
}

/// Builds a simple XML-like structure grouping files by folder.
pub fn build_xml(files: &[FileContents]) -> Result<String> {
    build_xml_with_escape(files, false)
}

/// Builds a simple XML-like structure grouping files by folder, with optional XML escaping.
pub fn build_xml_with_escape(
    files: &[FileContents],
    escape_xml: bool,
) -> Result<String> {
    let mut xml = String::new();
    xml.push_str("<shared-context>\n");
    // File map section
    xml.push_str(&format!("  <file-map total-files=\"{}\">\n", files.len()));
    for (id, file) in files.iter().enumerate() {
        let path = file.path.to_slash_lossy().to_string();
        let path_attr = maybe_escape_attr(&path, escape_xml);
        let contents = maybe_escape_text(&file.contents, escape_xml);
        let tokens = count_tokens(contents.as_ref());
        xml.push_str(&format!(
            "    <file id=\"{id}\" path=\"{path}\" tokens=\"{tokens}\" parts=\"1\"/>\n",
            path = path_attr
        ));
    }
    xml.push_str("  </file-map>\n");
    // Group by folder
    let mut current_folder: Option<String> = None;
    for file in files {
        let folder = file.folder.to_slash_lossy().to_string();
        let folder_display = if folder.is_empty() {
            ".".to_string()
        } else {
            folder
        };
        let folder_attr = maybe_escape_attr(&folder_display, escape_xml);
        if current_folder.as_ref() != Some(&folder_display) {
            if current_folder.is_some() {
                xml.push_str("  </folder>\n");
            }
            current_folder = Some(folder_display.clone());
            xml.push_str(&format!(
                "  <folder path=\"{folder}\">\n",
                folder = folder_attr
            ));
        }
        let path = file.path.to_slash_lossy().to_string();
        let name = file
            .path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let path_attr = maybe_escape_attr(&path, escape_xml);
        let name_attr = maybe_escape_attr(&name, escape_xml);
        let contents = maybe_escape_text(&file.contents, escape_xml);
        xml.push_str(&format!(
            "    <file-contents path=\"{path}\" name=\"{name}\">\n",
            path = path_attr,
            name = name_attr
        ));
        // Raw contents:
        xml.push_str(contents.as_ref());
        xml.push('\n');
        xml.push_str("    </file-contents>\n");
    }
    if current_folder.is_some() {
        xml.push_str("  </folder>\n");
    }
    xml.push_str("</shared-context>\n");
    Ok(xml)
}
