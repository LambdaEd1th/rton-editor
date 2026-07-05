use std::sync::Arc;

use crate::components::{FileListItem, FileSelection, TabHeader, file_item_matches_search};
use crate::domain::{
    EditorTabState, TextBuffer, TextSearchMatch, find_text_search_result, text_search_status_text,
};
use crate::file_import::{LoadedFileState, build_file_list_items};
use crate::i18n::I18n;

pub(super) struct EditorSearchSnapshot {
    pub(super) matches: Vec<TextSearchMatch>,
    pub(super) match_count: usize,
    pub(super) status_text: String,
    pub(super) controls_disabled: bool,
}

pub(super) fn editor_search_snapshot(
    text_buffer: Option<&TextBuffer>,
    query: &str,
    case_sensitive: bool,
    match_index: usize,
    i18n: I18n,
) -> EditorSearchSnapshot {
    let Some(text_buffer) = text_buffer else {
        return empty_editor_search_snapshot(query, i18n);
    };
    let result = find_text_search_result(text_buffer.text.as_ref(), query, case_sensitive);
    let matches = result.matches;
    let match_count = matches.len();
    let current_index = if match_count == 0 {
        None
    } else {
        Some(match_index.min(match_count - 1))
    };
    let status_text =
        text_search_status_text(query, match_count, current_index, result.capped, i18n);

    EditorSearchSnapshot {
        matches,
        match_count,
        status_text,
        controls_disabled: match_count == 0,
    }
}

pub(super) fn empty_editor_search_snapshot(query: &str, i18n: I18n) -> EditorSearchSnapshot {
    EditorSearchSnapshot {
        matches: Vec::new(),
        match_count: 0,
        status_text: text_search_status_text(query, 0, None, false, i18n),
        controls_disabled: true,
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(super) struct FilePanelSnapshot {
    pub(super) all_items: Arc<Vec<FileListItem>>,
    pub(super) filtered_items: Arc<Vec<FileListItem>>,
    pub(super) empty_message: String,
    pub(super) selected_count: usize,
    pub(super) selected_visible_count: usize,
    pub(super) subtitle: String,
}

pub(super) fn tab_headers_for_tabs(tabs: &[EditorTabState]) -> Vec<TabHeader> {
    tabs.iter()
        .map(|tab| TabHeader {
            id: tab.id,
            file_name: tab.file_name.clone(),
            mode: tab.mode,
            dirty: tab.dirty,
            closeable: true,
        })
        .collect()
}

pub(super) fn file_panel_snapshot(
    loaded_files: &[LoadedFileState],
    tab_headers: &[TabHeader],
    active_tab_id: usize,
    search_query: &str,
    selection: &FileSelection,
    i18n: I18n,
) -> FilePanelSnapshot {
    let all_items = build_file_list_items(loaded_files, tab_headers, active_tab_id, i18n);
    let search_active = !search_query.trim().is_empty();
    let filtered_items = if !search_active {
        all_items.clone()
    } else {
        all_items
            .iter()
            .filter(|item| file_item_matches_search(item, search_query))
            .cloned()
            .collect::<Vec<_>>()
    };
    let empty_message = if !search_active {
        i18n.t("common-no-file")
    } else {
        i18n.t("file-list-no-matches")
    };
    let selected_count = selection.selected_count(&all_items);
    let selected_visible_count = filtered_items
        .iter()
        .filter(|item| selection.is_selected(item))
        .count();
    let subtitle = if !search_active {
        i18n.t_args(
            "file-list-selected-count",
            &[
                ("selected", selected_count.to_string()),
                ("total", all_items.len().to_string()),
            ],
        )
    } else {
        i18n.t_args(
            "file-list-match-count",
            &[
                ("visible", filtered_items.len().to_string()),
                ("total", all_items.len().to_string()),
                ("selected", selected_count.to_string()),
            ],
        )
    };

    FilePanelSnapshot {
        all_items: Arc::new(all_items),
        filtered_items: Arc::new(filtered_items),
        empty_message,
        selected_count,
        selected_visible_count,
        subtitle,
    }
}
