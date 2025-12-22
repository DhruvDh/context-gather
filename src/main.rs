// Import modules from the library crate
use context_gather::config::Config;
use context_gather::io::clipboard;
use context_gather::output;
use context_gather::pipeline::{InvalidExcludePatterns, Pipeline};
use context_gather::ui::select_files_tui;
use context_gather::ui::stream::{multi_step_mode, streaming_mode};
use context_gather::{gather, tokenizer};

use anyhow::Result;
use tracing::{error, warn};
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    // Initialize tracing for structured logging, with RUST_LOG support
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();

    let config = Config::from_cli()?;
    tokenizer::init(config.tokenizer_model.as_deref())?;

    // Pre-validate CLI arg combos: chunk-index requires chunk-size > 0
    if matches!(config.chunk_size, Some(0)) {
        error!("--chunk-size must be > 0 (omit it to disable chunking)");
        std::process::exit(2);
    }
    let chunk_limit = config.chunk_size.unwrap_or(0);

    // 1) Expand user-specified paths (globs, etc.) and build candidates
    let mut pipeline = Pipeline::new();
    pipeline.expand_paths(&config.paths)?;
    pipeline.build_candidates()?;

    // 2) Exclude patterns: abort if all provided globs are invalid
    if let Err(err) = pipeline.apply_excludes(&config.exclude) {
        if let Some(invalid) = err.downcast_ref::<InvalidExcludePatterns>() {
            error!(
                "Every --exclude pattern was invalid: {:?}",
                invalid.patterns
            );
            std::process::exit(2);
        }
        return Err(err);
    }

    pipeline.compute_preselected();

    // 3) If interactive, open the TUI
    if config.interactive {
        let selected = match select_files_tui(
            pipeline.candidate_files().to_vec(),
            pipeline.preselected_paths(),
        ) {
            Ok(selected) => selected,
            Err(e) => {
                error!("Error in interactive TUI: {}", e);
                std::process::exit(1);
            }
        };
        pipeline.set_candidate_files(selected);
    }

    // 4) Read file data
    pipeline.collect_file_data(config.max_size)?;

    // 5) Build outputs
    let needs_chunks = config.multi_step || chunk_limit > 0;
    if needs_chunks {
        pipeline.build_chunks_with_header(
            chunk_limit,
            config.escape_xml,
            config.multi_step,
            config.git_info,
        )?;
    } else {
        pipeline.build_xml(config.escape_xml)?;
    }

    // Multi-step mode: REPL for fetching files on demand
    if config.multi_step {
        multi_step_mode(pipeline.chunks(), pipeline.file_data(), &config)?;
        return Ok(());
    }

    // Chunked mode interactive REPL: only when interactive flag is set
    if chunk_limit > 0 && config.interactive {
        streaming_mode(pipeline.chunks(), &config)?;
        return Ok(());
    }

    // If chunking disabled (-c 0), output full XML as a single chunk
    if chunk_limit == 0 {
        let xml_output = pipeline
            .xml_output()
            .expect("xml output should be built when chunking is disabled");
        // Print XML on stdout if requested
        if config.stdout {
            println!("{xml_output}");
        }
        // Copy to clipboard
        if !config.no_clipboard {
            clipboard::copy_to_clipboard(xml_output, false, !config.stdout)?;
        }
        // Summary: one chunk (index 0)
        let token_count = if config.model_context.is_some() {
            Some(gather::count_tokens(xml_output))
        } else {
            None
        };
        let summary = if config.stdout && config.no_clipboard && token_count.is_none() {
            // Skip tokenisation for pure stdout + no-clipboard runs when no model_context
            format!(
                "OK {} files • 1 chunk • copied={}",
                pipeline.file_data().len(),
                if !config.no_clipboard { "0" } else { "none" }
            )
        } else {
            format!(
                "OK {} files • {} tokens • 1 chunk • copied={}",
                pipeline.file_data().len(),
                token_count.unwrap_or_else(|| gather::count_tokens(xml_output)),
                if !config.no_clipboard { "0" } else { "none" }
            )
        };
        eprintln!("{summary}");
        if let (Some(limit), Some(total_token_count)) = (config.model_context, token_count)
            && total_token_count > limit
        {
            warn!(
                "token count {} exceeds model context limit {}",
                total_token_count, limit
            );
        }
        return Ok(());
    }

    // Determine default copy index: default to first chunk when unset and clipboard enabled
    let chunks = pipeline.chunks();
    if let Some(idx) = config.chunk_index
        && idx >= chunks.len()
    {
        warn!(
            "--chunk-index {} out of range (0..{})",
            idx,
            chunks.len().saturating_sub(1)
        );
        std::process::exit(3);
    }
    let copy_idx = if config.no_clipboard {
        None
    } else {
        config.chunk_index.or(Some(0))
    };
    // Non-interactive: handle selected chunk or print all
    // Non-interactive: for each chunk, print and/or copy full XML snippet
    let total_chunks = chunks.len();
    for i in 0..total_chunks {
        let snippet = output::format_chunk_snippet(chunks, i);
        if config.stdout {
            print!("{snippet}");
        }
        if copy_idx == Some(i) && !config.no_clipboard {
            clipboard::copy_to_clipboard(&snippet, false, !config.stdout)?;
        }
    }
    // 8) Summary
    let total_token_count: usize = chunks.iter().map(|c| c.tokens).sum();
    if config.model_context.is_some() {
        eprintln!(
            "OK {} files • {} tokens • {} chunks • copied={}",
            pipeline.file_data().len(),
            total_token_count,
            total_chunks,
            copy_idx
                .map(|idx| idx.to_string())
                .unwrap_or_else(|| "none".into())
        );
    } else {
        eprintln!(
            "OK {} files • {} chunks • copied={}",
            pipeline.file_data().len(),
            total_chunks,
            copy_idx
                .map(|idx| idx.to_string())
                .unwrap_or_else(|| "none".into())
        );
    }
    if config.no_clipboard && !config.stdout {
        eprintln!("Note: neither --stdout nor clipboard copy requested; nothing visible.");
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
