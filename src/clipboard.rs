use anyhow::Result;
use copypasta::{ClipboardContext, ClipboardProvider};

/// Copy text to clipboard, warning on failure if `fail_hard` is false.
pub fn copy_to_clipboard(text: &str,
                         fail_hard: bool)
                         -> Result<()> {
    // Initialize clipboard context
    #[cfg(target_os = "linux")]
    let ctx_res = ClipboardContext::new_wayland().or_else(|_| ClipboardContext::new());
    #[cfg(not(target_os = "linux"))]
    let ctx_res = ClipboardContext::new();
    let mut ctx = match ctx_res {
        Ok(c) => c,
        Err(e) => {
            if fail_hard {
                return Err(anyhow::anyhow!("Clipboard init failed: {:?}", e));
            } else {
                eprintln!("⚠️  Clipboard unavailable: {e:?}");
                return Ok(());
            }
        }
    };
    // Set clipboard contents
    if let Err(e) = ctx.set_contents(text.to_string()) {
        if fail_hard {
            return Err(anyhow::anyhow!("Clipboard copy failed: {:?}", e));
        } else {
            eprintln!("⚠️  Clipboard copy failed: {e:?}");
        }
    }
    Ok(())
}
