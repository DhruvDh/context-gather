use crate::ui::tui_state::UiState;
use tui::Frame;
use tui::backend::Backend;
use tui::layout::{Constraint, Direction, Layout};
use tui::style::{Color, Style};
use tui::text::{Span, Spans};
use tui::widgets::{Block, Borders, List, ListItem, ListState};

/// Renders the TUI given the current state.
pub fn render<B: Backend>(
    frame: &mut Frame<B>,
    state: &UiState,
) {
    let size = frame.size();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Min(1)].as_ref())
        .split(size);

    let items: Vec<ListItem> = state
        .items
        .iter()
        .map(|(path, checked)| {
            let mark = if *checked { "[x]" } else { "[ ]" };
            let content = Spans::from(vec![
                Span::styled(mark, Style::default().fg(Color::Yellow)),
                Span::raw(" "),
                Span::raw(path.display().to_string()),
            ]);
            ListItem::new(content)
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(state.selected_idx));

    let list = List::new(items)
        .block(Block::default().title("Select Files").borders(Borders::ALL))
        .highlight_style(Style::default().bg(Color::Blue));

    frame.render_stateful_widget(list, chunks[0], &mut list_state);
}
