// Smart chunk builder: structure-aware, token-bounded
use crate::gather::{FileContents, count_tokens};
use std::path::{Path, PathBuf};

/// Metadata for each file in the context header
pub struct FileMeta {
    pub id: usize,
    pub path: PathBuf,
    pub tokens: usize,
    pub parts: usize,
}

/// Represents one chunk of XML-ish output
#[allow(dead_code)]
pub struct Chunk {
    pub index: usize,
    pub xml: String,
    pub tokens: usize,
}

/// Builds smart chunks and metadata for header
/// Splits between file-contents blocks, and splits oversize files
pub fn build_chunks(
    files: &[FileContents],
    max_tokens: usize,
) -> (Vec<Chunk>, Vec<FileMeta>) {
    // If max_tokens is zero, do not split: generate one chunk with all files
    if max_tokens == 0 {
        let mut metas = Vec::new();
        let mut xml_all = String::new();
        for (file_id, file) in files.iter().enumerate() {
            let file_tok = count_tokens(&file.contents);
            let file_block = format!(
                "    <file-contents path=\"{}\" name=\"{}\">\n{}\n    </file-contents>\n",
                file.path.display(),
                file.path.file_name().unwrap().to_string_lossy(),
                file.contents,
            );
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
    let mut chunk_idx = 0usize;
    let mut file_id = 0usize;

    // Helper to push a chunk if non-empty, resetting xml and toks
    let mut push_chunk = |xml: &mut String, toks: &mut usize, idx: usize| {
        if !xml.is_empty() {
            chunks.push(Chunk {
                index: idx,
                xml: std::mem::take(xml),
                tokens: *toks,
            });
            *toks = 0;
        }
    };

    for file in files {
        let file_tok = count_tokens(&file.contents);
        let file_block = format!(
            "    <file-contents path=\"{}\" name=\"{}\">\n{}\n    </file-contents>\n",
            file.path.display(),
            file.path.file_name().unwrap().to_string_lossy(),
            file.contents,
        );

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
            push_chunk(&mut current_xml, &mut current_toks, chunk_idx);
            chunk_idx += 1;
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
        let mut part_xml = String::new();
        let mut part_tok = 0usize;
        let mut part_idx = 1usize;
        // Calculate number of parts with integer division (ceil)
        let mut max_parts =
            ((file_tok as u128 + max_tokens as u128 - 1) / max_tokens as u128) as usize;
        if max_parts == 0 {
            max_parts = 1;
        }
        for line in file.contents.lines() {
            let new_tok = count_tokens(line) + 1; // include newline
            if part_tok + new_tok > max_tokens {
                let wrapped = wrap_part(&file.path, part_idx, max_parts, &part_xml);
                push_chunk(&mut current_xml, &mut current_toks, chunk_idx);
                current_xml.push_str(&wrapped);
                current_toks = part_tok;
                chunk_idx += 1;
                part_xml.clear();
                part_tok = 0;
                part_idx += 1;
            }
            part_xml.push_str(line);
            part_xml.push('\n');
            part_tok += new_tok;
        }
        // flush last sub-part
        if !part_xml.is_empty() {
            let wrapped = wrap_part(&file.path, part_idx, max_parts, &part_xml);
            push_chunk(&mut current_xml, &mut current_toks, chunk_idx);
            current_xml.push_str(&wrapped);
            current_toks = part_tok;
        }
        metas.push(FileMeta {
            id: file_id,
            path: file.path.clone(),
            tokens: file_tok,
            parts: max_parts,
        });
        file_id += 1;
    }

    // push final chunk
    push_chunk(&mut current_xml, &mut current_toks, chunk_idx);
    (chunks, metas)
}

// Wrap a sub-part of a file into its own XML block
fn wrap_part(
    path: &Path,
    idx: usize,
    total: usize,
    body: &str,
) -> String {
    format!(
        "    <file-contents path=\"{}\" name=\"{}\" part=\"{}/{}\">\n{}    </file-contents>\n",
        path.display(),
        path.file_name().unwrap().to_string_lossy(),
        idx,
        total,
        body
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gather::{FileContents, count_tokens};
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
        let (chunks, metas) = build_chunks(&files, 1000);
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
