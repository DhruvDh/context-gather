mod cli;
mod clipboard;
mod gather;
mod interactive;
mod xml_output;

use anyhow::Result;
use clap::Parser;
use cli::Cli;

fn main() -> Result<()> {
    let cli = Cli::parse();

    let all_paths = gather::expand_paths(cli.paths)?;

    // If exactly one path is a directory -> open it alone (no preselect).
    // Otherwise -> gather from all input paths and preselect them.
    let (roots, preselected_paths) = if all_paths.len() == 1 && all_paths[0].is_dir() {
        (all_paths.clone(), vec![])
    } else {
        (all_paths.clone(), all_paths.clone())
    };

    // Gather all files from the chosen roots
    let mut candidate_files = gather::gather_all_file_paths(&roots)?;

    // If interactive, let user select from those files
    if cli.interactive {
        candidate_files = interactive::select_files_tui(candidate_files, &preselected_paths)?;
    }

    // 3. Only now read file data
    let file_data = gather::collect_file_data(&candidate_files)?;
    let xml_output = xml_output::build_xml(&file_data);
    clipboard::copy_to_clipboard(&xml_output)?;
    gather::count_tokens(&xml_output)?;

    Ok(())
}
