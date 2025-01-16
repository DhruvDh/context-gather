use std::path::PathBuf;
use anyhow::Result;

pub fn select_files_tui(paths: Vec<PathBuf>) -> Result<Vec<PathBuf>> {
    // TODO: Implement TUI with crossterm/tui
    // For now, return all paths as selected
    Ok(paths)
}
