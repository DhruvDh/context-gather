mod chunker;
mod cli;
mod clipboard;
mod gather;
mod interactive;
mod xml_output;

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use cli::Cli;
use glob::Pattern;
use path_slash::PathBufExt;

fn main() -> Result<()> {
    let cli = Cli::parse();

    // 1) Expand user-specified paths (globs, etc.)
    let user_paths_raw = gather::expand_paths(cli.paths)?;

    // Helper: check if `candidate` is "under" any user-specified path (including
    // exact matches).
    fn is_preselected(
        candidate: &std::path::Path,
        user_paths: &[PathBuf],
    ) -> bool {
        // Attempt to canonicalize the candidate; skip if it fails
        let cand_canon = match dunce::canonicalize(candidate) {
            Ok(c) => c,
            Err(_) => return false,
        };

        // If any user path is a parent of `cand_canon` or exact match => true
        for up in user_paths {
            // Canonicalize user path
            if let Ok(up_canon) = dunce::canonicalize(up) {
                // starts_with() means `cand_canon` is inside or equal to `up_canon`
                if cand_canon.starts_with(&up_canon) {
                    return true;
                }
            }
        }
        false
    }

    // 2) Build candidate file list: include explicit files and files under
    //    directories
    let mut candidate_files: Vec<PathBuf> = Vec::new();
    let mut dirs_to_scan: Vec<PathBuf> = Vec::new();
    for up in &user_paths_raw {
        if up.is_dir() {
            dirs_to_scan.push(up.clone());
        } else {
            candidate_files.push(up.clone());
        }
    }
    if !dirs_to_scan.is_empty() {
        candidate_files.extend(gather::gather_all_file_paths(&dirs_to_scan)?);
    }

    // 3) Among those gathered, preselect anything "under" or exactly matching user
    //    paths
    let preselected_paths: Vec<PathBuf> = candidate_files
        .iter()
        .filter(|cand| is_preselected(cand, &user_paths_raw))
        .cloned()
        .collect();

    // 4) If interactive, open the TUI
    if cli.interactive {
        candidate_files = match interactive::select_files_tui(candidate_files, &preselected_paths) {
            Ok(selected) => selected,
            Err(e) => {
                eprintln!("Error in interactive TUI: {e}");
                std::process::exit(1);
            }
        };
    }

    // 5) Exclude patterns: abort if all provided globs are invalid
    let raw_patterns: Vec<String> = cli.exclude.iter().map(|p| p.replace('\\', "/")).collect();
    let patterns: Vec<Pattern> = raw_patterns
        .iter()
        .filter_map(|p| Pattern::new(p).ok())
        .collect();
    if !raw_patterns.is_empty() && patterns.is_empty() {
        eprintln!("Error: every --exclude pattern was invalid: {raw_patterns:?}");
        std::process::exit(2);
    }
    if !patterns.is_empty() {
        candidate_files.retain(|path| {
            let p = path.to_slash_lossy();
            !patterns.iter().any(|pat| pat.matches(p.as_ref()))
        });
    }
    // Exclusion filtering applied

    // 6) Read file data and build XML
    let file_data = gather::collect_file_data(&candidate_files, cli.max_size)?;
    let xml_output = xml_output::build_xml(&file_data)?;

    // 7) Chunking and output streaming
    let chunks = chunker::chunk_by_tokens(&xml_output, cli.chunk_size);
    for (idx, part) in chunks.iter().enumerate() {
        let decorated = if cli.emit_markers {
            format!(
                "<context-part index=\"{}\" of=\"{}\">\n{}\n</context-part>",
                idx + 1,
                chunks.len(),
                part
            )
        } else {
            part.clone()
        };

        // 7a) Clipboard per chunk (soft failure)
        if !cli.no_clipboard {
            clipboard::copy_to_clipboard(&decorated, false)?;
        }
        // 7b) Stdout per chunk
        if cli.stdout {
            println!("{decorated}");
            if cli.emit_markers && idx + 1 < chunks.len() {
                println!("<more/>");
            }
        }
    }

    // 8) Count tokens and print summary with chunk count
    let token_count = gather::count_tokens(&xml_output);
    println!(
        "✔ Processed {} files ({} tokens) in {} chunks{}",
        file_data.len(),
        token_count,
        chunks.len(),
        if !cli.no_clipboard {
            " to clipboard"
        } else {
            ""
        }
    );

    // 9) Warn if token count exceeds model context limit
    if let Some(limit) = cli.model_context {
        if token_count > limit {
            eprintln!("Warning: token count {token_count} exceeds model context limit {limit}");
        }
    }

    Ok(())
}
