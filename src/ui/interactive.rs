use std::io::{self, IsTerminal};
use std::sync::Arc;
use std::{panic, path::PathBuf};

use crate::ui::{tui_events, tui_render, tui_state};
use anyhow::{Result, anyhow};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::{Backend, CrosstermBackend},
};

/// RAII guard that restores the previous panic hook on drop so we don't leak
/// the temporary terminal-cleanup hook into the rest of the process/tests.
struct HookGuard {
    original: Arc<dyn Fn(&panic::PanicHookInfo<'_>) + Send + Sync + 'static>,
}

impl Drop for HookGuard {
    fn drop(&mut self) {
        let orig = Arc::clone(&self.original);
        panic::set_hook(Box::new(move |info| (orig)(info)));
    }
}

/// RAII guard that restores terminal state on drop.
struct TerminalGuard<B: Backend> {
    terminal: Terminal<B>,
}

impl<B: Backend> TerminalGuard<B> {
    fn new(terminal: Terminal<B>) -> Self {
        Self { terminal }
    }

    fn terminal_mut(&mut self) -> &mut Terminal<B> {
        &mut self.terminal
    }
}

impl<B: Backend> Drop for TerminalGuard<B> {
    fn drop(&mut self) {
        let _ = cleanup_terminal(&mut self.terminal);
    }
}

/// Clean up terminal state: disable raw mode, exit alternate screen, disable mouse capture, and show cursor.
fn cleanup_terminal<B: Backend>(terminal: &mut Terminal<B>) -> Result<()> {
    disable_raw_mode()?;
    // Restore terminal screen and disable mouse capture using stderr (stdout might be piped)
    let mut stderr = io::stderr();
    execute!(stderr, LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;
    Ok(())
}

pub fn select_files_tui(
    paths: Vec<PathBuf>,
    preselected: &[PathBuf],
) -> Result<Vec<PathBuf>> {
    // Ensure we have a TTY to render against; stderr stays attached when stdout is piped.
    if !io::stderr().is_terminal() {
        return Err(anyhow!(
            "interactive mode requires an attached terminal (stderr is not a TTY)"
        ));
    }

    // Install panic hook to restore terminal on panic
    let default_hook = panic::take_hook();
    let default_hook: Arc<dyn Fn(&panic::PanicHookInfo<'_>) + Send + Sync + 'static> =
        default_hook.into();
    let _guard = HookGuard {
        original: default_hook.clone(),
    };
    panic::set_hook(Box::new({
        let dh = default_hook.clone();
        move |info| {
            let _ = disable_raw_mode();
            let _ = execute!(io::stderr(), LeaveAlternateScreen, DisableMouseCapture);
            (dh)(info);
        }
    }));

    // If running under tests with CG_TEST_AUTOQUIT set, skip TUI loop
    if std::env::var_os("CG_TEST_AUTOQUIT").is_some() {
        return Ok(paths);
    }

    // Initialize state
    let mut state = tui_state::UiState::new(paths, preselected);

    // Setup terminal
    enable_raw_mode()?;
    let mut stderr = io::stderr();
    execute!(stderr, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(io::stderr());
    let terminal = Terminal::new(backend)?;
    let mut terminal = TerminalGuard::new(terminal);

    // Event loop
    loop {
        // Render UI; pass mutable reference to state for rendering
        terminal
            .terminal_mut()
            .draw(|f| tui_render::render(f, &mut state))?;

        // Handle input
        let evt: Event = event::read()?;
        if let Some(msg) = tui_events::handle_event(&mut state, evt) {
            match msg {
                tui_events::UiMsg::Quit => {
                    return Ok(vec![]);
                }
                tui_events::UiMsg::Submit => {
                    return Ok(state.selected_paths());
                }
                _ => {}
            }
        }
    }
}
