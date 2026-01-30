#[derive(Debug, Clone)]
pub struct RenderedChunk {
    /// Fully rendered snippet as printed/copied.
    pub xml: String,
    /// Token count for the rendered snippet.
    pub tokens: usize,
}

pub(crate) fn render_chunk_snippet(
    header_xml: &str,
    body_xmls: &[String],
    idx: usize,
) -> String {
    let total = body_xmls.len() + 1;
    let rem = total.saturating_sub(idx + 1);
    if idx == 0 {
        let mut s = header_xml.to_string();
        if rem > 0 {
            s.push_str(&format!("<more remaining=\"{rem}\"/>\n"));
        } else {
            s.push_str("</shared-context>\n");
        }
        s
    } else if rem > 0 {
        format!(
            "<context-chunk id=\"{}/{}\">\n{}</context-chunk>\n",
            idx,
            total,
            body_xmls[idx - 1]
        )
    } else {
        format!(
            "<context-chunk id=\"{}/{}\">\n{}</context-chunk>\n</shared-context>\n",
            idx,
            total,
            body_xmls[idx - 1]
        )
    }
}
