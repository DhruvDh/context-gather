// Helper module for token-based chunking of XML output
use crate::gather::count_tokens;

/// Given the raw XML-ish string, break it at line-boundaries so
/// each part's token count ≤ limit. Zero means "just return one part".
pub fn chunk_by_tokens(
    xml: &str,
    limit: usize,
) -> Vec<String> {
    if limit == 0 {
        return vec![xml.to_string()];
    }

    let mut out = Vec::<String>::new();
    let mut current = String::new();

    for line in xml.lines() {
        let prospective = if current.is_empty() {
            line.to_owned()
        } else {
            format!("{current}\n{line}")
        };

        if count_tokens(&prospective) > limit {
            out.push(current);
            current = line.to_owned();
        } else {
            current = prospective;
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}
