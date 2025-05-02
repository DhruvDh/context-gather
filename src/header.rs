use crate::chunker::FileMeta;
use chrono::{SecondsFormat, Utc};
use std::fmt::Write;

/// Builds the shared-context-header XML for LLM consumption.
pub fn make_header(
    total_chunks: usize,
    limit: usize,
    files: &[FileMeta],
) -> String {
    // Timestamp in RFC3339 with seconds precision
    let ts = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    // Build file-map entries
    let mut map = String::new();
    for f in files {
        writeln!(
            &mut map,
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
        "  <instructions>\n    You will receive {total_chunks} chunks (including this header). Study these carefully, your understanding of the shared context is critical to your ability to help the user with their task.\n    Respond \"READY\" after the final chunk after you have read and understood the shared context.\n  </instructions>\n"
    );
    // Compose full header with closing tag
    format!(
        "<shared-context-header version=\"1\" total-chunks=\"{total_chunks}\" chunk-size=\"{limit}\" generated-at=\"{ts}\">\n  <file-map total-files=\"{total}\">\n{map}  </file-map>\n{instructions}</shared-context-header>\n",
        total_chunks = total_chunks,
        limit = limit,
        ts = ts,
        total = files.len(),
        map = map,
        instructions = instructions
    )
}
