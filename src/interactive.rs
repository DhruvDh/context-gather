use std::path::PathBuf;
use std::collections::{HashSet, HashMap};
use std::cmp::min;
use std::fmt::Write as FmtWrite;

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

    // ---------------------------
    // EXTENSION MODE STATE
    // ---------------------------
    let mut extension_mode = false;
    
    // Build a map: extension -> how many files have this extension
    let mut ext_counts: HashMap<String, usize> = HashMap::new();
    for (p, _) in &items {
        if let Some(ext) = p.extension().map(|e| format!(".{}", e.to_string_lossy())) {
            *ext_counts.entry(ext).or_insert(0) += 1;
        }
    }

    // extension_items: (ext_string, checked)
    let mut extension_items: Vec<(String, bool)> = {
        let mut exts: Vec<String> = items
            .iter()
            .filter_map(|(p, _)| p.extension().map(|e| format!(".{}", e.to_string_lossy())))
            .collect();
        exts.sort();
        exts.dedup();
        // Now we sort by frequency in descending order
        let mut with_counts: Vec<(String, usize)> = exts.into_iter().map(|e| {
            let c = ext_counts.get(&e).cloned().unwrap_or(0);
            (e, c)
        }).collect();
        // Sort by c descending
        with_counts.sort_by_key(|&(_, c)| std::cmp::Reverse(c));

        // Then build extension_items from that
        with_counts.into_iter()
            .map(|(e, _freq)| (e, false))
            .collect()
    };
    // We'll also maintain an extension search input
    let mut extension_search = String::new();
    // Index for which extension is highlighted in extension mode
    let mut ext_selected_idx = 0usize;
    
    // Build a map: extension -> how many files have this extension
    let mut ext_counts: HashMap<String, usize> = HashMap::new();
    for (p, _) in &items {
        if let Some(ext) = p.extension().map(|e| format!(".{}", e.to_string_lossy())) {
            *ext_counts.entry(ext).or_insert(0) += 1;
        }
    }
    // Scrolling offset for extension list
    let mut ext_scroll_offset = 0usize;
    // Control whether extension selections reset when toggling mode
    let reset_ext_on_toggle = true;
    // Store the old search_input here if we want to restore
    let mut saved_search_input = String::new();

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
        // If NOT in extension mode, compute filtered files
        let filtered = if !extension_mode {
            filter_items(&items, &search_input)
        } else {
            // if extension_mode is active, we skip "file list" logic
            vec![]
        };

        // If we *are* in extension mode, let's do fuzzy matching over extension_items
        let ext_filtered = if extension_mode {
            // We'll create a local function that returns the "display string"
            let display_ext = |ext: &str| {
                if let Some(count) = ext_counts.get(ext) {
                    let mut s = String::new();
                    // e.g. ".rs (12)"
                    let _ = write!(s, "{} ({})", ext, count);
                    s
                } else {
                    ext.to_owned()
                }
            };

            let to_list = |(i, e, c): (usize, String, bool)| -> (usize, String, bool) {
                // We'll transform e into e + " (count)"
                let shown = display_ext(&e);
                (i, shown, c)
            };

            let raw: Vec<(usize, String, bool)> = if extension_search.is_empty() {
                extension_items.iter()
                    .enumerate()
                    .map(|(i, (e, c))| (i, e.clone(), *c))
                    .collect()
            } else {
                let matcher = SkimMatcherV2::default();
                let mut results = Vec::new();
                for (i, (ext, checked)) in extension_items.iter().enumerate() {
                    if let Some(score) = matcher.fuzzy_match(ext, &extension_search) {
                        results.push((i, ext.clone(), *checked, score));
                    }
                }
                results.sort_by_key(|&(_, _, _, score)| -score);
                results.into_iter().map(|(i, e, c, _)| (i, e, c)).collect()
            };

            // transform them so we show "ext (count)" in UI
            raw.into_iter().map(to_list).collect()
        } else {
            // if not in extension mode, no ext list
            vec![]
        };

        // Sizing for the main list area
        let size = terminal.size()?;

        // If in extension mode => ext_selected_idx logic
        if extension_mode && !ext_filtered.is_empty() && ext_selected_idx >= ext_filtered.len() {
            ext_selected_idx = ext_filtered.len().saturating_sub(1);
        }

        // If NOT extension mode => normal selected_idx logic
        if !extension_mode {
            if filtered.is_empty() {
                selected_idx = 0;
            } else if selected_idx >= filtered.len() {
                selected_idx = filtered.len() - 1;
            }
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

            // Depending on extension_mode => we show a different "title" & input
            let (title, input_str) = if extension_mode {
                ("Extensions (Ctrl+E to exit, Enter to confirm)", &extension_search)
            } else {
                ("Fuzzy Search", &search_input)
            };

            let search_bar = Paragraph::new(input_str.as_str())
                .block(Block::default().title(title).borders(Borders::ALL));
            f.render_widget(search_bar, chunks[0]);

            // If extension_mode => display extension list. Otherwise => display file list
            if extension_mode {
                // We'll do the same "scroll offset" pattern => ext_scroll_offset
                let list_area = chunks[1];
                let max_lines = list_area.height.saturating_sub(2) as usize;

                if ext_selected_idx < ext_scroll_offset {
                    ext_scroll_offset = ext_selected_idx;
                } else if ext_selected_idx >= ext_scroll_offset + max_lines {
                    ext_scroll_offset = ext_selected_idx.saturating_sub(max_lines).saturating_add(1);
                }
                let end_idx = min(ext_scroll_offset + max_lines, ext_filtered.len());
                let slice = &ext_filtered[ext_scroll_offset..end_idx];

                // build items
                let list_items: Vec<ListItem> = slice
                    .iter()
                    .enumerate()
                    .map(|(i, (_orig_idx, ext_string, is_checked))| {
                        let displayed_idx = i + ext_scroll_offset;
                        let mark = if *is_checked { "[x]" } else { "[ ]" };
                        let line = format!("{} {}", mark, ext_string);
                        if displayed_idx == ext_selected_idx {
                            ListItem::new(Spans::from(vec![Span::styled(
                                line,
                                Style::default().fg(Color::Yellow),
                            )]))
                        } else {
                            ListItem::new(Spans::from(line))
                        }
                    })
                    .collect();

                let ext_list = List::new(list_items)
                    .block(Block::default().title("Extensions").borders(Borders::ALL));
                f.render_widget(ext_list, chunks[1]);
            } else {
                // normal file list
                let list_area = chunks[1];
                let max_lines = list_area.height.saturating_sub(2) as usize;
                if selected_idx < scroll_offset {
                    scroll_offset = selected_idx;
                } else if selected_idx >= scroll_offset + max_lines {
                    scroll_offset = selected_idx.saturating_sub(max_lines).saturating_add(1);
                }
                let end_idx = (scroll_offset + max_lines).min(filtered.len());
                let visible_slice = &filtered[scroll_offset..end_idx];

                let list_items: Vec<ListItem> = visible_slice
                    .iter()
                    .enumerate()
                    .map(|(i, (_idx_p, path, checked))| {
                        let displayed_idx = i + scroll_offset;
                        let mark = if *checked { "[x]" } else { "[ ]" };
                        let line = format!("{} {}", mark, path.display());
                        if displayed_idx == selected_idx {
                            ListItem::new(Spans::from(vec![Span::styled(
                                line,
                                Style::default().fg(Color::Yellow),
                            )]))
                        } else {
                            ListItem::new(Spans::from(line))
                        }
                    })
                    .collect();

                let files_list = List::new(list_items)
                    .block(Block::default().title("Files").borders(Borders::ALL));
                f.render_widget(files_list, chunks[1]);
            }
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
                // Toggle extension mode
                (KeyCode::Char('e'), KeyModifiers::CONTROL) => {
                    if !extension_mode {
                        // We are ENTERING extension mode
                        // Save the old search input in case we want to restore it
                        saved_search_input = search_input.clone();
                        // Clear the main search and rely on extension_search now
                        search_input.clear();
                        extension_mode = true;
                        if reset_ext_on_toggle {
                            // Clear all extension checks & extension search
                            for (_, is_checked) in extension_items.iter_mut() {
                                *is_checked = false;
                            }
                            extension_search.clear();
                        }
                    } else {
                        // We are EXITING extension mode
                        // Optionally restore the old search
                        search_input = saved_search_input.clone();
                        extension_mode = false;
                    }
                }

                // If extension_mode => up/down move ext_selected_idx
                (KeyCode::Up, _) if extension_mode => {
                    ext_selected_idx = ext_selected_idx.saturating_sub(1);
                }
                (KeyCode::Down, _) if extension_mode => {
                    if !ext_filtered.is_empty() && ext_selected_idx < ext_filtered.len() - 1 {
                        ext_selected_idx += 1;
                    }
                }
                // Space toggles extension check if extension_mode
                (KeyCode::Char(' '), _) if extension_mode => {
                    if let Some((orig_idx, _ext, _)) = ext_filtered.get(ext_selected_idx) {
                        extension_items[*orig_idx].1 = !extension_items[*orig_idx].1;
                    }
                }
                // If extension_mode => pressing Enter => apply extension checks to items
                (KeyCode::Enter, _) if extension_mode => {
                    // gather all extension_items that are checked
                    let chosen_exts: HashSet<String> = extension_items
                        .iter()
                        .filter_map(|(ext, c)| if *c { Some(ext.clone()) } else { None })
                        .collect();

                    // Instead, let's REPLACE the checks for those file extensions only:
                    // i.e. if an extension is now unchecked, we un-check matching items
                    // So first gather ALL known ext -> are they chosen?
                    let union_mode = false; // set to true if user wants union

                    let all_known_exts: HashSet<String> = extension_items
                        .iter()
                        .map(|(ext, _)| ext.clone())
                        .collect();

                    for (p, checked) in items.iter_mut() {
                        if let Some(pext) = p.extension().map(|e| format!(".{}", e.to_string_lossy())) {
                            if all_known_exts.contains(&pext) {
                                if union_mode {
                                    // Just set to true if chosen_exts has it, else keep old
                                    if chosen_exts.contains(&pext) {
                                        *checked = true;
                                    }
                                } else {
                                    // The existing logic: if it's in known exts, override with whether chosen_exts has it
                                    *checked = chosen_exts.contains(&pext);
                                }
                            }
                        }
                    }
                    // exit extension mode
                    extension_mode = false;
                }
                // If extension_mode => typed char goes to extension_search
                (KeyCode::Backspace, _) if extension_mode => {
                    extension_search.pop();
                }
                (KeyCode::Char(c), _) if extension_mode => {
                    extension_search.push(c);
                }
                // Otherwise, if not in extension mode, Enter finalizes
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
                // If extension_mode, remove from extension_input; else from search_input
                (KeyCode::Backspace, _) => {
                    if extension_mode {
                        extension_search.pop();
                    } else {
                        search_input.pop();
                    }
                }
                // Finally, add typed character
                (KeyCode::Char('r'), KeyModifiers::CONTROL) => {
                    // Uncheck everything
                    for (_, checked) in items.iter_mut() {
                        *checked = false;
                    }
                    for (_, checked) in extension_items.iter_mut() {
                        *checked = false;
                    }
                }
                (KeyCode::Char(c), _) => {
                    if extension_mode {
                        extension_search.push(c);
                    } else {
                        search_input.push(c);
                    }
                }
                _ => {}
            }
        }
    }
}
