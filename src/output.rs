use crate::chunker::Chunk;

pub fn format_header_snippet(chunks: &[Chunk]) -> String {
    if chunks.is_empty() {
        return String::new();
    }
    let rem = chunks.len().saturating_sub(1);
    let mut s = chunks[0].xml.clone();
    if rem > 0 {
        s.push_str(&format!("<more remaining=\"{rem}\"/>\n"));
    }
    s
}

pub fn format_chunk_snippet(
    chunks: &[Chunk],
    idx: usize,
) -> String {
    let total = chunks.len();
    let rem = total.saturating_sub(idx + 1);
    if idx == 0 {
        let mut s = chunks[0].xml.clone();
        if rem > 0 {
            s.push_str(&format!("<more remaining=\"{rem}\"/>\n"));
        } else {
            s.push_str("</shared-context>\n");
        }
        s
    } else if rem > 0 {
        format!(
            "<context-chunk id=\"{}/{}\">\n{}</context-chunk>\n<more remaining=\"{}\"/>\n",
            idx, total, chunks[idx].xml, rem
        )
    } else {
        format!(
            "<context-chunk id=\"{}/{}\">\n{}</context-chunk>\n</shared-context>\n",
            idx, total, chunks[idx].xml
        )
    }
}
