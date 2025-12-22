use crate::cli::Cli;
use crate::constants::DEFAULT_MODEL_CONTEXT;
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
    pub model_context: Option<usize>,
    pub tokenizer_model: Option<String>,
    pub chunk_size: Option<usize>,
    pub chunk_index: Option<usize>,
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
        let chunk_index = match cli.chunk_index {
            None => None,
            Some(-1) => None,
            Some(idx) => Some(idx as usize),
        };
        let model_context = if cli.no_model_context {
            None
        } else {
            Some(cli.model_context.unwrap_or(DEFAULT_MODEL_CONTEXT))
        };
        Ok(Config {
            paths,
            interactive: cli.interactive,
            no_clipboard: cli.no_clipboard,
            stdout: cli.stdout,
            max_size: cli.max_size,
            exclude: cli.exclude,
            model_context,
            tokenizer_model: cli.tokenizer_model,
            chunk_size: cli.chunk_size,
            chunk_index,
            multi_step: cli.multi_step,
            git_info: cli.git_info,
            escape_xml: cli.escape_xml,
        })
    }
}
