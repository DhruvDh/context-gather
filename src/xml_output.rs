use super::gather::FileContents;
use std::path::PathBuf;

pub fn build_xml(files: &[FileContents]) -> String {
    if files.is_empty() {
        return "".to_string();
    }

    // We iterate folder by folder
    let mut current_folder: Option<&PathBuf> = None;
    let mut output = String::new();

    for file in files {
        // If this is a new folder, close the old folder tag and open a new one
        if current_folder.is_none() || current_folder.unwrap() != &file.folder {
            // Close the previous folder if needed
            if current_folder.is_some() {
                output.push_str("  </folder>\n");
                output.push_str("\n");
            }
            current_folder = Some(&file.folder);

            // Start new folder
            let folder_str = file.folder.to_string_lossy();
            output.push_str(&format!("  <folder path=\"{}\">\n", folder_str));
        }

        // Add file contents
        let file_name = file.path.file_name()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_else(|| "unknown".to_string());
        let path_str = file.path.to_string_lossy();
        output.push_str(&format!("    <file-contents path=\"{}\" name=\"{}\">\n",
                                 path_str, file_name));
        // Indent file contents for readability, or just dump them as-is
        let escaped_contents = escape_special_chars(&file.contents);
        output.push_str(&format!("{}\n", escaped_contents));
        output.push_str("    </file-contents>\n");
    }

    // Close the last folder
    if current_folder.is_some() {
        output.push_str("  </folder>\n");
    }

    // Wrap everything in a top-level XML-ish tag for clarity
    format!("<current-context>\n{}\n</current-context>", output)
}

/// Escape special characters if needed (optional)
fn escape_special_chars(s: &str) -> String {
    // Very naive example:
    s.replace("&", "&amp;")
     .replace("<", "&lt;")
     .replace(">", "&gt;")
}
