use anyhow::{Result, anyhow};
use cli_clipboard::{ClipboardContext, ClipboardProvider};

fn try_copy(text: &str) -> Result<()> {
    let mut ctx = ClipboardContext::new().map_err(|e| anyhow!("init clipboard: {e}"))?;
    ctx.set_contents(text.to_owned())
        .map_err(|e| anyhow!("set clipboard contents: {e}"))
}

/// Copy text to clipboard, warning on failure if `fail_hard` is false.
/// When `fallback_stdout` is true, emit the text to stdout if clipboard access fails.
pub fn copy_to_clipboard(
    text: &str,
    fail_hard: bool,
    fallback_stdout: bool,
) -> Result<()> {
    if let Err(err) = try_copy(text) {
        if fail_hard {
            return Err(err);
        }
        if fallback_stdout {
            tracing::warn!(
                "WARNING: clipboard unavailable ({err}); writing snippet to stdout instead."
            );
            print!("{text}");
        } else {
            tracing::warn!("WARNING: clipboard unavailable: {err}");
        }
    }
    Ok(())
}
