use std::collections::HashMap;
use std::path::PathBuf;

/// Shared UI state for file selection TUI
pub struct UiState {
    pub items: Vec<(PathBuf, bool)>,
    pub ext_counts: HashMap<String, usize>,
    pub search_input: String,
    pub extension_mode: bool,
    pub extension_items: Vec<(String, bool)>,
    pub extension_search: String,
    pub ext_selected_idx: usize,
    pub ext_scroll_offset: usize,
    pub reset_ext_on_toggle: bool,
    pub saved_search_input: String,
    pub selected_idx: usize,
    pub scroll_offset: usize,
}

impl UiState {
    /// Initialize state from paths and preselected list
    pub fn new(
        paths: Vec<PathBuf>,
        preselected: &[PathBuf],
    ) -> Self {
        // Build items with initial checked state
        let items: Vec<(PathBuf, bool)> = paths
            .into_iter()
            .map(|p| (p.clone(), preselected.contains(&p)))
            .collect();

        // Count extensions
        let mut ext_counts = HashMap::new();
        for (p, _) in &items {
            if let Some(ext) = p.extension().map(|e| format!(".{}", e.to_string_lossy())) {
                *ext_counts.entry(ext).or_insert(0) += 1;
            }
        }

        // Build extension items sorted by count
        let mut ext_keys: Vec<String> = ext_counts.keys().cloned().collect();
        ext_keys.sort();
        let ext_items: Vec<(String, bool)> = ext_keys.into_iter().map(|e| (e, false)).collect();

        UiState {
            items,
            ext_counts,
            search_input: String::new(),
            extension_mode: false,
            extension_items: ext_items,
            extension_search: String::new(),
            ext_selected_idx: 0,
            ext_scroll_offset: 0,
            reset_ext_on_toggle: true,
            saved_search_input: String::new(),
            selected_idx: 0,
            scroll_offset: 0,
        }
    }

    /// Return selected `PathBuf`s based on the checkbox state
    pub fn selected_paths(&self) -> Vec<PathBuf> {
        self.items
            .iter()
            .filter(|(_, checked)| *checked)
            .map(|(p, _)| p.clone())
            .collect()
    }
}

/// Adjust scroll offset and compute visible range
pub fn adjust_scroll_and_slice(
    selected_idx: &mut usize,
    scroll_offset: &mut usize,
    max_lines: usize,
    data_len: usize,
) -> (usize, usize) {
    if *selected_idx < *scroll_offset {
        *scroll_offset = *selected_idx;
    } else if *selected_idx >= *scroll_offset + max_lines {
        *scroll_offset = selected_idx.saturating_sub(max_lines).saturating_add(1);
    }
    let end_idx = (*scroll_offset + max_lines).min(data_len);
    (*scroll_offset, end_idx)
}
