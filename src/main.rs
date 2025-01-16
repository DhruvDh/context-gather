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

    // 1) Expand user-specified paths (globs, etc.)
    let user_paths = gather::expand_paths(cli.paths)?;

    // 2) Determine the "root" directory to open in TUI
    //    If exactly one path is a directory, use that as root.
    //    Otherwise, default to "."
    let root = if user_paths.len() == 1 && user_paths[0].is_dir() {
        user_paths[0].clone()
    } else {
        std::path::PathBuf::from(".")
    };

    // 3) Gather all files in that root folder
    let mut candidate_files = gather::gather_all_file_paths(&[root])?;

    // 4) Among those gathered, preselect only items that match user_paths
    //    (i.e., if user specified certain files/folders, they start checked)
    let preselected_paths: Vec<_> = candidate_files
        .iter()
        .filter(|p| user_paths.contains(p))
        .cloned()
        .collect();

    // 5) If interactive, open the TUI
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
