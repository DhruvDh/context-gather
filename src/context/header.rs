use crate::chunker::FileMeta;
use crate::context::xml::{maybe_escape_attr, maybe_escape_text};
use chrono::{SecondsFormat, Utc};
use path_slash::PathBufExt;
use std::fmt::Write;
use std::process::Command;

/// Builds the shared-context-header XML for LLM consumption.
pub fn make_header(
    total_chunks: usize,
    limit: usize,
    files: &[FileMeta],
    multi_step: bool,
    escape_xml: bool,
    include_git: bool,
) -> String {
    // Timestamp in RFC3339 with seconds precision
    let ts = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    // Build file-map entries
    let mut map = String::new();
    for f in files {
        let path = f.path.to_slash_lossy().to_string();
        let path_attr = maybe_escape_attr(&path, escape_xml);
        let _ = writeln!(
            &mut map,
            "    <file id=\"{}\" path=\"{}\" tokens=\"{}\" parts=\"{}\"/>",
            f.id, path_attr, f.tokens, f.parts
        );
    }
    // Build instructions section
    let instructions = if multi_step {
        // Multi-step mode instructions
        format!(
            "  <instructions>\n    This header lists {total_files} files available for context retrieval. To fetch file contents, enter one or more file ids (e.g., '2'), file paths (e.g., 'src/main.rs'), or glob patterns (e.g., '*.rs'), and the tool will return their contents in the next message.\n  </instructions>\n",
            total_files = files.len(),
        )
    } else {
        // Chunked mode instructions
        format!(
            "  <instructions>\n    The shared context is split into {total_chunks} chunks (including this header). Review each chunk carefully. Acknowledge that you've studied this each chunk. After reading the final chunk, reply \"READY\" to confirm you have understood the context.\n  </instructions>\n",
            total_chunks = total_chunks,
        )
    };
    // Gather git info: branch, recent commits, and diff
    let mut git_info = String::new();
    let mut diff_xml = String::new();
    if include_git {
        let git_available = Command::new("git")
            .args(["rev-parse", "--is-inside-work-tree"])
            .output()
            .ok()
            .and_then(|o| {
                if o.status.success() {
                    String::from_utf8(o.stdout).ok()
                } else {
                    None
                }
            })
            .map(|s| s.trim() == "true")
            .unwrap_or(false);

        if git_available {
            let branch = Command::new("git")
                .args(["rev-parse", "--abbrev-ref", "HEAD"])
                .output()
                .ok()
                .and_then(|o| {
                    if o.status.success() {
                        String::from_utf8(o.stdout)
                            .ok()
                            .map(|s| s.trim_end().to_string())
                    } else {
                        None
                    }
                });
            let commits = Command::new("git")
                .args(["log", "-5", "--pretty=format:%s"])
                .output()
                .ok()
                .and_then(|o| {
                    if o.status.success() {
                        String::from_utf8(o.stdout).ok()
                    } else {
                        None
                    }
                })
                .unwrap_or_default();
            let commits: Vec<String> = commits.lines().map(|l| l.to_string()).collect();

            if let Some(branch) = branch {
                let branch_attr = maybe_escape_attr(&branch, escape_xml);
                let _ = writeln!(&mut git_info, "  <git-info branch=\"{}\">", branch_attr);
                for msg in commits {
                    let msg_text = maybe_escape_text(&msg, escape_xml);
                    let _ = writeln!(&mut git_info, "    <commit>{}</commit>", msg_text);
                }
                let _ = writeln!(&mut git_info, "  </git-info>");
            } else {
                let _ = writeln!(&mut git_info, "  <!-- git info unavailable -->");
            }

            let diff_out = Command::new("git")
                .args(["diff", "--name-only", "origin/main"])
                .output();
            let diff_ok = diff_out
                .as_ref()
                .map(|o| o.status.success())
                .unwrap_or(false);
            let diff_output = diff_out
                .ok()
                .and_then(|o| {
                    if o.status.success() {
                        String::from_utf8(o.stdout).ok()
                    } else {
                        None
                    }
                })
                .unwrap_or_default();
            let changed: Vec<String> = diff_output.lines().map(|l| l.to_string()).collect();
            if !changed.is_empty() {
                let _ = writeln!(
                    &mut diff_xml,
                    "  <changed-files diffed-against=\"origin/main\">"
                );
                for file in &changed {
                    let file_text = maybe_escape_text(file, escape_xml);
                    let _ = writeln!(&mut diff_xml, "    <file>{}</file>", file_text);
                }
                let _ = writeln!(&mut diff_xml, "  </changed-files>");
            } else if !diff_ok {
                let _ = writeln!(&mut diff_xml, "  <!-- git diff unavailable -->");
            }
        } else {
            let _ = writeln!(&mut git_info, "  <!-- git info unavailable -->");
            let _ = writeln!(&mut diff_xml, "  <!-- git diff unavailable -->");
        }
    }
    // Compose full header with closing tag
    format!(
        "<shared-context-header version=\"1\" total-chunks=\"{total_chunks}\" chunk-size=\"{limit}\" generated-at=\"{ts}\">\n  <file-map total-files=\"{total}\">\n{map}  </file-map>\n{instructions}{git_info}{diff_xml}</shared-context-header>\n",
        total_chunks = total_chunks,
        limit = limit,
        ts = ts,
        total = files.len(),
        map = map,
        instructions = instructions,
        git_info = git_info,
        diff_xml = diff_xml
    )
}
