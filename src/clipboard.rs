use anyhow::Result;
use copypasta::{ClipboardContext, ClipboardProvider};

pub fn copy_to_clipboard(text: &str) -> Result<()> {
    let mut ctx = ClipboardContext::new()
        .map_err(|e| anyhow::anyhow!("Failed to create clipboard context: {:?}", e))?;
    ctx.set_contents(text.to_string())
        .map_err(|e| anyhow::anyhow!("Failed to copy to clipboard: {:?}", e))?;
    Ok(())
}
