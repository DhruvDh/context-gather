pub use crate::context::types::FileContents;

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Result, anyhow};
use glob::glob;
use ignore::WalkBuilder;

pub fn expand_paths(paths: Vec<String>) -> Result<Vec<PathBuf>> {
    let mut expanded = Vec::new();

    for p in paths {
        // Normalize Windows path separators for glob patterns
        let pattern = p.replace('\\', "/");
        let pattern_results =
            glob(&pattern).map_err(|e| anyhow!("Invalid glob pattern {}: {:?}", pattern, e))?;

        // If no matches, consider it a normal path
        let mut has_match = false;
        for path_res in pattern_results {
            has_match = true;
            let path = path_res?;
            expanded.push(path);
        }
        // If it's not a valid glob or no matches found, treat as a literal path
        if !has_match {
            expanded.push(PathBuf::from(&p));
        }
    }

    Ok(expanded)
}

/// Returns all file paths (recursively) if any of them are directories.
pub fn gather_all_file_paths(paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut results = Vec::new();

    for path in paths {
        // Recursively gather files, letting WalkBuilder handle ignore files
        let walker = WalkBuilder::new(path)
            .follow_links(false) // Adjust if you want to follow symlinks
            .standard_filters(true) // Respects hidden files and default filters
            .add_custom_ignore_filename(".gitignore")
            .build();

        for entry_result in walker {
            match entry_result {
                Ok(entry) => {
                    if entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
                        results.push(entry.path().to_path_buf());
                    }
                }
                Err(e) => {
                    tracing::warn!("Could not process entry in {:?}: {:?}", path, e);
                }
            }
        }
    }

    results.sort();
    results.dedup();
    Ok(results)
}

/// Reads the contents of each file path into `FileContents`, enforcing a
/// maximum size.
pub fn collect_file_data(
    file_paths: &[PathBuf],
    max_size: u64,
    root: &Path,
) -> Result<Vec<FileContents>> {
    let mut results = Vec::new();
    for path in file_paths {
        match read_file(path, max_size, root) {
            Ok(fc) => results.push(fc),
            Err(e) => eprintln!("{e}"),
        }
    }
    // Sort by folder then file name
    results.sort_by(|a, b| {
        let folder_cmp = a.folder.cmp(&b.folder);
        if folder_cmp == std::cmp::Ordering::Equal {
            a.path.cmp(&b.path)
        } else {
            folder_cmp
        }
    });
    Ok(results)
}

/// Returns the number of tokens in the given text.
pub fn count_tokens(text: &str) -> usize {
    crate::tokenizer::count(text)
}

pub fn read_file(
    path: &Path,
    max_size: u64,
    root: &Path,
) -> Result<FileContents> {
    // Enforce the maximum file size
    let metadata = fs::metadata(path)?;
    if metadata.len() > max_size {
        return Err(anyhow!(
            "Warning: {:?} exceeds {} bytes. Skipping.",
            path,
            max_size
        ));
    }
    // Read the entire file into memory
    let content_bytes = fs::read(path)?;
    // Convert to UTF-8; treat invalid UTF-8 as binary
    let contents = String::from_utf8(content_bytes)
        .map_err(|_| anyhow!("Warning: {:?} appears to be a binary file. Skipping.", path))?;
    let rel_path = path.strip_prefix(root).unwrap_or(path).to_path_buf();
    let folder = rel_path
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .to_path_buf();
    Ok(FileContents {
        folder,
        path: rel_path,
        contents,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{env, fs};

    #[test]
    fn utf8_non_ascii_is_not_binary() -> anyhow::Result<()> {
        let dir = env::temp_dir();
        let fp = dir.join("ctx_gather_test");
        let s = "é 中文 ";
        fs::write(&fp, s)?;
        let root = env::current_dir()?;
        let files = collect_file_data(std::slice::from_ref(&fp), u64::MAX, &root)?;
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].contents, s);
        let _ = fs::remove_file(&fp);
        Ok(())
    }
}
