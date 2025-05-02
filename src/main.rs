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

    // If chunking disabled (-c 0), output full XML as a single chunk
    if cli.chunk_size == 0 {
        // Print XML on stdout if requested or interactive
        if cli.stdout || cli.interactive {
            println!("{xml_output}");
        }
        // Copy to clipboard
        if !cli.no_clipboard {
            clipboard::copy_to_clipboard(&xml_output, false)?;
        }
        // Summary: one chunk (index 0)
        let token_count = gather::count_tokens(&xml_output);
        println!(
            "✔ {} files • {} tokens • 1 chunk • copied={}",
            file_data.len(),
            token_count,
            if !cli.no_clipboard { "0" } else { "none" }
        );
        return Ok(());
    }
    // Precompute token count for summary
    let token_count = gather::count_tokens(&xml_output);

    // 7) Smart chunking with metadata
    let (chunks, _metas) = chunker::build_chunks(&file_data, cli.chunk_size);

    // Interactive mode: prompt to copy/print chunks sequentially
    if cli.interactive {
        use std::io::{self, Write};
        // Print header chunk and marker
        let header_xml = &chunks[0].xml;
        if cli.stdout {
            println!("{header_xml}");
            if chunks.len() > 1 {
                println!("<more/>");
            }
        }
        // Prompt for chunks
        let total = chunks.len();
        println!("▲ There are {} chunks (0..{}).", total, total - 1);
        println!("   Press Enter to copy chunk 0, or enter chunk # to copy/print, or 'q' to quit.");
        let mut idx = 0usize;
        loop {
            print!("[{idx}] > ");
            io::stdout().flush().unwrap();
            let mut line = String::new();
            io::stdin().read_line(&mut line).unwrap();
            let cmd = line.trim();
            if cmd == "q" {
                break;
            }
            let n = if cmd.is_empty() {
                idx
            } else {
                match cmd.parse::<usize>() {
                    Ok(v) if v < total => v,
                    Ok(v) => {
                        println!("Invalid chunk index: {}. Range is 0..{}", v, total - 1);
                        continue;
                    }
                    Err(_) => {
                        println!("Unknown command '{cmd}', enter a number or 'q'");
                        continue;
                    }
                }
            };
            // Print and copy selected chunk
            let xml = &chunks[n].xml;
            if cli.stdout {
                println!("{xml}");
                if n + 1 < total {
                    println!("<more/>");
                }
            }
            if !cli.no_clipboard {
                clipboard::copy_to_clipboard(xml, false)?;
                println!("✔ copied chunk {n}");
            }
            idx = (n + 1) % total;
        }
        return Ok(());
    }

    // Determine default copy index: default to first chunk when unset and clipboard enabled
    let mut copy_idx = cli.chunk_index;
    if copy_idx == -1 && !cli.no_clipboard {
        copy_idx = 0;
    }
    // Non-interactive: handle selected chunk or print all
    if cli.chunk_index >= 0 && cli.chunk_size == 0 {
        eprintln!("error: `--chunk-index` requires `--chunk-size`");
        std::process::exit(2);
    }
    if copy_idx >= chunks.len() as isize {
        eprintln!(
            "⚠  --chunk-index {} out of range (0..{})",
            copy_idx,
            chunks.len().saturating_sub(1)
        );
        std::process::exit(3);
    }
    // Non-interactive: wrap each chunk in <context-chunk> with id and remaining marker
    let total_chunks = chunks.len();
    for (i, chunk) in chunks.iter().enumerate() {
        if cli.stdout {
            println!(r#"<context-chunk id="{}/{}">"#, i, total_chunks);
            println!("{}", chunk.xml);
            println!("</context-chunk>");
            let remaining = total_chunks - i - 1;
            if remaining > 0 {
                println!(r#"<more remaining="{}"/>"#, remaining);
            }
        }
        if copy_idx == i as isize {
            clipboard::copy_to_clipboard(&chunk.xml, false)?;
        }
    }
    // 8) Summary
    println!(
        "✔ {} files • {} tokens • {} chunks • copied={}",
        file_data.len(),
        token_count,
        chunks.len(),
        if copy_idx >= 0 {
            copy_idx.to_string()
        } else {
            "none".into()
        }
    );
    if cli.no_clipboard && !cli.stdout {
        eprintln!("Note: neither --stdout nor clipboard copy requested; nothing visible.");
    }

    // 9) Warn if token count exceeds model context limit
    if let Some(limit) = cli.model_context {
        if token_count > limit {
            eprintln!("Warning: token count {token_count} exceeds model context limit {limit}");
        }
    }

    Ok(())
}
