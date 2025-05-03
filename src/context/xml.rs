use crate::context::types::FileContents;
use crate::tokenizer::count as count_tokens;
use anyhow::Result;
use path_slash::PathBufExt;

/// Builds a simple XML-like structure grouping files by folder, with a file map and no escaping.
pub fn build_xml(files: &[FileContents]) -> Result<String> {
    let mut xml = String::new();
    xml.push_str("<shared-context>\n");
    // File map section
    xml.push_str(&format!("  <file-map total-files=\"{}\">\n", files.len()));
    for (id, file) in files.iter().enumerate() {
        let path = file.path.to_slash_lossy().to_string();
        let tokens = count_tokens(&file.contents);
        xml.push_str(&format!(
            "    <file id=\"{id}\" path=\"{path}\" tokens=\"{tokens}\" parts=\"1\"/>\n"
        ));
    }
    xml.push_str("  </file-map>\n");
    // Group by folder
    let mut current_folder: Option<String> = None;
    for file in files {
        let folder = file.folder.to_slash_lossy().to_string();
        if current_folder.as_ref() != Some(&folder) {
            if current_folder.is_some() {
                xml.push_str("  </folder>\n");
            }
            current_folder = Some(folder.clone());
            xml.push_str(&format!("  <folder path=\"{folder}\">\n"));
        }
        let path = file.path.to_slash_lossy().to_string();
        let name = file
            .path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        xml.push_str(&format!(
            "    <file-contents path=\"{path}\" name=\"{name}\">\n"
        ));
        // Raw contents:
        xml.push_str(&file.contents);
        xml.push('\n');
        xml.push_str("    </file-contents>\n");
    }
    if current_folder.is_some() {
        xml.push_str("  </folder>\n");
    }
    xml.push_str("</shared-context>\n");
    Ok(xml)
}
