mod cli;
mod clipboard;
mod gather;
mod interactive;
mod xml_output;

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use cli::Cli;

fn main() -> Result<()> {
    let cli = Cli::parse();

    // 1) Expand user-specified paths (globs, etc.)
    let user_paths_raw = gather::expand_paths(cli.paths)?;

    // Helper: check if `candidate` is "under" any user-specified path (including
    // exact matches).
    fn is_preselected(candidate: &PathBuf, user_paths: &[PathBuf]) -> bool {
        // Attempt to canonicalize the candidate; skip if it fails
        let cand_canon = match candidate.canonicalize() {
            Ok(c) => c,
            Err(_) => return false,
        };

        // If any user path is a parent of `cand_canon` or exact match => true
        for up in user_paths {
            // Canonicalize user path
            if let Ok(up_canon) = up.canonicalize() {
                // starts_with() means `cand_canon` is inside or equal to `up_canon`
                if cand_canon.starts_with(&up_canon) {
                    return true;
                }
            }
        }
        false
    }

    // 2) Determine the "root" directory to open in TUI If exactly one path is a
    //    directory, use that as root. Otherwise, default to "."
    let root = if user_paths_raw.len() == 1 && user_paths_raw[0].is_dir() {
        user_paths_raw[0].clone()
    } else {
        std::path::PathBuf::from(".")
    };

    // 3) Gather all files in that root folder
    let mut candidate_files = gather::gather_all_file_paths(&[root])?;

    // 4) Among those gathered, preselect anything "under" or exactly matching
    //    user-specified paths. This uses the helper `is_preselected`.
    let preselected_paths: Vec<PathBuf> = candidate_files
        .iter()
        .filter(|cand| is_preselected(cand, &user_paths_raw))
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
