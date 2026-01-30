use crate::cli::Cli;
use crate::constants::DEFAULT_MODEL_CONTEXT;
use anyhow::Result;
use clap::Parser;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkCopy {
    /// User did not specify --chunk-index; use defaults.
    Default,
    /// User specified --chunk-index -1; copy/print none.
    None,
    /// User specified an explicit chunk index.
    Index(usize),
}

/// Application configuration derived from CLI arguments
#[derive(Debug, Clone)]
pub struct Config {
    pub paths: Vec<String>,
    pub interactive: bool,
    pub select: bool,
    pub stream: bool,
    pub no_clipboard: bool,
    pub stdout: bool,
    pub max_size: u64,
    pub exclude: Vec<String>,
    pub model_context: Option<usize>,
    pub tokenizer_model: Option<String>,
    pub chunk_size: Option<usize>,
    pub chunk_copy: ChunkCopy,
    /// Enable multi-step mode: copy only header initially and serve files on demand.
    pub multi_step: bool,
    pub git_info: bool,
    pub escape_xml: bool,
}

impl Config {
    /// Parse CLI arguments into a Config
    pub fn from_cli() -> Result<Self> {
        let cli = Cli::parse();
        let paths = cli.paths.clone();
        let chunk_copy = match cli.chunk_index {
            None => ChunkCopy::Default,
            Some(-1) => ChunkCopy::None,
            Some(idx) => ChunkCopy::Index(idx as usize),
        };
        let model_context = if cli.no_model_context {
            None
        } else {
            Some(cli.model_context.unwrap_or(DEFAULT_MODEL_CONTEXT))
        };
        let select = cli.select || cli.interactive;
        let stream = cli.stream || (cli.interactive && cli.chunk_size.is_some());
        let escape_xml = cli.escape_xml;
        Ok(Config {
            paths,
            interactive: cli.interactive,
            select,
            stream,
            no_clipboard: cli.no_clipboard,
            stdout: cli.stdout,
            max_size: cli.max_size,
            exclude: cli.exclude,
            model_context,
            tokenizer_model: cli.tokenizer_model,
            chunk_size: cli.chunk_size,
            chunk_copy,
            multi_step: cli.multi_step,
            git_info: cli.git_info,
            escape_xml,
        })
    }
}
