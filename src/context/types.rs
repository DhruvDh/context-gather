use std::path::PathBuf;

/// Contents of a file with its folder and path metadata
#[derive(Debug, Clone)]
pub struct FileContents {
    pub folder: PathBuf,
    pub path: PathBuf,
    pub contents: String,
}
