use anyhow::{Result, anyhow};
use glob::glob;
use std::{
    fs,
    io::{Read, BufReader},
    path::{Path, PathBuf},
};
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

pub fn collect_file_data(paths: &[PathBuf]) -> Result<Vec<FileContents>> {
    let mut results = Vec::new();

    for path in paths {
        if !path.is_file() {
            eprintln!("Warning: {:?} is not a file. Skipping.", path);
            continue;
        }

        // Attempt to read the file
        match fs::File::open(path) {
            Ok(file) => {
                let mut reader = BufReader::new(file);
                // Try reading to string
                let mut content = String::new();
                if let Err(_) = reader.read_to_string(&mut content) {
                    eprintln!("Warning: {:?} is not a valid text file. Skipping.", path);
                    continue;
                }

                // If successful, store results
                results.push(FileContents {
                    folder: path.parent().unwrap_or_else(|| Path::new("")).to_path_buf(),
                    path: path.clone(),
                    contents: content,
                });
            }
            Err(e) => {
                eprintln!("Warning: Could not open {:?}: {:?}", path, e);
            }
        }
    }

    // Sort results by folder (then by file name)
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
