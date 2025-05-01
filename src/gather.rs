use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Result, anyhow};
use glob::glob;
use ignore::WalkBuilder;
use once_cell::sync::Lazy;
use tiktoken_rs::{CoreBPE, o200k_base};

#[derive(Debug)]
pub struct FileContents {
    pub folder:   PathBuf,
    pub path:     PathBuf,
    pub contents: String,
}

static TOKENIZER: Lazy<CoreBPE> =
    Lazy::new(|| o200k_base().expect("Failed to initialize tokenizer"));

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
        // Recursively gather files with `.gitignore` support
        let walker = WalkBuilder::new(path).follow_links(false) // Adjust if you want to follow symlinks
                                           .standard_filters(true) // Respects .gitignore, hidden files, etc.
                                           .build();

        for entry_result in walker {
            match entry_result {
                Ok(entry) => {
                    if entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
                        results.push(entry.path().to_path_buf());
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Could not process entry in {path:?}: {e:?}");
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
pub fn collect_file_data(file_paths: &[PathBuf],
                         max_size: u64)
                         -> Result<Vec<FileContents>> {
    let mut results = Vec::new();
    for path in file_paths {
        match read_file(path, max_size) {
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
    let tokens = TOKENIZER.encode_with_special_tokens(text);
    tokens.len()
}

fn read_file(path: &Path,
             max_size: u64)
             -> Result<FileContents> {
    // Enforce the maximum file size
    let metadata = fs::metadata(path)?;
    if metadata.len() > max_size {
        return Err(anyhow!("Warning: {:?} exceeds {} bytes. Skipping.",
                           path,
                           max_size));
    }
    // Read the entire file into memory and detect binary
    let content_bytes = fs::read(path)?;
    // Simple binary detection: check first 4KiB for non-text bytes
    let sample_size = content_bytes.len().min(4096);
    let non_text = content_bytes[..sample_size].iter()
                                               .filter(|&&b| b == 0 || b > 0x7F)
                                               .count();
    if sample_size > 0 && (non_text as f64) / (sample_size as f64) > 0.3 {
        return Err(anyhow!("Warning: {:?} appears to be a binary file. \
                            Skipping.",
                           path));
    }
    // Convert to UTF-8, replacing invalid sequences to avoid double allocation
    let contents = String::from_utf8_lossy(&content_bytes).into_owned();
    Ok(FileContents { folder: path.parent().unwrap_or_else(|| Path::new("")).to_path_buf(),
                      path: path.to_path_buf(),
                      contents })
}
