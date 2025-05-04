use crate::chunker::FileMeta;
use chrono::{SecondsFormat, Utc};
use std::fmt::Write;
use std::process::Command;

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
        let _ = writeln!(
            &mut map,
            "    <file id=\"{}\" path=\"{}\" tokens=\"{}\" parts=\"{}\"/>",
            f.id,
            f.path.display(),
            f.tokens,
            f.parts
        );
    }
    // Build instructions section with actual chunk count
    let instructions = format!(
        "  <instructions>\n    You will receive {total_chunks} chunks (including this header). Study these carefully, your understanding of the shared context is critical to your ability to help the user with their task.\n    Respond \"READY\" after the final chunk after you have read and understood the shared context.\n  </instructions>\n"
    );
    // Gather git info: branch and last 5 commits
    let (branch_opt, commits) = (|| {
        // Get current branch
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
        // Get last 5 commit messages
        let msgs = Command::new("git")
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
        // Collect last 5 commit messages into a Vec<String>
        let commits: Vec<String> = msgs.lines().map(|l| l.to_string()).collect();
        (branch, commits)
    })();
    // Build git-info XML
    let mut git_info = String::new();
    if let Some(branch) = branch_opt {
        let _ = writeln!(&mut git_info, "  <git-info branch=\"{}\">", branch);
        for msg in commits {
            let _ = writeln!(&mut git_info, "    <commit>{}</commit>", msg);
        }
        let _ = writeln!(&mut git_info, "  </git-info>");
    }
    // Gather changed files relative to origin/main
    let diff_output = Command::new("git")
        .args(["diff", "--name-only", "origin/main"])
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
    let changed: Vec<String> = diff_output.lines().map(|l| l.to_string()).collect();
    let mut diff_xml = String::new();
    if !changed.is_empty() {
        let _ = writeln!(&mut diff_xml, "  <changed-files>");
        for file in &changed {
            let _ = writeln!(&mut diff_xml, "    <file>{}</file>", file);
        }
        let _ = writeln!(&mut diff_xml, "  </changed-files>");
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
