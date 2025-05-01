use anyhow::Result;
use copypasta::{ClipboardContext, ClipboardProvider};

pub fn copy_to_clipboard(text: &str) -> Result<()> {
    // Platform-specific clipboard context
    #[cfg(target_os = "linux")]
    let mut ctx = ClipboardContext::new_wayland().or_else(|_| ClipboardContext::new())
                                                 .map_err(|e| {
                                                     anyhow::anyhow!("Failed to create Wayland \
                                                                      clipboard context: {:?}",
                                                                     e)
                                                 })?;
    #[cfg(not(target_os = "linux"))]
    let mut ctx =
        ClipboardContext::new().map_err(|e| {
                                   anyhow::anyhow!("Failed to create clipboard context: {:?}", e)
                               })?;
    ctx.set_contents(text.to_string())
       .map_err(|e| anyhow::anyhow!("Failed to copy to clipboard: {:?}", e))?;
    Ok(())
}
