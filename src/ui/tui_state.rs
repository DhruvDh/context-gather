use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Shared UI state for file selection TUI
pub struct UiState {
    pub items: Vec<(PathBuf, bool)>,
    pub item_display: Vec<String>,
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

        let preselected: HashSet<PathBuf> = preselected.iter().cloned().collect();
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

        // Build extension items sorted by count (desc), then name
        let mut ext_keys: Vec<(String, usize)> =
            ext_counts.iter().map(|(k, v)| (k.clone(), *v)).collect();
        ext_keys.sort_by(|(a, ac), (b, bc)| bc.cmp(ac).then_with(|| a.cmp(b)));
        let ext_items: Vec<(String, bool)> =
            ext_keys.into_iter().map(|(e, _)| (e, false)).collect();

        UiState {
            items,
            item_display,
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

pub fn filtered_files(state: &UiState) -> Vec<usize> {
    let matcher = SkimMatcherV2::default();
    let mut entries: Vec<(usize, i64)> = if state.search_input.is_empty() {
        (0..state.items.len()).map(|idx| (idx, 0)).collect()
    } else {
        state
            .item_display
            .iter()
            .enumerate()
            .filter_map(|(idx, text)| {
                matcher
                    .fuzzy_match(text, &state.search_input)
                    .map(|score| (idx, score))
            })
            .collect()
    };
    entries.sort_unstable_by_key(|&(_, score)| std::cmp::Reverse(score));
    entries.into_iter().map(|(idx, _)| idx).collect()
}

pub fn filtered_exts(state: &UiState) -> Vec<usize> {
    let matcher = SkimMatcherV2::default();
    let mut entries: Vec<(usize, i64)> = if state.extension_search.is_empty() {
        (0..state.extension_items.len())
            .map(|idx| (idx, 0))
            .collect()
    } else {
        state
            .extension_items
            .iter()
            .enumerate()
            .filter_map(|(idx, (ext, _))| {
                matcher
                    .fuzzy_match(ext, &state.extension_search)
                    .map(|score| (idx, score))
            })
            .collect()
    };
    entries.sort_unstable_by_key(|&(_, score)| std::cmp::Reverse(score));
    entries.into_iter().map(|(idx, _)| idx).collect()
}

pub fn clamp_selection(
    selected_idx: &mut usize,
    scroll_offset: &mut usize,
    list_len: usize,
) {
    if list_len == 0 {
        *selected_idx = 0;
        *scroll_offset = 0;
    } else {
        *selected_idx = (*selected_idx).min(list_len.saturating_sub(1));
        *scroll_offset = (*scroll_offset).min(*selected_idx);
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
