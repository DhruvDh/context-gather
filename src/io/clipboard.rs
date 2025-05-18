use anyhow::Result;
use copypasta::{ClipboardContext, ClipboardProvider};

/// Copy text to clipboard, warning on failure if `fail_hard` is false.
pub fn copy_to_clipboard(
    text: &str,
    fail_hard: bool,
) -> Result<()> {
    // Initialize clipboard context
    let ctx_res = ClipboardContext::new();
    let mut ctx = match ctx_res {
        Ok(c) => c,
        Err(e) => {
            if fail_hard {
                return Err(anyhow::anyhow!("Clipboard init failed: {:?}", e));
            } else {
                tracing::warn!("WARNING: Clipboard unavailable: {:?}", e);
                return Ok(());
            }
        }
    };
    // Set clipboard contents
    if let Err(e) = ctx.set_contents(text.to_string()) {
        if fail_hard {
            return Err(anyhow::anyhow!("Clipboard copy failed: {:?}", e));
        } else {
            tracing::warn!("WARNING: Clipboard copy failed: {:?}", e);
        }
    }
    Ok(())
}
