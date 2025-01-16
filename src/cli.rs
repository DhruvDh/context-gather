use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "ctx-gather")]
#[command(
    about = "Gather text file contents, group them by folder, output as XML to clipboard, then \
             show token count."
)]
pub struct Cli {
    /// File paths (supporting globs), defaults to "."
    #[arg(default_value = ".", num_args(1..))]
    pub paths: Vec<String>,

    /// If set, opens the TUI for interactive selection.
    #[arg(short = 'i', long = "interactive")]
    pub interactive: bool,
}
