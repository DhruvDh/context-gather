use crate::cli::Cli;
use anyhow::Result;
use clap::Parser;

/// Application configuration derived from CLI arguments
#[derive(Debug, Clone)]
pub struct Config {
    pub paths: Vec<String>,
    pub interactive: bool,
    pub no_clipboard: bool,
    pub stdout: bool,
    pub max_size: u64,
    pub exclude: Vec<String>,
    pub model_context: usize,
    pub chunk_size: usize,
    pub chunk_index: isize,
    pub emit_markers: bool,
    /// Enable multi-step mode: copy only header initially and serve files on demand.
    pub multi_step: bool,
}

impl Config {
    /// Parse CLI arguments into a Config
    pub fn from_cli() -> Result<Self> {
        let cli = Cli::parse();
        let paths = cli.paths.clone();
        Ok(Config {
            paths,
            interactive: cli.interactive,
            no_clipboard: cli.no_clipboard,
            stdout: cli.stdout,
            max_size: cli.max_size,
            exclude: cli.exclude,
            model_context: cli.model_context,
            chunk_size: cli.chunk_size,
            chunk_index: cli.chunk_index,
            emit_markers: cli.emit_markers,
            multi_step: cli.multi_step,
        })
    }
}
