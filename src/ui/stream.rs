use crate::chunker::Chunk;
use crate::config::Config;
use crate::context::types::FileContents;
use crate::io::clipboard;
use anyhow::Result;
use glob::Pattern;
use path_slash::PathBufExt;
use std::io::{self, Write};

/// Multi-step mode: initial header then REPL for fetching files by id or glob.
pub fn multi_step_mode(
    chunks: &[Chunk],
    file_data: &[FileContents],
    config: &Config,
) -> Result<()> {
    let total = chunks.len();
    let rem = total.saturating_sub(1);
    // Header snippet without closing </shared-context>
    let mut snippet = chunks[0].xml.clone();
    if rem > 0 {
        snippet.push_str(&format!("<more remaining=\"{}\"/>\n", rem));
    }
    // Always output the header snippet
    print!("{}", snippet);
    if !config.no_clipboard {
        clipboard::copy_to_clipboard(&snippet, false)?;
    }

    // REPL for on-demand file requests
    loop {
        print!("Request file id or glob (or 'q' to quit): ");
        io::stdout().flush()?;
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
        } else if let Ok(pat) = Pattern::new(cmd) {
            for (i, fc) in file_data.iter().enumerate() {
                if pat.matches(fc.path.to_slash_lossy().as_ref()) {
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
            let path = fc.path.to_slash_lossy();
            let name = fc
                .path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let out = format!(
                "<file-contents id=\"{id}\" path=\"{path}\" name=\"{name}\">\n{contents}\n</file-contents>\n",
                id = id,
                path = path,
                name = name,
                contents = fc.contents
            );
            if config.stdout {
                print!("{}", out);
            }
            if !config.no_clipboard {
                clipboard::copy_to_clipboard(&out, false)?;
                println!("Copied file id {}", id);
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
    println!("▲ Streaming {total} chunks (0..{}).", total - 1);
    loop {
        let rem = total - idx - 1;
        let snippet = if idx == 0 {
            let mut s = chunks[0].xml.clone();
            if rem > 0 {
                s.push_str(&format!("<more remaining=\"{}\"/>\n", rem));
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
        if config.stdout {
            print!("{}", snippet);
        }
        if !config.no_clipboard {
            clipboard::copy_to_clipboard(&snippet, false)?;
            println!("✔ copied chunk {idx}");
        }
        print!("Enter chunk # (0..{}) or 'q' to quit: ", total - 1);
        io::stdout().flush()?;
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
