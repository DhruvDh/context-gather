// Smart chunk builder: structure-aware, token-bounded
use crate::context::types::FileContents;
use crate::context::xml::{maybe_escape_attr, maybe_escape_text};
use crate::tokenizer::count as count_tokens;
use path_slash::PathExt;
use std::path::{Path, PathBuf};
use tracing::warn;

/// Metadata for each file in the context header
pub struct FileMeta {
    pub id: usize,
    pub path: PathBuf,
    pub tokens: usize,
    pub parts: usize,
}

/// Represents one chunk body (file-contents blocks only; wrappers are added later).
pub struct Chunk {
    pub index: usize,
    pub xml: String,
    pub tokens: usize,
}

/// Represents one file block inside a chunk
pub struct FileBlock {
    pub xml: String,
    pub tokens: usize,
}

/// Represents a chunk body prior to wrapper rendering
pub struct ChunkBody {
    pub blocks: Vec<FileBlock>,
    pub tokens: usize,
}

/// Build metadata for files without chunking or splitting.
pub fn build_file_meta(
    files: &[FileContents],
    escape_xml: bool,
) -> Vec<FileMeta> {
    files
        .iter()
        .enumerate()
        .map(|(file_id, file)| {
            let contents = maybe_escape_text(&file.contents, escape_xml);
            let content_tokens = count_tokens(contents.as_ref());
            FileMeta {
                id: file_id,
                path: file.path.clone(),
                tokens: content_tokens,
                parts: 1,
            }
        })
        .collect()
}

fn wrap_file(
    path: &Path,
    body: &str,
    escape_xml: bool,
) -> String {
    let filename = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let path_str = path.to_slash_lossy().to_string();
    let folder = path
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .to_slash_lossy()
        .to_string();
    let folder_display = if folder.is_empty() {
        ".".to_string()
    } else {
        folder
    };
    let path_attr = maybe_escape_attr(&path_str, escape_xml);
    let filename_attr = maybe_escape_attr(&filename, escape_xml);
    let folder_attr = maybe_escape_attr(&folder_display, escape_xml);
    format!(
        "    <file-contents path=\"{}\" name=\"{}\" folder=\"{}\">\n{}\n    </file-contents>\n",
        path_attr, filename_attr, folder_attr, body
    )
}

// Wrap a sub-part of a file into its own XML block
fn wrap_part(
    path: &Path,
    idx: usize,
    total: usize,
    body: &str,
    escape_xml: bool,
) -> String {
    let filename = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let path_str = path.to_slash_lossy().to_string();
    let folder = path
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .to_slash_lossy()
        .to_string();
    let folder_display = if folder.is_empty() {
        ".".to_string()
    } else {
        folder
    };
    let path_attr = maybe_escape_attr(&path_str, escape_xml);
    let filename_attr = maybe_escape_attr(&filename, escape_xml);
    let folder_attr = maybe_escape_attr(&folder_display, escape_xml);
    format!(
        "    <file-contents path=\"{}\" name=\"{}\" folder=\"{}\" part=\"{}/{}\">\n{}    </file-contents>\n",
        path_attr, filename_attr, folder_attr, idx, total, body
    )
}

fn split_with_total(
    lines: &[String],
    path: &Path,
    max_tokens: usize,
    escape_xml: bool,
    total_parts: usize,
) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut part_idx = 1usize;

    for line in lines {
        if current.is_empty() {
            current.push_str(line);
            let wrapped = wrap_part(path, part_idx, total_parts, &current, escape_xml);
            let wrapped_tokens = count_tokens(&wrapped);
            if wrapped_tokens > max_tokens {
                warn!(
                    "line in {:?} exceeds chunk size {}; emitting oversize part",
                    path, max_tokens
                );
                parts.push(std::mem::take(&mut current));
                part_idx += 1;
            }
            continue;
        }

        let prev_len = current.len();
        current.push_str(line);
        let wrapped = wrap_part(path, part_idx, total_parts, &current, escape_xml);
        let wrapped_tokens = count_tokens(&wrapped);
        if wrapped_tokens > max_tokens {
            current.truncate(prev_len);
            parts.push(std::mem::take(&mut current));
            part_idx += 1;

            current.push_str(line);
            let wrapped = wrap_part(path, part_idx, total_parts, &current, escape_xml);
            let wrapped_tokens = count_tokens(&wrapped);
            if wrapped_tokens > max_tokens {
                warn!(
                    "line in {:?} exceeds chunk size {}; emitting oversize part",
                    path, max_tokens
                );
                parts.push(std::mem::take(&mut current));
                part_idx += 1;
            }
        }
    }

    if !current.is_empty() {
        parts.push(current);
    }

    parts
}

