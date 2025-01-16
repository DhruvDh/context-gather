mod cli;
mod interactive;
mod gather;
mod xml_output;
mod clipboard;

use cli::Cli;
use clap::Parser;
use anyhow::Result;

fn main() -> Result<()> {
    let cli = Cli::parse();

    let mut all_paths = gather::expand_paths(cli.paths)?;
    if cli.interactive {
        all_paths = interactive::select_files_tui(all_paths)?;
    }

    let file_data = gather::collect_file_data(&all_paths)?;
    let xml_output = xml_output::build_xml(&file_data);
    clipboard::copy_to_clipboard(&xml_output)?;
    gather::count_tokens(&xml_output)?;

    Ok(())
}
