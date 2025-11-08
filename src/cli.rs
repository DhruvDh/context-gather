use crate::constants::{DEFAULT_CHUNK_SIZE, DEFAULT_MAX_FILE_SIZE};
use clap::Parser;

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

    /// Open interactive TUI for file selection and chunk browsing (use with -c or -m for chunked/multi-step UIs).
    #[arg(short = 'i', long = "interactive", default_value_t = false)]
    pub interactive: bool,

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

    /// Maximum token count for model context; warn if exceeded.
    #[arg(long = "model-context", default_value = "200000")]
    pub model_context: Option<usize>,

    /// Split the context into chunks no larger than this many tokens (use with -i to browse chunks in TUI).
    #[arg(short = 'c', long = "chunk-size", default_value_t = DEFAULT_CHUNK_SIZE)]
    pub chunk_size: usize,

    /// Which chunk to copy (0-based); -1 means none.
    #[arg(short = 'k', long = "chunk-index", default_value_t = -1)]
    pub chunk_index: isize,

    /// Enable multi-step mode: copy only header initially; then serve files on demand (use -i for TUI file picker).
    #[arg(short = 'm', long = "multi-step")]
    pub multi_step: bool,
}
