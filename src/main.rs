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
    // 1. Turn globs & directories into a full list of files
    let mut candidate_files = gather::gather_all_file_paths(&all_paths)?;

    // 2. If interactive, let user select from those files
    if cli.interactive {
        candidate_files = interactive::select_files_tui(candidate_files)?;
    }

    // 3. Only now read file data
    let file_data = gather::collect_file_data(&candidate_files)?;
    let xml_output = xml_output::build_xml(&file_data);
    clipboard::copy_to_clipboard(&xml_output)?;
    gather::count_tokens(&xml_output)?;

    Ok(())
}
