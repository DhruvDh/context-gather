# `ctx-gather`

Below is a step-by-step outline for implementing a `ctx-gather` Rust CLI tool that meets your specifications. It’s designed to do the following:

1. **Accept file paths (and glob patterns) on the command line.**  
2. **Optionally open a TUI** for interactive file selection when a flag (e.g., `-i` or `--interactive`) is used.  
3. **Gather contents** of the specified text files.  
4. **Group them by folder** in an XML-like structure for clarity.  
5. **Copy** the resulting XML output to the clipboard.  
6. **Token-count** the resulting output using `tiktoken_rs`.  
7. **Handle non-text files** gracefully (warn, but do not fail).  

Below, I’ll describe an example architecture, key dependencies, and pseudo-code to illustrate how the various steps tie together.

## 1. Project Structure

A recommended project layout:

```
ctx-gather/
├─ Cargo.toml
└─ src/
   ├─ main.rs
   ├─ cli.rs          // Arg parsing
   ├─ interactive.rs  // TUI functionality
   ├─ gather.rs       // Core logic for gathering, grouping, etc.
   ├─ xml_output.rs   // Functions to generate the XML-like output
   └─ clipboard.rs    // Clipboard integration
```

This structure is not mandatory, but it keeps different components modular and easier to maintain.

## 2. Dependencies

In your `Cargo.toml`, include:

```toml
[package]
name = "ctx-gather"
version = "0.1.0"
edition = "2024"

[dependencies]
anyhow = "1.0.75"
clap = { version = "4.4.6", features = ["derive"] }
copypasta = "0.10.0"
crossterm = "0.27.0"
glob = "0.3.1"
tiktoken-rs = "0.5.4"
tui = "0.19.0"
```

(Adjust versions to the latest semver releases as needed.)

## 3. Parsing Command-Line Arguments (in `cli.rs`)

Use whichever CLI parser you prefer (e.g., [`clap`](https://github.com/clap-rs/clap)). Example:

```rust
use clap::{Parser, Arg};

#[derive(Parser, Debug)]
#[command(name = "ctx-gather")]
#[command(about = "Gather text file contents, group them by folder, output as XML to clipboard, then show token count.")]
pub struct Cli {
    /// File paths (supporting globs)
    #[arg(required = true)]
    pub paths: Vec<String>,

    /// If set, opens the TUI for interactive selection.
    #[arg(short = 'i', long = "interactive")]
    pub interactive: bool,
}
```

## 4. Entry Point in `main.rs`

```rust
mod cli;
mod interactive;
mod gather;
mod xml_output;
mod clipboard;

use cli::Cli;
use clap::Parser;
use anyhow::Result;

fn main() -> Result<()> {
    let cli = Cli::parse();

    // 1. Expand globs and gather file paths
    let mut all_paths = gather::expand_paths(cli.paths)?;

    // 2. If `interactive` is true, launch TUI to select files
    if cli.interactive {
        all_paths = interactive::select_files_tui(all_paths)?;
    }

    // 3. Gather file contents, ignoring non-text files (with warnings)
    let file_data = gather::collect_file_data(&all_paths)?;

    // 4. Build XML-like output grouped by folder
    let xml_output = xml_output::build_xml(&file_data);

    // 5. Copy to clipboard
    clipboard::copy_to_clipboard(&xml_output)?;

    // 6. Count tokens and print them
    gather::count_tokens(&xml_output)?;

    Ok(())
}
```

### 4.1 Handling Globs

In `gather.rs` (or a dedicated utility file), you might have:

```rust
use anyhow::{Result, anyhow};
use glob::glob;
use std::path::PathBuf;

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
```

## 5. Interactive TUI (in `interactive.rs`)

This is a sketch of how you could structure the TUI:

```rust
use std::path::PathBuf;
use anyhow::Result;

// This would be more elaborate in practice with TUI rendering, etc.
pub fn select_files_tui(paths: Vec<PathBuf>) -> Result<Vec<PathBuf>> {
    // 1. Initialize TUI with crossterm or similar
    // 2. Present a list of files with checkboxes or selected states.
    // 3. Let user toggle selection using arrow keys + space.
    // 4. On Enter, exit with the selected paths.

    // For demonstration, simply return them all as selected
    Ok(paths)
}
```

Of course, you would implement the actual TUI rendering loop (with `tui::Terminal`, etc.), but the above shows how it might fit into the overall flow.

## 6. Gathering and Grouping File Contents

In `gather.rs`:

```rust
use anyhow::{Result, anyhow};
use std::{
    fs,
    io::{Read, BufReader},
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct FileContents {
    pub folder: PathBuf,
    pub path: PathBuf,
    pub contents: String,
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

// For token counting
use tiktoken_rs::o200k_base;

pub fn count_tokens(text: &str) -> Result<()> {
    let bpe = o200k_base()?;
    let tokens = bpe.encode_with_special_tokens(text);
    println!("Token count: {}", tokens.len());
    Ok(())
}
```

## 7. Generating the XML-Like Output (in `xml_output.rs`)

Your XML-like structure might look like:

```xml
<folder path="src">
  <file-contents path="src/main.rs" name="main.rs">
  // file contents
  </file-contents>
  <file-contents path="src/lib.rs" name="lib.rs">
  // file contents
  </file-contents>
</folder>
```

Here’s a possible approach:

```rust
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
    format!("<ctx-gather>\n{}\n</ctx-gather>", output)
}

/// Escape special characters if needed (optional)
fn escape_special_chars(s: &str) -> String {
    // Very naive example:
    s.replace("&", "&amp;")
     .replace("<", "&lt;")
     .replace(">", "&gt;")
}
```

Note that escaping may be helpful if you want to ensure valid XML. You can skip it if it’s purely for an LLM “context” use case and you’re confident the LLM can handle angle brackets.

## 8. Clipboard Integration (in `clipboard.rs`)

Using [`copypasta`](https://crates.io/crates/copypasta):

```rust
use anyhow::Result;
use copypasta::{ClipboardContext, ClipboardProvider};

pub fn copy_to_clipboard(text: &str) -> Result<()> {
    let mut ctx = ClipboardContext::new()
        .map_err(|e| anyhow::anyhow!("Failed to create clipboard context: {:?}", e))?;
    ctx.set_contents(text.to_string())
        .map_err(|e| anyhow::anyhow!("Failed to copy to clipboard: {:?}", e))?;
    Ok(())
}
```

## 9. Overall Flow

Putting it all together, your CLI will:

1. **Parse arguments** (including `--interactive`).  
2. **Expand globs** and gather a list of files.  
3. If `--interactive`, **show TUI** to let the user unselect or select files.  
4. **Collect file contents**; for each file that isn’t valid text, log a warning.  
5. **Generate an XML-like string**, grouping by folder.  
6. **Copy** that string to the clipboard.  
7. **Count tokens** using `tiktoken_rs`.  
8. **Print** the token count.  

By default (without `--interactive`), it just does steps 1, 2, 4–8 and finishes immediately.