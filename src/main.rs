// Import modules from the library crate
use context_gather::config::{ChunkCopy, Config};
use context_gather::io::clipboard;
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

    // 3) If selection UI requested, open the TUI
    if config.select {
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
        multi_step_mode(pipeline.rendered_chunks(), pipeline.file_data(), &config)?;
        return Ok(());
    }

    // Chunked mode interactive REPL: only when streaming is requested
    if chunk_limit > 0 && config.stream {
        streaming_mode(pipeline.rendered_chunks(), &config)?;
        return Ok(());
    }

    // If chunking disabled (no --chunk-size), output full XML as a single chunk
    if chunk_limit == 0 {
        let xml_output = pipeline
            .xml_output()
            .expect("xml output should be built when chunking is disabled");
        // Print XML on stdout if requested
        if config.stdout {
            print!("{xml_output}");
        }
        // Copy to clipboard
        let mut copied_idx: Option<usize> = None;
        if !config.no_clipboard {
            let copied = clipboard::copy_to_clipboard(xml_output, !config.stdout)?;
            if copied {
                copied_idx = Some(0);
            }
        }
        // Summary: one chunk (index 0)
        let token_count = config
            .model_context
            .map(|_| gather::count_tokens(xml_output));
        let summary = match token_count {
            Some(tokens) => format!(
                "OK {} files • {} tokens • 1 chunk • copied={}",
                pipeline.file_data().len(),
                tokens,
                copied_idx
                    .map(|idx| idx.to_string())
                    .unwrap_or_else(|| "none".into())
            ),
            None => format!(
                "OK {} files • 1 chunk • copied={}",
                pipeline.file_data().len(),
                copied_idx
                    .map(|idx| idx.to_string())
                    .unwrap_or_else(|| "none".into())
            ),
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
    let chunks = pipeline.rendered_chunks();
    let total_chunks = chunks.len();
    if let ChunkCopy::Index(idx) = config.chunk_copy
        && idx >= total_chunks
    {
        warn!(
            "--chunk-index {} out of range (0..{})",
            idx,
            total_chunks.saturating_sub(1)
        );
        std::process::exit(3);
    }
    let copy_idx = if config.no_clipboard {
        None
    } else {
        match config.chunk_copy {
            ChunkCopy::Default => Some(0),
            ChunkCopy::Index(idx) => Some(idx),
            ChunkCopy::None => None,
        }
    };
    let mut copied_idx: Option<usize> = None;
    // Non-interactive: print/copy requested chunk(s)
    if config.stdout {
        match config.chunk_copy {
            ChunkCopy::Default => {
                for (i, chunk) in chunks.iter().take(total_chunks).enumerate() {
                    let snippet = chunk.xml.as_str();
                    print!("{snippet}");
                    if copy_idx == Some(i) && !config.no_clipboard {
                        let copied = clipboard::copy_to_clipboard(snippet, !config.stdout)?;
                        if copied {
                            copied_idx = Some(i);
                        }
                    }
                }
            }
            ChunkCopy::Index(idx) => {
                let snippet = chunks[idx].xml.as_str();
                print!("{snippet}");
                if copy_idx == Some(idx) && !config.no_clipboard {
                    let copied = clipboard::copy_to_clipboard(snippet, !config.stdout)?;
                    if copied {
                        copied_idx = Some(idx);
                    }
                }
            }
            ChunkCopy::None => {
                if let Some(idx) = copy_idx {
                    let snippet = chunks[idx].xml.as_str();
                    let copied = clipboard::copy_to_clipboard(snippet, !config.stdout)?;
                    if copied {
                        copied_idx = Some(idx);
                    }
                }
            }
        }
    } else if let Some(idx) = copy_idx {
        let snippet = chunks[idx].xml.as_str();
        let copied = clipboard::copy_to_clipboard(snippet, !config.stdout)?;
        if copied {
            copied_idx = Some(idx);
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
            copied_idx
                .map(|idx| idx.to_string())
                .unwrap_or_else(|| "none".into())
        );
    } else {
        eprintln!(
            "OK {} files • {} chunks • copied={}",
            pipeline.file_data().len(),
            total_chunks,
            copied_idx
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
