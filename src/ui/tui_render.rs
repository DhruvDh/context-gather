use crate::ui::tui_state::{UiState, adjust_scroll_and_slice};
use tui::{
    Frame,
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

/// Renders the TUI given the current state, updating scroll offsets.
pub fn render<B: Backend>(
    frame: &mut Frame<B>,
    state: &mut UiState,
) {
    // Layout: search bar (3 lines), list area, then help bar
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(frame.size());

    // Search bar title and input binding
    let (title, input) = if state.extension_mode {
        (
            "Extensions (Ctrl+E to exit, Enter to apply)".to_owned(),
            &state.extension_search,
        )
    } else {
        let selected_count = state.items.iter().filter(|(_, checked)| *checked).count();
        (
            format!("Fuzzy Search ({selected_count} selected)"),
            &state.search_input,
        )
    };
    let search =
        Paragraph::new(input.as_str()).block(Block::default().borders(Borders::ALL).title(title));
    frame.render_widget(search, chunks[0]);

    let area = chunks[1];
    let max_lines = area.height.saturating_sub(2) as usize;

    if state.extension_mode {
        state.ensure_filtered_exts();
        let list = &state.filtered_exts;

        // Adjust scroll and get visible window
        let (offset, end) = adjust_scroll_and_slice(
            &mut state.ext_selected_idx,
            &mut state.ext_scroll_offset,
            max_lines,
            list.len(),
        );
        let window = &list[offset..end];

        // Build ListItems
        let items: Vec<ListItem> = window
            .iter()
            .map(|&idx| {
                let (text, checked) = &state.extension_items[idx];
                let mark = if *checked { "[x]" } else { "[ ]" };
                let spans = Spans::from(vec![
                    Span::styled(mark, Style::default().fg(Color::Yellow)),
                    Span::raw(" "),
                    Span::raw(text.clone()),
                ]);
                ListItem::new(spans)
            })
            .collect();

        // Render Extensions list with highlighting
        let mut list_state = ListState::default();
        list_state.select(Some(state.ext_selected_idx.saturating_sub(offset)));
        let widget = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Extensions"))
            .highlight_style(Style::default().bg(Color::Blue));
        frame.render_stateful_widget(widget, area, &mut list_state);
    } else {
        state.ensure_filtered_files();
        let list = &state.filtered_files;

        // Adjust scroll and get visible window
        let (offset, end) = adjust_scroll_and_slice(
            &mut state.selected_idx,
            &mut state.scroll_offset,
            max_lines,
            list.len(),
        );
        let window = &list[offset..end];

        // Build ListItems
        let items: Vec<ListItem> = window
            .iter()
            .map(|&idx| {
                let text = &state.item_display[idx];
                let checked = state.items[idx].1;
                let mark = if checked { "[x]" } else { "[ ]" };
                let spans = Spans::from(vec![
                    Span::styled(mark, Style::default().fg(Color::Yellow)),
                    Span::raw(" "),
                    Span::raw(text.clone()),
                ]);
                ListItem::new(spans)
            })
            .collect();

        // Render Files list with highlighting
        let mut list_state = ListState::default();
        list_state.select(Some(state.selected_idx.saturating_sub(offset)));
        let widget = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Files"))
            .highlight_style(Style::default().bg(Color::Blue));
        frame.render_stateful_widget(widget, area, &mut list_state);
    }

    // Help bar at bottom
    let help_text = vec![
        Span::styled("↑/↓: Navigate  ", Style::default().fg(Color::Yellow)),
        Span::styled("Space: Toggle  ", Style::default().fg(Color::Yellow)),
        Span::styled("Enter: Submit  ", Style::default().fg(Color::Yellow)),
        Span::styled("Ctrl+E: Ext  ", Style::default().fg(Color::Yellow)),
        Span::styled("q: Quit", Style::default().fg(Color::Yellow)),
    ];
    let help_bar =
        Paragraph::new(Spans::from(help_text)).block(Block::default().borders(Borders::ALL));
    frame.render_widget(help_bar, chunks[2]);
}
