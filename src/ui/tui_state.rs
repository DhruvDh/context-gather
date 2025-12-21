use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Shared UI state for file selection TUI
pub struct UiState {
    pub items: Vec<(PathBuf, bool)>,
    pub item_display: Vec<String>,
    pub ext_counts: HashMap<String, usize>,
    pub search_input: String,
    pub file_query_cache: String,
    pub filtered_files: Vec<usize>,
    pub extension_mode: bool,
    pub extension_items: Vec<(String, bool)>,
    pub extension_search: String,
    pub extension_query_cache: String,
    pub filtered_exts: Vec<usize>,
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
        let cwd = std::env::current_dir()
            .ok()
            .and_then(|p| dunce::canonicalize(p).ok());

        fn display_path(
            path: &Path,
            cwd: Option<&Path>,
        ) -> String {
            if let Some(cwd) = cwd
                && let Ok(stripped) = path.strip_prefix(cwd)
            {
                if stripped.as_os_str().is_empty() {
                    return ".".to_string();
                }
                return stripped.to_string_lossy().to_string();
            }
            path.display().to_string()
        }

        // Build items with initial checked state
        let items: Vec<(PathBuf, bool)> = paths
            .into_iter()
            .map(|p| (p.clone(), preselected.contains(&p)))
            .collect();
        let item_display: Vec<String> = items
            .iter()
            .map(|(p, _)| display_path(p, cwd.as_deref()))
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

        let items_len = items.len();
        let ext_items_len = ext_items.len();
        UiState {
            items,
            item_display,
            ext_counts,
            search_input: String::new(),
            file_query_cache: String::new(),
            filtered_files: (0..items_len).collect(),
            extension_mode: false,
            extension_items: ext_items,
            extension_search: String::new(),
            extension_query_cache: String::new(),
            filtered_exts: (0..ext_items_len).collect(),
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

    pub fn ensure_filtered_files(&mut self) {
        if self.search_input == self.file_query_cache {
            return;
        }
        let matcher = SkimMatcherV2::default();
        let mut entries: Vec<(usize, i64)> = if self.search_input.is_empty() {
            (0..self.items.len()).map(|idx| (idx, 0)).collect()
        } else {
            self.item_display
                .iter()
                .enumerate()
                .filter_map(|(idx, text)| {
                    matcher
                        .fuzzy_match(text, &self.search_input)
                        .map(|score| (idx, score))
                })
                .collect()
        };
        entries.sort_unstable_by_key(|&(_, score)| std::cmp::Reverse(score));
        self.filtered_files = entries.into_iter().map(|(idx, _)| idx).collect();
        self.file_query_cache = self.search_input.clone();
        if self.filtered_files.is_empty() {
            self.selected_idx = 0;
            self.scroll_offset = 0;
        } else {
            self.selected_idx = self
                .selected_idx
                .min(self.filtered_files.len().saturating_sub(1));
            self.scroll_offset = self.scroll_offset.min(self.selected_idx);
        }
    }

    pub fn ensure_filtered_exts(&mut self) {
        if self.extension_search == self.extension_query_cache {
            return;
        }
        let matcher = SkimMatcherV2::default();
        let mut entries: Vec<(usize, i64)> = if self.extension_search.is_empty() {
            (0..self.extension_items.len())
                .map(|idx| (idx, 0))
                .collect()
        } else {
            self.extension_items
                .iter()
                .enumerate()
                .filter_map(|(idx, (ext, _))| {
                    matcher
                        .fuzzy_match(ext, &self.extension_search)
                        .map(|score| (idx, score))
                })
                .collect()
        };
        entries.sort_unstable_by_key(|&(_, score)| std::cmp::Reverse(score));
        self.filtered_exts = entries.into_iter().map(|(idx, _)| idx).collect();
        self.extension_query_cache = self.extension_search.clone();
        if self.filtered_exts.is_empty() {
            self.ext_selected_idx = 0;
            self.ext_scroll_offset = 0;
        } else {
            self.ext_selected_idx = self
                .ext_selected_idx
                .min(self.filtered_exts.len().saturating_sub(1));
            self.ext_scroll_offset = self.ext_scroll_offset.min(self.ext_selected_idx);
        }
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
