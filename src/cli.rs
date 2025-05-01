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
    #[arg(long)]
    pub no_clipboard: bool,

    /// Print XML output to stdout.
    #[arg(long)]
    pub stdout: bool,

    /// Maximum file size in bytes before skipping files.
    #[arg(long, default_value_t = 1048576)]
    pub max_size: u64,

    /// Glob patterns to exclude files from processing.
    #[arg(long)]
    pub exclude: Vec<String>,

    /// Maximum token count for model context; warn if exceeded.
    #[arg(long)]
    pub model_context: Option<usize>,
}
