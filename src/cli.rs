use crate::constants::DEFAULT_MAX_FILE_SIZE;
use clap::Parser;

fn parse_chunk_index(s: &str) -> Result<isize, String> {
    let idx: isize = s.parse().map_err(|_| format!("invalid chunk index: {s}"))?;
    if idx >= -1 {
        Ok(idx)
    } else {
        Err("chunk index must be >= 0 or -1 (none)".to_string())
    }
}

#[derive(Parser, Debug)]
#[command(name = "context-gather")]
#[command(
    about = "Gather text file contents, group them by folder, output as XML to clipboard, \
                   then show token count."
)]
pub struct Cli {
    /// File paths (supporting globs), defaults to "."
    #[arg(default_value = ".", num_args(1..))]
    pub paths: Vec<String>,

    /// Open interactive TUI for file selection; with --chunk-size, also stream chunks (alias for --select + --stream).
    #[arg(short = 'i', long = "interactive", default_value_t = false)]
    pub interactive: bool,

    /// Open the file-selection TUI only.
    #[arg(long = "select", default_value_t = false)]
    pub select: bool,

    /// After chunking, open the chunk streaming REPL (requires --chunk-size).
    #[arg(
        long = "stream",
        default_value_t = false,
        requires = "chunk_size",
        conflicts_with = "multi_step",
        conflicts_with = "chunk_index"
    )]
    pub stream: bool,

    /// Do not copy to clipboard.
    #[arg(short = 'n', long = "no-clipboard", default_value_t = false)]
    pub no_clipboard: bool,

    /// Print XML output to stdout.
    #[arg(short = 'o', long = "stdout", default_value_t = false)]
    pub stdout: bool,

    /// Maximum file size in bytes before skipping files.
    #[arg(long = "max-size", default_value_t = DEFAULT_MAX_FILE_SIZE)]
    pub max_size: u64,

    /// Glob patterns to exclude files from processing.
    #[arg(long = "exclude-paths")]
    pub exclude: Vec<String>,

    /// Maximum token count for model context; warn if exceeded (default 200000).
    #[arg(long = "model-context")]
    pub model_context: Option<usize>,

    /// Disable model context warnings and token-count summary.
    #[arg(
        long = "no-model-context",
        default_value_t = false,
        conflicts_with = "model_context"
    )]
    pub no_model_context: bool,

    /// Tokenizer model name (defaults to GPT-5.2).
    #[arg(long = "tokenizer-model")]
    pub tokenizer_model: Option<String>,

    /// Split the context into chunks no larger than this many tokens (omit to disable chunking).
    #[arg(short = 'c', long = "chunk-size")]
    pub chunk_size: Option<usize>,

    /// Which chunk to copy/print (0-based); -1 means none.
    #[arg(
        short = 'k',
        long = "chunk-index",
        value_parser = parse_chunk_index,
        requires = "chunk_size",
        allow_hyphen_values = true
    )]
    pub chunk_index: Option<isize>,

    /// Enable multi-step mode: copy only header initially; then serve files on demand (use --select or -i for TUI).
    #[arg(short = 'm', long = "multi-step", conflicts_with = "chunk_size")]
    pub multi_step: bool,

    /// Include git metadata (branch, recent commits, diff) in the header.
    #[arg(long = "git-info", default_value_t = false)]
    pub git_info: bool,

    /// Escape XML special characters in content (default: off; attributes are always escaped when needed).
    #[arg(long = "escape-xml", default_value_t = false)]
    pub escape_xml: bool,
}
