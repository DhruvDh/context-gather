use crate::chunker::FileMeta;
use chrono::Utc;
use std::fmt::Write;

/// Builds the context header XML for LLM consumption.
pub fn make_header(
    total_chunks: usize,
    limit: usize,
    files: &[FileMeta],
) -> String {
    // Timestamp in RFC3339 with seconds precision
    let ts = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    // Build file-map entries
    let mut map = String::new();
    for f in files {
        writeln!(
            map,
            "    <file id=\"{}\" path=\"{}\" tokens=\"{}\" parts=\"{}\"/>",
            f.id,
            f.path.display(),
            f.tokens,
            f.parts
        )
        .unwrap();
    }
    // Build instructions section with actual chunk count
    let instructions = format!(
        "  <instructions>\n    You will receive {total_chunks} chunks (including this header).\n    Reassemble files in <file-map> order. Respond \"READY\" after the final chunk.\n  </instructions>\n"
    );
    // Compose full header
    format!(
        "<context-header version=\"1\" total-chunks=\"{total_chunks}\" chunk-size=\"{limit}\" generated-at=\"{ts}\">\n  <file-map total-files=\"{total}\">\n{map}  </file-map>\n{instructions}</context-header>\n",
        total_chunks = total_chunks,
        limit = limit,
        ts = ts,
        total = files.len(),
        map = map,
        instructions = instructions
    )
}
