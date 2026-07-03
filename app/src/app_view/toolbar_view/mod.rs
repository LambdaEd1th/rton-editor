mod groups;

use dioxus::prelude::*;

use crate::components::{FileSelection, ToolbarGroup};
use crate::domain::{
    DropMarker, EditorMode, Status, ThemePreference, ToolbarDropTarget, ToolbarGroupId,
};
use crate::file_import::LoadedFileState;
use crate::i18n::{I18n, LanguageOption, Locale};
use rton_editor_core::TextFormat;

use groups::ToolbarGroupContent;

#[component]
pub(super) fn ToolbarView(
    i18n: I18n,
    toolbar_rows_snapshot: Vec<Vec<ToolbarGroupId>>,
    dragged_toolbar_group_id_snapshot: Option<ToolbarGroupId>,
    toolbar_drop_marker_snapshot: Option<DropMarker<ToolbarGroupId>>,
    active_mode_snapshot: Option<EditorMode>,
    active_file_label: String,
    compact_snapshot: bool,
    encrypt_snapshot: bool,
    theme_preference_snapshot: ThemePreference,
    locale_snapshot: Locale,
    language_options: Vec<LanguageOption>,
    line_wrapping_snapshot: bool,
    editor_search_panel_visible_snapshot: bool,
    can_undo_snapshot: bool,
    can_redo_snapshot: bool,
    loaded_files: Signal<Vec<LoadedFileState>>,
    next_loaded_file_id: Signal<usize>,
    file_selection: Signal<FileSelection>,
    encrypt_output: Signal<bool>,
    theme_preference: Signal<ThemePreference>,
    locale: Signal<Locale>,
    line_wrapping: Signal<bool>,
    editor_search_panel_visible: Signal<bool>,
    status: Signal<Status>,
    update_toolbar_drop_target: EventHandler<ToolbarDropTarget>,
    finish_toolbar_drag: EventHandler<()>,
    start_toolbar_group_drag: EventHandler<ToolbarGroupId>,
    open_native_files: EventHandler<()>,
    open_native_folder: EventHandler<()>,
    load_sample: EventHandler<()>,
    undo_edit: EventHandler<()>,
    redo_edit: EventHandler<()>,
    switch_mode: EventHandler<EditorMode>,
    on_compact_change: EventHandler<bool>,
    export_text: EventHandler<TextFormat>,
    parse_current: EventHandler<()>,
    export_rton: EventHandler<()>,
) -> Element {
    rsx! {
        header {
            class: if dragged_toolbar_group_id_snapshot.is_some() { "rton-toolbar dragging-toolbar" } else { "rton-toolbar" },
            for (row_index, row) in toolbar_rows_snapshot.iter().cloned().enumerate() {
                div {
                    key: "toolbar-row-{row_index}",
                    class: "rton-toolbar-row",
                    onmousemove: move |event| {
                        event.stop_propagation();
                        update_toolbar_drop_target.call(ToolbarDropTarget::RowEnd { row_index });
                    },
                    onmouseup: move |_| finish_toolbar_drag.call(()),
                    for group_id in row {
                        ToolbarGroup {
                            key: "{group_id.code()}",
                            id: group_id,
                            label: i18n.t(group_id.label_key()),
                            i18n,
                            dragging: dragged_toolbar_group_id_snapshot == Some(group_id),
                            drop_placement: toolbar_drop_marker_snapshot
                                .filter(|marker| marker.id == group_id)
                                .map(|marker| marker.placement),
                            on_drag_start: start_toolbar_group_drag,
                            on_drop_target: update_toolbar_drop_target,
                            on_drag_end: finish_toolbar_drag,
                            ToolbarGroupContent {
                                group_id,
                                i18n,
                                active_mode_snapshot,
                                active_file_label: active_file_label.clone(),
                                compact_snapshot,
                                encrypt_snapshot,
                                theme_preference_snapshot,
                                locale_snapshot,
                                language_options: language_options.clone(),
                                line_wrapping_snapshot,
                                editor_search_panel_visible_snapshot,
                                can_undo_snapshot,
                                can_redo_snapshot,
                                loaded_files,
                                next_loaded_file_id,
                                file_selection,
                                encrypt_output,
                                theme_preference,
                                locale,
                                line_wrapping,
                                editor_search_panel_visible,
                                status,
                                open_native_files,
                                open_native_folder,
                                load_sample,
                                undo_edit,
                                redo_edit,
                                switch_mode,
                                on_compact_change,
                                export_text,
                                parse_current,
                                export_rton
                            }
                        }
                    }
                }
            }
        }
    }
}
