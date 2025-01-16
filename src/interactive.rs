use std::path::PathBuf;
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal,
};

pub fn select_files_tui(paths: Vec<PathBuf>) -> Result<Vec<PathBuf>> {
    // Convert paths into a local data structure
    let mut items: Vec<(PathBuf, bool)> = paths.into_iter().map(|p| (p, false)).collect();

    // State for fuzzy search input
    let mut search_input = String::new();

    // Helper closure for filtering items
    let filter_items = |items: &[(PathBuf, bool)], search: &str| {
        let search_lower = search.to_lowercase();
        items.iter()
            .enumerate()
            .filter_map(|(idx, (p, checked))| {
                let filename = p.file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default()
                    .to_lowercase();
                if filename.contains(&search_lower) {
                    Some((idx, p.clone(), *checked))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
    };

    // Keep track of the currently selected index among filtered items
    let mut filtered = filter_items(&items, &search_input);
    let mut selected_idx = 0usize;

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(std::io::stdout());
    let mut terminal = Terminal::new(backend)?;

    // Main loop
    loop {
        // Recompute filtered list each frame
        filtered = filter_items(&items, &search_input);
        if filtered.is_empty() {
            selected_idx = 0;
        } else if selected_idx >= filtered.len() {
            selected_idx = filtered.len() - 1;
        }

        // Draw UI
        terminal.draw(|f| {
            let size = f.size();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),  // for search input
                    Constraint::Min(1),    // for file list
                ])
                .split(size);

            // Draw search bar
            let search_bar = Paragraph::new(search_input.as_ref())
                .block(Block::default().title("Fuzzy Search").borders(Borders::ALL));
            f.render_widget(search_bar, chunks[0]);

            // Build list items
            let list_items: Vec<ListItem> = filtered.iter().enumerate().map(|(i, (_idx_p, path, checked))| {
                let mark = if *checked { "[x]" } else { "[ ]" };
                let filename = path.file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "unknown".to_string());
                let line = format!("{} {}", mark, filename);
                if i == selected_idx {
                    // highlight selected
                    ListItem::new(Spans::from(vec![Span::styled(line, Style::default().fg(Color::Yellow))]))
                } else {
                    ListItem::new(Spans::from(line))
                }
            }).collect();

            let files_list = List::new(list_items)
                .block(Block::default().title("Files").borders(Borders::ALL));
            f.render_widget(files_list, chunks[1]);
        })?;

        // Handle input
        if let Event::Key(KeyEvent { code, modifiers, .. }) = event::read()? {
            match (code, modifiers) {
                (KeyCode::Char('q'), _) => {
                    // Quit without selection
                    disable_raw_mode()?;
                    execute!(
                        terminal.backend_mut(),
                        LeaveAlternateScreen,
                        DisableMouseCapture
                    )?;
                    terminal.show_cursor()?;
                    return Ok(vec![]);
                }
                KeyCode::Char('\n') => {
                    // Done selecting
                    disable_raw_mode()?;
                    execute!(
                        terminal.backend_mut(),
                        LeaveAlternateScreen,
                        DisableMouseCapture
                    )?;
                    terminal.show_cursor()?;
                    // Return selected paths
                    let selected_paths: Vec<PathBuf> = items
                        .iter()
                        .filter(|(_p, checked)| *checked)
                        .map(|(p, _)| p.clone())
                        .collect();
                    return Ok(selected_paths);
                }
                KeyCode::Up => {
                    if selected_idx > 0 {
                        selected_idx -= 1;
                    }
                }
                KeyCode::Down => {
                    if selected_idx + 1 < filtered.len() {
                        selected_idx += 1;
                    }
                }
                KeyCode::Char(' ') => {
                    // Toggle checkbox on currently selected item
                    if let Some((actual_idx, _, _)) = filtered.get(selected_idx) {
                        items[*actual_idx].1 = !items[*actual_idx].1;
                    }
                }
                (KeyCode::Char('a'), KeyModifiers::CONTROL) => {
                    // Ctrl-A => Select all
                    for i in 0..items.len() {
                        items[i].1 = true;
                    }
                }
                (KeyCode::Char('i'), KeyModifiers::CONTROL) => {
                    // Ctrl-I => Invert selection
                    for i in 0..items.len() {
                        items[i].1 = !items[i].1;
                    }
                }
                KeyCode::Backspace => {
                    search_input.pop();
                }
                KeyCode::Char(c) => {
                    // Type in fuzzy search
                    search_input.push(c);
                }
                _ => {}
            }
        }
    }
}
