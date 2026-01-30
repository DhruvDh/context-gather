use std::panic;
use std::sync::atomic::{AtomicBool, Ordering};

use context_gather::ui::interactive::{test_clear_wrapper_hook_flag, test_wrapper_hook_ran};
use context_gather::ui::select_files_tui;

static HOOK_CALLED: AtomicBool = AtomicBool::new(false);

#[test]
fn tui_restores_panic_hook() {
    // Custom hook that flips a flag when invoked
    let custom_hook = Box::new(|_: &panic::PanicHookInfo<'_>| {
        HOOK_CALLED.store(true, Ordering::SeqCst);
    });
    let orig = panic::take_hook();
    panic::set_hook(custom_hook);

    // Force select_files_tui to exit immediately
    unsafe {
        std::env::set_var("CG_TEST_AUTOQUIT", "1");
    }
    let _ = select_files_tui(Vec::new(), &[]);
    unsafe {
        std::env::remove_var("CG_TEST_AUTOQUIT");
    }
    test_clear_wrapper_hook_flag();

    // Trigger a panic and swallow it
    let _ = panic::catch_unwind(|| panic!("boom"));

    assert!(HOOK_CALLED.load(Ordering::SeqCst));
    assert!(
        !test_wrapper_hook_ran(),
        "wrapper hook ran after select_files_tui returned"
    );

    // Restore original hook for cleanliness
    panic::set_hook(orig);
}
