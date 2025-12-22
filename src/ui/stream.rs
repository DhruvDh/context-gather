use crate::chunker::Chunk;
use crate::config::Config;
use crate::context::types::FileContents;
use crate::context::xml::{maybe_escape_attr, maybe_escape_text};
use crate::io::clipboard;
use crate::output;
use anyhow::Result;
use globset::{Glob, GlobSetBuilder};
use path_slash::PathBufExt;
use std::io::{self, Write};

/// Multi-step mode: initial header then REPL for fetching files by id or glob.
pub fn multi_step_mode(
    chunks: &[Chunk],
    file_data: &[FileContents],
    config: &Config,
) -> Result<()> {
    // Header snippet without closing </shared-context>
    let snippet = chunks.first().map(|c| c.xml.as_str()).unwrap_or("");
    // Output the header snippet if requested
    if config.stdout {
        print!("{}", snippet);
    }
    if !config.no_clipboard {
        clipboard::copy_to_clipboard(snippet, false, false)?;
    }
    // Display REPL instructions
    eprintln!("Commands: enter file ids, file paths, or glob patterns; type 'q' to quit.");

    // REPL for on-demand file requests
    loop {
        {
            let mut ui = io::stderr();
            write!(ui, "Request file id or glob (or 'q' to quit): ")?;
            ui.flush()?;
        }
        let mut cmd = String::new();
        io::stdin().read_line(&mut cmd)?;
        let cmd = cmd.trim();
        if cmd.eq_ignore_ascii_case("q") {
            break;
        }
        // Determine selection: numeric ID or glob
        let mut selected = Vec::new();
        if let Ok(id) = cmd.parse::<usize>() {
            if id < file_data.len() {
                selected.push(id);
            } else {
                eprintln!("Invalid file id: {}", id);
                continue;
            }
        } else if let Ok(glob) = Glob::new(&cmd.replace('\\', "/")) {
            let mut builder = GlobSetBuilder::new();
            builder.add(glob);
            let matcher = builder.build().unwrap();
            for (i, fc) in file_data.iter().enumerate() {
                if matcher.is_match(fc.path.to_slash_lossy().as_ref()) {
                    selected.push(i);
                }
            }
            if selected.is_empty() {
                eprintln!("No files match pattern: {}", cmd);
                continue;
            }
        } else {
            eprintln!("Invalid request: {}", cmd);
            continue;
        }
        // Output each requested file
        for &id in &selected {
            let fc = &file_data[id];
            let path = fc.path.to_slash_lossy().to_string();
            let name = fc
                .path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let path_attr = maybe_escape_attr(&path, config.escape_xml);
            let name_attr = maybe_escape_attr(&name, config.escape_xml);
            let contents = maybe_escape_text(&fc.contents, config.escape_xml);
            let out = format!(
                "<file-contents id=\"{id}\" path=\"{path}\" name=\"{name}\">\n{contents}\n</file-contents>\n",
                id = id,
                path = path_attr,
                name = name_attr,
                contents = contents
            );
            if config.stdout {
                print!("{}", out);
            }
            if !config.no_clipboard {
                clipboard::copy_to_clipboard(&out, false, !config.stdout)?;
                eprintln!("Copied file id {}", id);
            }
        }
    }
    Ok(())
}

/// Interactive streaming mode: REPL for browsing and copying context chunks.
pub fn streaming_mode(
    chunks: &[Chunk],
    config: &Config,
) -> Result<()> {
    let total = chunks.len();
    let mut idx = 0usize;
    eprintln!("▲ Streaming {total} chunks (0..{}).", total - 1);
    // Display REPL instructions
    eprintln!("Commands: press Enter for next chunk, number to jump, or 'q' to quit.");
    loop {
        let snippet = output::format_chunk_snippet(chunks, idx);
        if config.stdout {
            print!("{}", snippet);
        }
        if !config.no_clipboard {
            clipboard::copy_to_clipboard(&snippet, false, !config.stdout)?;
            eprintln!("Copied chunk {idx}");
        }
        {
            let mut ui = io::stderr();
            write!(ui, "Enter chunk # (0..{}) or 'q' to quit: ", total - 1)?;
            ui.flush()?;
        }
        let mut cmd = String::new();
        io::stdin().read_line(&mut cmd)?;
        let cmd = cmd.trim();
        if cmd.eq_ignore_ascii_case("q") {
            break;
        }
        idx = if cmd.is_empty() {
            (idx + 1) % total
        } else {
            cmd.parse().unwrap_or(idx)
        };
    }
    Ok(())
}
