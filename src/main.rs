// Import modules from the library crate
use context_gather::chunker;
use context_gather::config::Config;
use context_gather::gather;
use context_gather::header;
use context_gather::io::clipboard;
use context_gather::output;
use context_gather::ui::select_files_tui;
use context_gather::ui::stream::{multi_step_mode, streaming_mode};
use context_gather::xml_output;

use std::path::PathBuf;

use anyhow::Result;
use glob::Pattern;
use path_slash::PathBufExt;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    // Initialize tracing for structured logging, with RUST_LOG support
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let config = Config::from_cli()?;

    // Pre-validate CLI arg combos: chunk-index requires chunk-size > 0
    if matches!(config.chunk_size, Some(0)) {
        error!("--chunk-size must be > 0 (omit it to disable chunking)");
        std::process::exit(2);
    }
    if config.chunk_size.is_none() && config.chunk_index >= 0 {
        error!("--chunk-index requires --chunk-size > 0");
        std::process::exit(3);
    }
    let chunk_limit = config.chunk_size.unwrap_or(0);

    // 1) Expand user-specified paths (globs, etc.)
    let user_paths_raw = gather::expand_paths(config.paths.clone())?;

    // Helper: check if `candidate` is "under" any user-specified path (including
    // exact matches).
    fn is_preselected(
        candidate: &std::path::Path,
        user_paths: &[PathBuf],
    ) -> bool {
        user_paths.iter().any(|up| candidate.starts_with(up))
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

    // Canonicalize and deduplicate explicit and discovered files
    for path in &mut candidate_files {
        if let Ok(canon) = dunce::canonicalize(&*path) {
            *path = canon;
        }
    }
    candidate_files.sort();
    candidate_files.dedup();

    // 3) Among those gathered, preselect anything "under" or exactly matching user paths
    let user_paths_canon: Vec<PathBuf> = user_paths_raw
        .iter()
        .filter_map(|p| dunce::canonicalize(p).ok())
        .collect();
    let preselected_paths: Vec<PathBuf> = candidate_files
        .iter()
        .filter(|cand| is_preselected(cand, &user_paths_canon))
        .cloned()
        .collect();

    // 4) If interactive, open the TUI
    if config.interactive {
        candidate_files = match select_files_tui(candidate_files, &preselected_paths) {
            Ok(selected) => selected,
            Err(e) => {
                error!("Error in interactive TUI: {}", e);
                std::process::exit(1);
            }
        };
    }

    // 5) Exclude patterns: abort if all provided globs are invalid
    let raw_patterns: Vec<String> = config
        .exclude
        .iter()
        .map(|p| p.replace('\\', "/"))
        .collect();
    let patterns: Vec<Pattern> = raw_patterns
        .iter()
        .filter_map(|p| Pattern::new(p).ok())
        .collect();
    if !raw_patterns.is_empty() && patterns.is_empty() {
        error!("Every --exclude pattern was invalid: {:?}", raw_patterns);
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
    let file_data = gather::collect_file_data(&candidate_files, config.max_size)?;
    let xml_output = xml_output::build_xml_with_escape(&file_data, config.escape_xml)?;

    // Build smart chunks and metadata (header plus file parts), needed for multi-step and chunked modes
    let (mut data_chunks, metas) =
        chunker::build_chunks(&file_data, chunk_limit, config.escape_xml);
    use context_gather::gather::count_tokens;
    let total_chunks = data_chunks.len() + 1; // +1 for header
    // Create header XML: opens <shared-context> and includes file-map header
    let header_xml = format!(
        "<shared-context>\n{}\n",
        header::make_header(
            total_chunks,
            chunk_limit,
            &metas,
            config.multi_step,
            config.escape_xml,
        )
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

    // Multi-step mode: REPL for fetching files on demand
    if config.multi_step {
        multi_step_mode(&chunks, &file_data, &config)?;
        return Ok(());
    }

    // Chunked mode interactive REPL: only when interactive flag is set
    if chunk_limit > 0 && config.interactive {
        streaming_mode(&chunks, &config)?;
        return Ok(());
    }

    // If chunking disabled (-c 0), output full XML as a single chunk
    if chunk_limit == 0 {
        // Print XML on stdout if requested
        if config.stdout {
            println!("{xml_output}");
        }
        // Copy to clipboard
        if !config.no_clipboard {
            clipboard::copy_to_clipboard(&xml_output, false, !config.stdout)?;
        }
        // Summary: one chunk (index 0)
        let summary = if config.stdout && config.no_clipboard && config.model_context.is_none() {
            // Skip tokenisation for pure stdout + no-clipboard runs when no model_context
            format!(
                "OK {} files • 1 chunk • copied={}",
                file_data.len(),
                if !config.no_clipboard { "0" } else { "none" }
            )
        } else {
            let token_count = gather::count_tokens(&xml_output);
            format!(
                "OK {} files • {} tokens • 1 chunk • copied={}",
                file_data.len(),
                token_count,
                if !config.no_clipboard { "0" } else { "none" }
            )
        };
        println!("{summary}");
        return Ok(());
    }

    // Determine default copy index: default to first chunk when unset and clipboard enabled
    let mut copy_idx = config.chunk_index;
    if copy_idx == -1 && !config.no_clipboard {
        copy_idx = 0;
    }
    // Non-interactive: handle selected chunk or print all
    if copy_idx >= chunks.len() as isize {
        warn!(
            "--chunk-index {} out of range (0..{})",
            copy_idx,
            chunks.len().saturating_sub(1)
        );
        std::process::exit(3);
    }
    // Non-interactive: for each chunk, print and/or copy full XML snippet
    let total_chunks = chunks.len();
    for i in 0..total_chunks {
        let snippet = output::format_chunk_snippet(&chunks, i);
        if config.stdout {
            print!("{snippet}");
        }
        if copy_idx == i as isize && !config.no_clipboard {
            clipboard::copy_to_clipboard(&snippet, false, !config.stdout)?;
        }
    }
    // 8) Summary
    let total_token_count = count_tokens(&xml_output);
    println!(
        "OK {} files • {} tokens • {} chunks • copied={}",
        file_data.len(),
        total_token_count,
        chunks.len(),
        if copy_idx >= 0 {
            copy_idx.to_string()
        } else {
            "none".into()
        }
    );
    if config.no_clipboard && !config.stdout {
        info!("Note: neither --stdout nor clipboard copy requested; nothing visible.");
    }

    // 9) Warn if token count exceeds model context limit
    if let Some(limit) = config.model_context
        && total_token_count > limit
    {
        warn!(
            "token count {} exceeds model context limit {}",
            total_token_count, limit
        );
    }

    Ok(())
}