fn split_file_into_parts(
    contents: &str,
    path: &Path,
    max_tokens: usize,
    escape_xml: bool,
) -> Vec<String> {
    let lines: Vec<String> = contents
        .split('\n')
        .map(|line| format!("{line}\n"))
        .collect();
    let mut target_parts = 1usize;
    let mut parts = Vec::new();
    for _ in 0..16 {
        parts = split_with_total(&lines, path, max_tokens, escape_xml, target_parts);
        let actual = parts.len().max(1);
        if actual == target_parts {
            return parts;
        }
        target_parts = actual;
    }
    parts
}

/// Builds chunk bodies and metadata for header
/// Splits between file-contents blocks, and splits oversize files
pub fn build_chunk_bodies(
    files: &[FileContents],
    max_tokens: usize,
    escape_xml: bool,
) -> (Vec<ChunkBody>, Vec<FileMeta>) {
    let mut metas = Vec::<FileMeta>::new();
    let mut blocks = Vec::<FileBlock>::new();

    for (file_id, file) in files.iter().enumerate() {
        let contents = maybe_escape_text(&file.contents, escape_xml);
        let contents_str = contents.as_ref();
        let content_tokens = count_tokens(contents_str);
        let file_block = wrap_file(&file.path, contents_str, escape_xml);
        let block_tokens = count_tokens(&file_block);

        if max_tokens == 0 || block_tokens <= max_tokens {
            blocks.push(FileBlock {
                xml: file_block,
                tokens: block_tokens,
            });
            metas.push(FileMeta {
                id: file_id,
                path: file.path.clone(),
                tokens: content_tokens,
                parts: 1,
            });
            continue;
        }

        let parts = split_file_into_parts(contents_str, &file.path, max_tokens, escape_xml);
        let parts_count = parts.len().max(1);
        for (idx, body) in parts.iter().enumerate() {
            let wrapped = wrap_part(&file.path, idx + 1, parts_count, body, escape_xml);
            let wrapped_tokens = count_tokens(&wrapped);
            if wrapped_tokens > max_tokens {
                warn!(
                    "file {:?} part {} exceeds chunk size {}; emitting oversize part",
                    file.path,
                    idx + 1,
                    max_tokens
                );
            }
            blocks.push(FileBlock {
                xml: wrapped,
                tokens: wrapped_tokens,
            });
        }
        metas.push(FileMeta {
            id: file_id,
            path: file.path.clone(),
            tokens: content_tokens,
            parts: parts_count,
        });
    }

    let mut chunks = Vec::<ChunkBody>::new();
    let mut current = ChunkBody {
        blocks: Vec::new(),
        tokens: 0,
    };
    for block in blocks {
        if max_tokens > 0
            && !current.blocks.is_empty()
            && current.tokens + block.tokens > max_tokens
        {
            chunks.push(current);
            current = ChunkBody {
                blocks: Vec::new(),
                tokens: 0,
            };
        }
        current.tokens += block.tokens;
        current.blocks.push(block);
    }
    if !current.blocks.is_empty() {
        chunks.push(current);
    }

    (chunks, metas)
}

/// Builds smart chunks and metadata for header
/// Splits between file-contents blocks, and splits oversize files
pub fn build_chunks(
    files: &[FileContents],
    max_tokens: usize,
    escape_xml: bool,
) -> (Vec<Chunk>, Vec<FileMeta>) {
    let (bodies, metas) = build_chunk_bodies(files, max_tokens, escape_xml);
    let chunks = bodies
        .into_iter()
        .enumerate()
        .map(|(idx, body)| Chunk {
            index: idx,
            xml: body.blocks.into_iter().map(|b| b.xml).collect(),
            tokens: body.tokens,
        })
        .collect();
    (chunks, metas)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::types::FileContents;
    use crate::tokenizer::count as count_tokens;
    use std::path::PathBuf;

    #[test]
    fn header_token_math() {
        // Create dummy file data with known contents
        let files = vec![FileContents {
            folder: PathBuf::new(),
            path: PathBuf::from("dummy.txt"),
            contents: "hello world\n".repeat(10),
        }];
        // Build chunks with generous limit
        let (chunks, metas) = build_chunks(&files, 1000, false);
        // Concatenate all chunk XML
        let xml_all: String = chunks.iter().map(|c| c.xml.clone()).collect();
        let total_tokens = count_tokens(&xml_all);
        let sum_meta: usize = metas.iter().map(|m| m.tokens).sum();
        assert!(
            sum_meta <= total_tokens,
            "Sum of file.tokens should be <= total tokens ({sum_meta} <= {total_tokens})"
        );
    }
}
