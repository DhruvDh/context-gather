use std::path::PathBuf;

use anyhow::Result;
use crossterm::{
    event::{
        self,
        DisableMouseCapture,
        EnableMouseCapture,
        Event,
        KeyCode,
        KeyEvent,
        KeyModifiers,
    },
    execute,
    terminal::{
        EnterAlternateScreen,
        LeaveAlternateScreen,
        disable_raw_mode,
        enable_raw_mode,
    },
};
use fuzzy_matcher::{
    FuzzyMatcher,
    skim::SkimMatcherV2,
};
use tui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{
        Constraint,
        Direction,
        Layout,
    },
    style::{
        Color,
        Style,
    },
    text::{
        Span,
        Spans,
    },
    widgets::{
        Block,
        Borders,
        List,
        ListItem,
        Paragraph,
    },
};

pub fn select_files_tui(paths: Vec<PathBuf>, preselected: &[PathBuf]) -> Result<Vec<PathBuf>> {
    // Mark items as checked if they're in `preselected`
    let mut items: Vec<(PathBuf, bool)> = paths
        .into_iter()
        .map(|p| {
            let is_checked = preselected.contains(&p);
            (p, is_checked)
        })
        .collect();

    // State for fuzzy search input
    let mut search_input = String::new();

    // Helper closure for filtering items
    let filter_items = |items: &[(PathBuf, bool)], search: &str| {
        // If nothing is typed, just return everything in original order:
        if search.is_empty() {
            return items
                .iter()
                .enumerate()
                .map(|(idx, (p, checked))| (idx, p.clone(), *checked))
                .collect::<Vec<_>>();
        }

        // Otherwise, do fuzzy matching with descending score
        let matcher = SkimMatcherV2::default();
        let mut results = Vec::new();

        for (i, (p, checked)) in items.iter().enumerate() {
            let path_str = p.to_string_lossy();
            if let Some(score) = matcher.fuzzy_match(&path_str, search) {
                results.push((i, p.clone(), *checked, score));
            }
        }

        // Sort by score (descending)
        results.sort_by_key(|&(_, _, _, score)| -score);

        // Drop the score; return the triple
        results.into_iter().map(|(i, p, c, _)| (i, p, c)).collect()
    };

    // Keep track of the currently selected index among filtered items
    let mut selected_idx = 0usize;
    let mut scroll_offset = 0usize; // which line in `filtered` we start rendering from

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(std::io::stdout());
    let mut terminal = Terminal::new(backend)?;

    // Main loop
    loop {
        // Recompute filtered list each frame
        let filtered = filter_items(&items, &search_input);
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
                    Constraint::Length(3), // for search input
                    Constraint::Min(1),    // for file list
                ])
                .split(size);

            let list_area = chunks[1];
            // The list area height determines how many lines we can show
            let max_lines = list_area.height.saturating_sub(2) as usize;
            // some extra margin for borders, adjust as needed

            // Adjust scroll_offset so selected_idx is always in view
            if selected_idx < scroll_offset {
                scroll_offset = selected_idx;
            } else if selected_idx >= scroll_offset + max_lines {
                scroll_offset = selected_idx.saturating_sub(max_lines).saturating_add(1);
            }

            // We'll slice the filtered items for drawing
            let end_idx = (scroll_offset + max_lines).min(filtered.len());
            let visible_slice = &filtered[scroll_offset..end_idx];

            // Draw search bar
            let search_bar = Paragraph::new(search_input.as_ref())
                .block(Block::default().title("Fuzzy Search").borders(Borders::ALL));
            f.render_widget(search_bar, chunks[0]);

            // Build list items from the visible slice only
            let list_items: Vec<ListItem> = visible_slice
                .iter()
                .enumerate()
                .map(|(i, (_idx_p, path, checked))| {
                    let mark = if *checked { "[x]" } else { "[ ]" };
                    let line = format!("{} {}", mark, path.display());
                    // The index for display is i + scroll_offset
                    let displayed_idx = i + scroll_offset;
                    if displayed_idx == selected_idx {
                        // highlight selected
                        ListItem::new(Spans::from(vec![Span::styled(
                            line,
                            Style::default().fg(Color::Yellow),
                        )]))
                    } else {
                        ListItem::new(Spans::from(line))
                    }
                })
                .collect();

            let files_list =
                List::new(list_items).block(Block::default().title("Files").borders(Borders::ALL));
            f.render_widget(files_list, chunks[1]);
        })?;

        // Handle input
        if let Event::Key(KeyEvent {
            code,
            modifiers,
            ..
        }) = event::read()?
        {
            match (code, modifiers) {
                // Quit without selection (requires Ctrl+Q)
                (KeyCode::Char('q'), KeyModifiers::CONTROL) => {
                    disable_raw_mode()?;
                    execute!(
                        terminal.backend_mut(),
                        LeaveAlternateScreen,
                        DisableMouseCapture
                    )?;
                    terminal.show_cursor()?;
                    return Ok(vec![]);
                }
                // Done selecting
                (KeyCode::Enter, _) => {
                    disable_raw_mode()?;
                    execute!(
                        terminal.backend_mut(),
                        LeaveAlternateScreen,
                        DisableMouseCapture
                    )?;
                    terminal.show_cursor()?;
                    let selected_paths: Vec<PathBuf> = items
                        .iter()
                        .filter(|(_p, checked)| *checked)
                        .map(|(p, _)| p.clone())
                        .collect();
                    return Ok(selected_paths);
                }
                (KeyCode::Up, _) => {
                    selected_idx = selected_idx.saturating_sub(1);
                }
                (KeyCode::Down, _) => {
                    if !filtered.is_empty() && selected_idx < filtered.len() - 1 {
                        selected_idx += 1;
                    }
                }
                (KeyCode::Char(' '), _) => {
                    // Toggle checkbox
                    if let Some((actual_idx, _, _)) = filtered.get(selected_idx) {
                        items[*actual_idx].1 = !items[*actual_idx].1;
                    }
                }
                // Ctrl-A => toggle (select/unselect) all VISIBLE items
                (KeyCode::Char('a'), KeyModifiers::CONTROL) => {
                    // Check if *all filtered* are currently selected
                    let all_filtered_selected = filtered.iter().all(|(_, _, c)| *c);
                    for (idx, _, _) in &filtered {
                        items[*idx].1 = !all_filtered_selected;
                    }
                }

                // Ctrl-I => invert selection for VISIBLE (filtered) items
                (KeyCode::Char('i'), KeyModifiers::CONTROL) => {
                    for (idx, _, _) in &filtered {
                        items[*idx].1 = !items[*idx].1;
                    }
                }
                (KeyCode::Backspace, _) => {
                    search_input.pop();
                }
                // Add typed character to fuzzy input
                (KeyCode::Char(c), _) => {
                    search_input.push(c);
                }
                _ => {}
            }
        }
    }
}
