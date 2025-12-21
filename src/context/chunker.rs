// Smart chunk builder: structure-aware, token-bounded
use crate::context::types::FileContents;
use crate::context::xml::{maybe_escape_attr, maybe_escape_text};
use crate::tokenizer::count as count_tokens;
use std::path::{Path, PathBuf};

/// Metadata for each file in the context header
pub struct FileMeta {
    pub id: usize,
    pub path: PathBuf,
    pub tokens: usize,
    pub parts: usize,
}

/// Represents one chunk of XML-ish output
pub struct Chunk {
    pub index: usize,
    pub xml: String,
    pub tokens: usize,
}

fn split_oversize_parts(
    lines: &[&str],
    path: &Path,
    total_parts: usize,
    max_tokens: usize,
    escape_xml: bool,
) -> Vec<String> {
    let mut parts = Vec::new();
    let mut part_xml = String::new();
    let mut part_tok = 0usize;
    let mut part_idx = 1usize;
    let mut overhead = count_tokens(&wrap_part(path, part_idx, total_parts, "", escape_xml));
    for line in lines {
        let new_tok = count_tokens(line) + 1; // include newline
        if !part_xml.is_empty() && part_tok + new_tok + overhead > max_tokens {
            parts.push(std::mem::take(&mut part_xml));
            part_tok = 0;
            part_idx += 1;
            overhead = count_tokens(&wrap_part(path, part_idx, total_parts, "", escape_xml));
        }
        part_xml.push_str(line);
        part_xml.push('\n');
        part_tok += new_tok;
    }
    if !part_xml.is_empty() {
        parts.push(part_xml);
    }
    parts
}

/// Builds smart chunks and metadata for header
/// Splits between file-contents blocks, and splits oversize files
pub fn build_chunks(
    files: &[FileContents],
    max_tokens: usize,
    escape_xml: bool,
) -> (Vec<Chunk>, Vec<FileMeta>) {
    // If max_tokens is zero, do not split: generate one chunk with all files
    if max_tokens == 0 {
        let mut metas = Vec::new();
        let mut xml_all = String::new();
        for (file_id, file) in files.iter().enumerate() {
            let contents = maybe_escape_text(&file.contents, escape_xml);
            let path = file.path.display().to_string();
            let path_attr = maybe_escape_attr(&path, escape_xml);
            let name = file
                .path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let name_attr = maybe_escape_attr(&name, escape_xml);
            let file_block = format!(
                "    <file-contents path=\"{}\" name=\"{}\">\n{}\n    </file-contents>\n",
                path_attr, name_attr, contents,
            );
            let file_tok = count_tokens(&file_block);
            xml_all.push_str(&file_block);
            metas.push(FileMeta {
                id: file_id,
                path: file.path.clone(),
                tokens: file_tok,
                parts: 1,
            });
        }
        let total_toks = count_tokens(&xml_all);
        let chunk = Chunk {
            index: 0,
            xml: xml_all,
            tokens: total_toks,
        };
        return (vec![chunk], metas);
    }
    let mut chunks = Vec::<Chunk>::new();
    let mut metas = Vec::<FileMeta>::new();
    let mut current_xml = String::new();
    let mut current_toks = 0usize;
    let mut file_id = 0usize;

    // Helper to push a chunk if non-empty, resetting xml and toks
    let mut push_chunk = |xml: &mut String, toks: &mut usize| {
        if !xml.is_empty() {
            let idx = chunks.len();
            chunks.push(Chunk {
                index: idx,
                xml: std::mem::take(xml),
                tokens: *toks,
            });
            *toks = 0;
        }
    };

    for file in files {
        let contents = maybe_escape_text(&file.contents, escape_xml);
        let contents_str = contents.as_ref();
        let path = file.path.display().to_string();
        let path_attr = maybe_escape_attr(&path, escape_xml);
        let name = file
            .path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let name_attr = maybe_escape_attr(&name, escape_xml);
        let file_block = format!(
            "    <file-contents path=\"{}\" name=\"{}\">\n{}\n    </file-contents>\n",
            path_attr, name_attr, contents_str,
        );
        let file_tok = count_tokens(&file_block);

        // fits entirely in current chunk
        if current_toks + file_tok <= max_tokens {
            current_xml.push_str(&file_block);
            current_toks += file_tok;
            metas.push(FileMeta {
                id: file_id,
                path: file.path.clone(),
                tokens: file_tok,
                parts: 1,
            });
            file_id += 1;
            continue;
        }

        // start new chunk if file alone fits
        if file_tok <= max_tokens {
            push_chunk(&mut current_xml, &mut current_toks);
            current_xml.push_str(&file_block);
            current_toks = file_tok;
            metas.push(FileMeta {
                id: file_id,
                path: file.path.clone(),
                tokens: file_tok,
                parts: 1,
            });
            file_id += 1;
            continue;
        }

        // oversize file: split into parts by lines
        let lines: Vec<&str> = contents_str.split('\n').collect();
        let mut parts_target = 1usize;
        let parts = loop {
            let parts =
                split_oversize_parts(&lines, &file.path, parts_target, max_tokens, escape_xml);
            let actual = parts.len().max(1);
            if actual == parts_target {
                break parts;
            }
            parts_target = actual;
        };
        let mut total_file_tokens = 0usize;
        let parts_count = parts.len().max(1);
        for (idx, body) in parts.iter().enumerate() {
            let wrapped = wrap_part(&file.path, idx + 1, parts_count, body, escape_xml);
            push_chunk(&mut current_xml, &mut current_toks);
            let wrapped_tok = count_tokens(&wrapped);
            current_xml.push_str(&wrapped);
            current_toks = wrapped_tok;
            total_file_tokens += wrapped_tok;
        }
        metas.push(FileMeta {
            id: file_id,
            path: file.path.clone(),
            tokens: total_file_tokens,
            parts: parts_count,
        });
        file_id += 1;
    }

    // push final chunk
    push_chunk(&mut current_xml, &mut current_toks);
    (chunks, metas)
}

// Wrap a sub-part of a file into its own XML block
fn wrap_part(
    path: &Path,
    idx: usize,
    total: usize,
    body: &str,
    escape_xml: bool,
) -> String {
    // Safely extract filename, fallback to empty string
    let filename = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let path_str = path.display().to_string();
    let path_attr = maybe_escape_attr(&path_str, escape_xml);
    let filename_attr = maybe_escape_attr(&filename, escape_xml);
    format!(
        "    <file-contents path=\"{}\" name=\"{}\" part=\"{}/{}\">\n{}    </file-contents>\n",
        path_attr, filename_attr, idx, total, body
    )
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
