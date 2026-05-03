use crate::chunker::FileMeta;
use crate::context::xml::{maybe_escape_attr, maybe_escape_text};
use chrono::{SecondsFormat, Utc};
use path_slash::PathBufExt;
use std::fmt::Write;
use std::process::Command;

fn git_stdout(args: &[&str]) -> Option<String> {
    Command::new("git").args(args).output().ok().and_then(|o| {
        if o.status.success() {
            String::from_utf8(o.stdout)
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        } else {
            None
        }
    })
}

fn git_ref_exists(refname: &str) -> bool {
    Command::new("git")
        .args(["rev-parse", "--verify", "--quiet", refname])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn detect_changed_files_base() -> Option<String> {
    if let Some(upstream) = git_stdout(&[
        "rev-parse",
        "--abbrev-ref",
        "--symbolic-full-name",
        "@{upstream}",
    ]) {
        return Some(upstream);
    }
    if let Some(origin_head) = git_stdout(&[
        "symbolic-ref",
        "--quiet",
        "--short",
        "refs/remotes/origin/HEAD",
    ]) {
        return Some(origin_head);
    }
    for (refname, display) in [
        ("refs/heads/main", "main"),
        ("refs/heads/master", "master"),
        ("refs/remotes/origin/main", "origin/main"),
        ("refs/remotes/origin/master", "origin/master"),
    ] {
        if git_ref_exists(refname) {
            return Some(display.to_string());
        }
    }
    None
}

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
    let escape_note = if escape_xml {
        "    File contents are XML-escaped; angle brackets and ampersands are encoded.\n"
    } else {
        "    File contents are unescaped; header metadata remains escaped.\n"
    };
    let instructions = if multi_step {
        // Multi-step mode instructions
        format!(
            "  <instructions>\n    This header lists {total_files} files available for context retrieval. To fetch file contents, enter a file id (e.g., '2'), a file path (e.g., 'src/main.rs'), or a glob pattern (e.g., '*.rs'); glob patterns may match multiple files, and the tool will return those contents in the next message.\n{escape_note}  </instructions>\n",
            total_files = files.len(),
            escape_note = escape_note
        )
    } else {
        // Chunked mode instructions
        format!(
            "  <instructions>\n    The shared context is split into {total_chunks} chunks (including this header). Review each chunk carefully. Acknowledge that you've studied this each chunk. After reading the final chunk, reply \"READY\" to confirm you have understood the context.\n{escape_note}  </instructions>\n",
            total_chunks = total_chunks,
            escape_note = escape_note
        )
    };
    // Gather git info: branch, recent commits, and changed files
    let mut git_info = String::new();
    let mut changed_files_xml = String::new();
    if include_git {
        let git_available = git_stdout(&["rev-parse", "--is-inside-work-tree"])
            .map(|s| s == "true")
            .unwrap_or(false);

        if git_available {
            let branch = git_stdout(&["rev-parse", "--abbrev-ref", "HEAD"]);
            let commits = git_stdout(&["log", "-5", "--pretty=format:%s"]).unwrap_or_default();
            let commits: Vec<String> = commits.lines().map(|l| l.to_string()).collect();

            if let Some(branch) = branch {
                let branch_attr = maybe_escape_attr(&branch, escape_xml);
                let _ = writeln!(&mut git_info, "  <git-info branch=\"{}\">", branch_attr);
                for msg in commits {
                    let msg_text = maybe_escape_text(&msg, true);
                    let _ = writeln!(&mut git_info, "    <commit>{}</commit>", msg_text);
                }
                let _ = writeln!(&mut git_info, "  </git-info>");
            } else {
                let _ = writeln!(&mut git_info, "  <!-- git info unavailable -->");
            }

            if let Some(base) = detect_changed_files_base() {
                let diff_out = Command::new("git")
                    .args(["diff", "--name-only", &base])
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
                    let base_attr = maybe_escape_attr(&base, escape_xml);
                    let _ = writeln!(
                        &mut changed_files_xml,
                        "  <changed-files diffed-against=\"{}\">",
                        base_attr
                    );
                    for file in &changed {
                        let file_text = maybe_escape_text(file, true);
                        let _ = writeln!(&mut changed_files_xml, "    <file>{}</file>", file_text);
                    }
                    let _ = writeln!(&mut changed_files_xml, "  </changed-files>");
                } else if !diff_ok {
                    let _ = writeln!(
                        &mut changed_files_xml,
                        "  <!-- changed files unavailable -->"
                    );
                }
            } else {
                let _ = writeln!(
                    &mut changed_files_xml,
                    "  <!-- changed files unavailable: no git base found -->"
                );
            }
        } else {
            let _ = writeln!(&mut git_info, "  <!-- git info unavailable -->");
            let _ = writeln!(
                &mut changed_files_xml,
                "  <!-- changed files unavailable: not a git repository -->"
            );
        }
    }
    // Compose full header with closing tag
    format!(
        "<shared-context-header version=\"1\" total-chunks=\"{total_chunks}\" chunk-size=\"{limit}\" generated-at=\"{ts}\">\n  <file-map total-files=\"{total}\">\n{map}  </file-map>\n{instructions}{git_info}{changed_files_xml}</shared-context-header>\n",
        total_chunks = total_chunks,
        limit = limit,
        ts = ts,
        total = files.len(),
        map = map,
        instructions = instructions,
        git_info = git_info,
        changed_files_xml = changed_files_xml
    )
}
