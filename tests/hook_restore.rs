use std::panic;
use std::sync::atomic::{AtomicBool, Ordering};

use context_gather::ui::select_files_tui;

static HOOK_CALLED: AtomicBool = AtomicBool::new(false);

#[test]
fn tui_restores_panic_hook() {
    // Custom hook that flips a flag when invoked
    let custom_hook = Box::new(|_: &panic::PanicInfo<'_>| {
        HOOK_CALLED.store(true, Ordering::SeqCst);
    });
    let orig = panic::take_hook();
    panic::set_hook(custom_hook);

    // Force select_files_tui to exit immediately
    std::env::set_var("CG_TEST_AUTOQUIT", "1");
    let _ = select_files_tui(Vec::new(), &[]);
    std::env::remove_var("CG_TEST_AUTOQUIT");

    // Trigger a panic and swallow it
    let _ = panic::catch_unwind(|| panic!("boom"));

    assert!(HOOK_CALLED.load(Ordering::SeqCst));

    // Restore original hook for cleanliness
    panic::set_hook(orig);
}
