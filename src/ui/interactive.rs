use std::{panic, path::PathBuf};

use crate::ui::{tui_events, tui_render, tui_state};
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use tui::{
    Terminal,
    backend::{Backend, CrosstermBackend},
};

/// Clean up terminal state: disable raw mode, exit alternate screen, disable mouse capture, and show cursor.
fn cleanup_terminal<B: Backend>(terminal: &mut Terminal<B>) -> Result<()> {
    disable_raw_mode()?;
    // Restore terminal screen and disable mouse capture using stdout
    let mut stdout = std::io::stdout();
    execute!(stdout, LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;
    Ok(())
}

pub fn select_files_tui(
    paths: Vec<PathBuf>,
    preselected: &[PathBuf],
) -> Result<Vec<PathBuf>> {
    // Install panic hook to restore terminal on panic
    let default_hook = panic::take_hook();
    let default_hook_ptr = &default_hook as *const _;
    panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(std::io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        unsafe { (*default_hook_ptr)(info); }
    }));

    // Initialize state
    let mut state = tui_state::UiState::new(paths, preselected);

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(std::io::stdout());
    let mut terminal = Terminal::new(backend)?;

    // Event loop
    loop {
        // Render UI; pass mutable reference to state for rendering
        terminal.draw(|f| tui_render::render(f, &mut state))?;

        // Handle input
        let evt: Event = event::read()?;
        if let Some(msg) = tui_events::handle_event(&mut state, evt) {
            match msg {
                tui_events::UiMsg::Quit => {
                    cleanup_terminal(&mut terminal)?;
                    panic::set_hook(default_hook);
                    return Ok(vec![]);
                }
                tui_events::UiMsg::Submit => {
                    cleanup_terminal(&mut terminal)?;
                    panic::set_hook(default_hook);
                    return Ok(state.selected_paths());
                }
                _ => {}
            }
        }
    }
}
