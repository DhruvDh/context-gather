// Import modules from the library crate
use context_gather::chunker;
use context_gather::cli::Cli;
use context_gather::gather;
use context_gather::header;
use context_gather::io::clipboard;
use context_gather::ui::select_files_tui;
use context_gather::xml_output;

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
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
        candidate_files = match select_files_tui(candidate_files, &preselected_paths) {
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
    let (mut data_chunks, metas) = chunker::build_chunks(&file_data, cli.chunk_size);
    // Build full set of chunks including header
    use context_gather::gather::count_tokens;
    let total_chunks = data_chunks.len() + 1; // +1 for header
    // Create header XML: opens <shared-context> and includes file-map header
    let header_xml = format!(
        "<shared-context>\n{}\n",
        header::make_header(total_chunks, cli.chunk_size, &metas)
    );
    let header_chunk = chunker::Chunk {
        index: 0,
        tokens: count_tokens(&header_xml),
        xml: header_xml,
    };
    // Renumber data chunks and prepend header
    for (i, chunk) in data_chunks.iter_mut().enumerate() {
        chunk.index = i + 1;
    }
    let mut chunks = Vec::with_capacity(total_chunks);
    chunks.push(header_chunk);
    chunks.extend(data_chunks);

    // Interactive streaming: copy full XML with context-chunk wrappers
    if cli.interactive {
        use std::io::{self, Write};
        let total = chunks.len();
        let mut idx = 0usize;
        println!("▲ Streaming {} chunks (0..{}).", total, total - 1);
        loop {
            let rem = total - idx - 1;
            // Build snippet for this chunk
            let snippet = if idx == 0 {
                // Header: includes opening <shared-context> and header
                let mut s = chunks[0].xml.clone();
                if rem > 0 {
                    s.push_str(&format!("<more remaining=\"{rem}\"/>\n"));
                }
                s
            } else if rem > 0 {
                format!(
                    "<context-chunk id=\"{}/{}\">\n{}</context-chunk>\n<more remaining=\"{}\"/>\n",
                    idx, total, chunks[idx].xml, rem
                )
            } else {
                format!(
                    "<context-chunk id=\"{}/{}\">\n{}</context-chunk>\n</shared-context>\n",
                    idx, total, chunks[idx].xml
                )
            };
            if !cli.no_clipboard {
                clipboard::copy_to_clipboard(&snippet, false)?;
                println!("✔ copied chunk {idx}");
            }
            if cli.stdout {
                print!("{snippet}");
            }
            print!("Enter chunk # (0..{}) or 'q' to quit: ", total - 1);
            io::stdout().flush().unwrap();
            let mut cmd = String::new();
            io::stdin().read_line(&mut cmd).unwrap();
            let cmd = cmd.trim();
            if cmd == "q" {
                break;
            }
            idx = if cmd.is_empty() {
                (idx + 1) % total
            } else {
                cmd.parse().unwrap_or(idx)
            };
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
    // Non-interactive: for each chunk, print and/or copy full XML snippet
    let total_chunks = chunks.len();
    for (i, chunk) in chunks.iter().enumerate() {
        let rem = total_chunks - i - 1;
        let snippet = if i == 0 {
            let mut s = chunk.xml.clone();
            if rem > 0 {
                s.push_str(&format!("<more remaining=\"{rem}\"/>\n"));
            }
            s
        } else if rem > 0 {
            format!(
                "<context-chunk id=\"{}/{}\">\n{}</context-chunk>\n<more remaining=\"{}\"/>\n",
                i, total_chunks, chunk.xml, rem
            )
        } else {
            format!(
                "<context-chunk id=\"{}/{}\">\n{}</context-chunk>\n</shared-context>\n",
                i, total_chunks, chunk.xml
            )
        };
        if cli.stdout {
            print!("{snippet}");
        }
        if copy_idx == i as isize && !cli.no_clipboard {
            clipboard::copy_to_clipboard(&snippet, false)?;
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
