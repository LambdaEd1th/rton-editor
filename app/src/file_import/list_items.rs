use rton_editor_core::format_bytes;
use std::collections::{HashMap, HashSet};

use crate::components::{FileListItem, TabHeader};
use crate::domain::{
    file_list_file_key, file_list_tab_key, leaf_display_name, loadable_file_kind_label,
};
use crate::i18n::I18n;

use super::state::LoadedFileState;

pub(crate) fn build_file_list_items(
    loaded_files: &[LoadedFileState],
    tabs: &[TabHeader],
    active_tab_id: usize,
    i18n: I18n,
) -> Vec<FileListItem> {
    let mut items = Vec::new();
    let tabs_by_id = tabs
        .iter()
        .map(|tab| (tab.id, tab))
        .collect::<HashMap<_, _>>();
    let linked_tab_ids = loaded_files
        .iter()
        .filter_map(|file| file.tab_id)
        .collect::<HashSet<_>>();

    for file in loaded_files {
        let tab = file.tab_id.and_then(|id| tabs_by_id.get(&id).copied());
        let path = tab
            .map(|tab| tab.file_name.clone())
            .unwrap_or_else(|| file.display_name.clone());
        let name = leaf_display_name(&path);
        let detail = if let Some(tab) = tab {
            let size = file
                .size
                .map(format_bytes)
                .unwrap_or_else(|| i18n.t("common-empty"));
            format!("{size} · {}", tab.mode.label())
        } else {
            i18n.t_args(
                "file-list-closed-detail",
                &[
                    (
                        "size",
                        file.size
                            .map(format_bytes)
                            .unwrap_or_else(|| i18n.t("common-empty")),
                    ),
                    ("kind", loadable_file_kind_label(&file.display_name)),
                ],
            )
        };

        items.push(FileListItem {
            key: file_list_file_key(file.id),
            file_id: Some(file.id),
            tab_id: tab.map(|tab| tab.id),
            path,
            name,
            detail,
            active: tab.is_some_and(|tab| tab.id == active_tab_id),
            dirty: tab.is_some_and(|tab| tab.dirty),
            closeable: tab.is_some_and(|tab| tab.closeable),
        });
    }

    for tab in tabs {
        if linked_tab_ids.contains(&tab.id) {
            continue;
        }
        let path = tab.file_name.clone();
        items.push(FileListItem {
            key: file_list_tab_key(tab.id),
            file_id: None,
            tab_id: Some(tab.id),
            name: leaf_display_name(&path),
            path,
            detail: tab.mode.label().to_string(),
            active: tab.id == active_tab_id,
            dirty: tab.dirty,
            closeable: tab.closeable,
        });
    }

    items
}
