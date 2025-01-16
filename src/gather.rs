use anyhow::{Result, anyhow};
use glob::glob;
use std::{
    fs,
    io::{Read, BufReader},
    path::{Path, PathBuf},
};
use walkdir::WalkDir;
use tiktoken_rs::o200k_base;

#[derive(Debug)]
pub struct FileContents {
    pub folder: PathBuf,
    pub path: PathBuf,
    pub contents: String,
}

pub fn expand_paths(paths: Vec<String>) -> Result<Vec<PathBuf>> {
    let mut expanded = Vec::new();

    for p in paths {
        // Attempt to treat it like a glob first
        let pattern_results = glob(&p)
            .map_err(|e| anyhow!("Invalid glob pattern {}: {:?}", p, e))?;

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
        if path.is_dir() {
            // Recursively gather files in directories
            for entry in WalkDir::new(path) {
                match entry {
                    Ok(e) => {
                        if e.file_type().is_file() {
                            results.push(e.path().to_path_buf());
                        }
                    }
                    Err(e) => {
                        eprintln!("Warning: Could not read entry in directory {:?}: {:?}", path, e);
                    }
                }
            }
        } else if path.is_file() {
            results.push(path.to_path_buf());
        } else {
            eprintln!("Warning: {:?} is neither file nor directory. Skipping.", path);
        }
    }

    results.sort();
    results.dedup();
    Ok(results)
}

/// Reads the contents of each file path into `FileContents`.
pub fn collect_file_data(file_paths: &[PathBuf]) -> Result<Vec<FileContents>> {
    let mut results = Vec::new();
    for path in file_paths {
        match read_file(path) {
            Ok(fc) => results.push(fc),
            Err(e) => eprintln!("{}", e),
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

pub fn count_tokens(text: &str) -> Result<()> {
    let bpe = o200k_base()?;
    let tokens = bpe.encode_with_special_tokens(text);
    println!("Token count: {}", tokens.len());
    Ok(())
}
fn read_file(path: &Path) -> Result<FileContents> {
    let file = fs::File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut content = String::new();
    reader.read_to_string(&mut content)
        .map_err(|_| anyhow!("Warning: {:?} is not a valid text file. Skipping.", path))?;

    Ok(FileContents {
        folder: path.parent().unwrap_or_else(|| Path::new("")).to_path_buf(),
        path: path.to_path_buf(),
        contents: content,
    })
}
