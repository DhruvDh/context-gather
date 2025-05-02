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

    /// If set, opens the TUI for interactive selection.
    #[arg(short = 'i', long = "interactive")]
    pub interactive: bool,

    /// Do not copy to clipboard.
    #[arg(short = 'n', long = "no-clipboard")]
    pub no_clipboard: bool,

    /// Print XML output to stdout.
    #[arg(short = 'o', long = "stdout")]
    pub stdout: bool,

    /// Maximum file size in bytes before skipping files.
    #[arg(short = 's', long = "max-size", default_value_t = 1048576)]
    pub max_size: u64,

    /// Glob patterns to exclude files from processing.
    #[arg(short = 'x', long = "exclude")]
    pub exclude: Vec<String>,

    /// Maximum token count for model context; warn if exceeded.
    #[arg(short = 'L', long = "model-context")]
    pub model_context: Option<usize>,

    /// Split the context into chunks no larger than this many tokens. Default = 0 means no chunking.
    #[arg(short = 'c', long = "chunk-size", default_value_t = 0)]
    pub chunk_size: usize,

    /// Which chunk to copy (0-based); -1 means none.
    #[arg(short = 'k', long = "chunk-index", default_value_t = -1)]
    pub chunk_index: isize,

    /// Insert <more/> markers so an LLM knows additional chunks will follow.
    #[arg(short = 'm', long = "emit-markers")]
    pub emit_markers: bool,
}
