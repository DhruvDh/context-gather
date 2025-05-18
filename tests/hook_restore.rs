use context_gather::ui::select_files_tui;
use std::panic;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};

fn sentinel(_: &panic::PanicInfo<'_>) {}

#[test]
fn tui_restores_hook() {
    // capture original hook
    let orig_hook = panic::take_hook();
    panic::set_hook(Box::new(sentinel));
    let sentinel_ptr = sentinel as usize;

    let handle = std::thread::spawn(|| {
        let _ = select_files_tui(Vec::new(), &[]);
    });
    event::push(Event::Key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE))).unwrap();
    handle.join().unwrap();

    let cur_hook = panic::take_hook();
    let cur_ptr = &*cur_hook as *const _ as usize;
    assert_eq!(cur_ptr, sentinel_ptr, "panic hook not restored");
    panic::set_hook(cur_hook);
    panic::set_hook(orig_hook);
}
