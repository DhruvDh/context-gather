use anyhow::{Result, anyhow};
use cli_clipboard::{ClipboardContext, ClipboardProvider};

fn try_copy(text: &str) -> Result<()> {
    let mut ctx = ClipboardContext::new().map_err(|e| anyhow!("init clipboard: {e}"))?;
    ctx.set_contents(text.to_owned())
        .map_err(|e| anyhow!("set clipboard contents: {e}"))
}

/// Copy text to clipboard, warning on failure if `fail_hard` is false.
/// Returns true when the clipboard copy succeeds.
pub fn copy_to_clipboard(
    text: &str,
    fail_hard: bool,
) -> Result<bool> {
    match try_copy(text) {
        Ok(()) => Ok(true),
        Err(err) => {
            if fail_hard {
                return Err(anyhow!(
                    "clipboard unavailable ({err}); re-run with --stdout or --no-clipboard"
                ));
            }
            tracing::warn!("clipboard unavailable: {err}");
            Ok(false)
        }
    }
}
