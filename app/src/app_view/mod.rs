use crate::i18n::{self, I18n};
use dioxus::prelude::*;
use rton_editor_core::{BinaryEncoding, EncodeOptions, TextFormat};

use crate::app_actions::*;
use crate::app_i18n::*;
use crate::app_layout::StatusBar;
use crate::components::{
    PanelResizeDrag, PanelResizeHandle, PanelSide, TabStrip, file_path_matches_scope,
};
use crate::domain::{
    BatchExportMode, ByteDocument, EditorMode, HexEdit, Status, TextBuffer, TextRangeReplacement,
    TextSearchMatch, Tone, default_expanded_paths, empty_tree_rows,
};
use crate::file_import::*;

mod editor_stage;
mod effects;
mod file_panel;
mod handlers;
mod index_panel;
mod signals;
mod snapshot;
mod toolbar_view;
mod workspace_drop;
use editor_stage::{EditorStage, EditorStageTab};
#[cfg(target_arch = "wasm32")]
use effects::use_web_i18n_loader;
use effects::{RtonOutputSize, use_rton_output_size_effect};
use file_panel::FilePanel;
use handlers::{
    commit_panel_resize_width, finish_workspace_drag_state, start_tab_drag_if_needed,
    start_toolbar_group_drag_state, start_workspace_panel_resize_preview,
    update_tab_drop_marker_for_drag, update_toolbar_drop_marker_for_drag,
};
use index_panel::IndexPanel;
use signals::{AppSignals, use_app_signals};
use snapshot::{
    editor_search_snapshot, empty_editor_search_snapshot, file_panel_snapshot, tab_headers_for_tabs,
};
use toolbar_view::ToolbarView;
use workspace_drop::handle_workspace_file_drop;
const APP_CSS: &str = include_str!("../../assets/style.css");
const _: Asset = asset!("/assets/i18n", AssetOptions::folder());
const _: Asset = asset!("/assets/worker", AssetOptions::folder());

#[component]
pub(crate) fn App() -> Element {
    #[cfg(not(target_arch = "wasm32"))]
    let _loaded_i18n_count = use_hook(load_i18n_sources);
    let initial_locale_snapshot = initial_locale();

    let AppSignals {
        tabs,
        loaded_files,
        active_tab_id,
        next_tab_id,
        next_loaded_file_id,
        compact_output,
        encrypt_output,
        mut dragging_files,
        theme_preference,
        locale,
        i18n_revision,
        #[cfg(target_arch = "wasm32")]
        i18n_loaded,
        line_wrapping,
        mut editor_search_panel_visible,
        mut editor_search_text,
        mut editor_replace_text,
        mut editor_search_case_sensitive,
        mut editor_search_match_index,
        editor_search_focus_token,
        mut file_search_query,
        file_selection,
        next_jump_id,
        text_jump_target,
        hex_jump_target,
        left_panel_width,
        right_panel_width,
        mut panel_resize_drag,
        dragged_tab_id,
        tab_drop_marker,
        toolbar_rows,
        dragged_toolbar_group_id,
        toolbar_drop_marker,
        mut status,
    } = use_app_signals(initial_locale_snapshot);

    #[cfg(target_arch = "wasm32")]
    use_web_i18n_loader(
        locale,
        status,
        i18n_loaded,
        i18n_revision,
        initial_locale_snapshot,
    );

    #[cfg(target_arch = "wasm32")]
    if !*i18n_loaded.read() {
        return rsx! {
            style { {APP_CSS} }
            div { class: "rton-app rton-loading-screen", "Loading localization..." }
        };
    }

    let active_id_snapshot = *active_tab_id.read();
    let rton_output_size = use_signal(|| None::<RtonOutputSize>);
    let rton_output_size_generation = use_signal(|| 0_u64);
    let (
        active_tab_snapshot,
        active_stage_tab_snapshot,
        active_doc_snapshot,
        active_byte_doc_snapshot,
        active_file_name_snapshot,
        active_mode_snapshot,
        active_search,
        value_search_result,
        selected_path_snapshot,
        tree_rows,
        expanded_paths_snapshot,
        can_undo_snapshot,
        can_redo_snapshot,
        active_text_buffer_snapshot,
        tab_headers,
    ) = {
        let tabs_snapshot = tabs.read();
        let active_tab_snapshot = tabs_snapshot
            .iter()
            .find(|tab| tab.id == active_id_snapshot)
            .or_else(|| tabs_snapshot.first());
        let active_search = active_tab_snapshot
            .map(|tab| tab.search_query.clone())
            .unwrap_or_default();
        let value_search_result = if active_search.trim().is_empty() {
            None
        } else {
            active_tab_snapshot.and_then(|tab| tab.search_result.clone())
        };
        (
            active_tab_snapshot.cloned(),
            active_tab_snapshot.map(|tab| EditorStageTab {
                id: tab.id,
                mode: tab.mode,
                text_buffer: tab.text_buffer.clone(),
                text_state: tab.text_state.clone(),
                task_state: tab.task_state.clone(),
            }),
            active_tab_snapshot.and_then(|tab| tab.doc.clone()),
            active_tab_snapshot.and_then(|tab| tab.byte_doc.clone()),
            active_tab_snapshot.map(|tab| tab.file_name.clone()),
            active_tab_snapshot.map(|tab| tab.mode),
            active_search,
            value_search_result,
            active_tab_snapshot
                .map(|tab| tab.selected_path.clone())
                .unwrap_or_else(|| "$".to_string()),
            active_tab_snapshot
                .map(|tab| tab.tree_rows.clone())
                .unwrap_or_else(empty_tree_rows),
            active_tab_snapshot
                .map(|tab| tab.expanded_paths.clone())
                .unwrap_or_else(default_expanded_paths),
            active_tab_snapshot.is_some_and(tab_can_undo),
            active_tab_snapshot.is_some_and(tab_can_redo),
            active_tab_snapshot
                .filter(|tab| tab.mode.text_format().is_some())
                .and_then(|tab| tab.text_buffer.clone()),
            tab_headers_for_tabs(&tabs_snapshot),
        )
    };
    let compact_snapshot = *compact_output.read();
    let encrypt_snapshot = *encrypt_output.read();
    use_rton_output_size_effect(
        active_tab_snapshot.clone(),
        compact_snapshot,
        encrypt_snapshot,
        rton_output_size,
        rton_output_size_generation,
    );
    let rton_output_size_snapshot = *rton_output_size.read();
    let dragging_snapshot = *dragging_files.read();
    let theme_preference_snapshot = *theme_preference.read();
    let locale_snapshot = *locale.read();
    let _i18n_revision_snapshot = *i18n_revision.read();
    let i18n = I18n::new(locale_snapshot);
    let language_options = i18n::language_options(i18n);
    let line_wrapping_snapshot = *line_wrapping.read();
    let editor_search_panel_visible_snapshot = *editor_search_panel_visible.read();
    let editor_search_text_snapshot = editor_search_text.read().clone();
    let editor_replace_text_snapshot = editor_replace_text.read().clone();
    let editor_search_case_sensitive_snapshot = *editor_search_case_sensitive.read();
    let editor_search_match_index_snapshot = *editor_search_match_index.read();
    let editor_search_focus_token_snapshot = *editor_search_focus_token.read();
    let should_search_editor_text =
        editor_search_panel_visible_snapshot && !editor_search_text_snapshot.is_empty();
    let editor_search = if should_search_editor_text {
        editor_search_snapshot(
            active_text_buffer_snapshot.as_deref(),
            &editor_search_text_snapshot,
            editor_search_case_sensitive_snapshot,
            editor_search_match_index_snapshot,
            i18n,
        )
    } else {
        empty_editor_search_snapshot(&editor_search_text_snapshot, i18n)
    };
    let text_search_jump = if should_search_editor_text {
        active_text_buffer_snapshot
            .as_deref()
            .and_then(|text_buffer| {
                current_text_search_jump(
                    text_buffer,
                    &editor_search.matches,
                    editor_search_match_index_snapshot,
                    editor_search_focus_token_snapshot,
                )
            })
    } else {
        None
    };
    let active_text_jump_target = text_search_jump.or(*text_jump_target.read());
    let file_search_snapshot = file_search_query.read().clone();
    let file_selection_snapshot = file_selection.read().clone();
    let file_panel_memo = use_memo(move || {
        let loaded_files_snapshot = loaded_files.read();
        let tabs_snapshot = tabs.read();
        let tab_headers = tab_headers_for_tabs(&tabs_snapshot);
        let active_id = *active_tab_id.read();
        let file_search = file_search_query.read().clone();
        let selection = file_selection.read().clone();
        let locale_snapshot = *locale.read();
        let _i18n_revision_snapshot = *i18n_revision.read();
        file_panel_snapshot(
            &loaded_files_snapshot,
            &tab_headers,
            active_id,
            &file_search,
            &selection,
            I18n::new(locale_snapshot),
        )
    });
    let file_panel = file_panel_memo.read().clone();
    let left_panel_width_snapshot = *left_panel_width.read();
    let right_panel_width_snapshot = *right_panel_width.read();
    let panel_resize_drag_snapshot = *panel_resize_drag.read();
    let panel_resizing_snapshot = panel_resize_drag_snapshot.is_some();
    let dragged_tab_id_snapshot = *dragged_tab_id.read();
    let tab_drop_marker_snapshot = *tab_drop_marker.read();
    let toolbar_rows_snapshot = toolbar_rows.read().clone();
    let dragged_toolbar_group_id_snapshot = *dragged_toolbar_group_id.read();
    let toolbar_drop_marker_snapshot = *toolbar_drop_marker.read();
    let status_snapshot = status.read().clone();
    let no_file_label = i18n.t("common-no-file");
    let active_file_label = active_file_name_snapshot.unwrap_or_else(|| no_file_label.clone());
    let active_text_byte_count_snapshot = active_stage_tab_snapshot
        .as_ref()
        .and_then(|tab| tab.text_buffer.as_ref())
        .map(|buffer| buffer.byte_count());
    let input_value_label = file_input_summary(
        active_stage_tab_snapshot.is_some(),
        active_byte_doc_snapshot.as_ref(),
        active_text_byte_count_snapshot,
        i18n,
    );
    let output_value_label = file_output_summary(FileOutputSummaryInput {
        has_active_file: active_stage_tab_snapshot.is_some(),
        active_tab_id: active_tab_snapshot.as_ref().map(|tab| tab.id),
        mode: active_mode_snapshot,
        byte_doc: active_byte_doc_snapshot.as_ref(),
        text_byte_count: active_text_byte_count_snapshot,
        compact: compact_snapshot,
        encrypted: encrypt_snapshot,
        rton_output_size: rton_output_size_snapshot,
        i18n,
    });
    let workspace_style = format!(
        "--rton-left-panel-width: {left}px; --rton-right-panel-width: {right}px;",
        left = left_panel_width_snapshot,
        right = right_panel_width_snapshot
    );
    let parse_current = move |_| {
        validate_active_tab(tabs, active_tab_id, status, i18n);
    };

    let load_sample = move |_| {
        load_sample_tab(next_tab_id, tabs, active_tab_id, status, i18n);
    };

    let activate_tab = move |id: usize| {
        activate_tab_by_id(id, tabs, active_tab_id, status, i18n);
    };

    let open_loaded_file = move |file_id: usize| {
        open_loaded_file_by_id(
            file_id,
            loaded_files,
            next_tab_id,
            tabs,
            active_tab_id,
            status,
            i18n,
        );
    };

    let close_tab = move |id: usize| {
        close_tab_by_id(id, tabs, active_tab_id, status, i18n);
        unlink_loaded_file_tab(loaded_files, id);
    };

    let remove_file_list_item = move |key: String| {
        if let Some(name) = remove_file_list_item_by_key(key, loaded_files, tabs, active_tab_id) {
            status.set(Status::new(
                i18n.t_args("status-removed-file", &[("name", name)]),
                Tone::Info,
            ));
        }
    };

    let remove_file_list_path = {
        let file_list_items = file_panel.filtered_items.clone();
        move |path: String| {
            let keys = file_list_items
                .iter()
                .filter(|item| file_path_matches_scope(&item.path, &path))
                .map(|item| item.key.clone())
                .collect::<Vec<_>>();
            let count = remove_file_list_items_by_keys(keys, loaded_files, tabs, active_tab_id);
            if count > 0 {
                status.set(Status::new(
                    i18n.t_args(
                        "status-removed-path",
                        &[("name", path), ("count", count.to_string())],
                    ),
                    Tone::Info,
                ));
            }
        }
    };

    let switch_mode = move |next_mode: EditorMode| {
        switch_active_mode(
            next_mode,
            rton_display_encode_options(*compact_output.read()),
            tabs,
            active_tab_id,
            status,
            i18n,
        );
    };

    let handle_compact_change = move |compact: bool| {
        update_compact_output(compact, compact_output, tabs, active_tab_id, status, i18n);
    };

    let select_all_visible_files = {
        let file_search_snapshot = file_search_snapshot.clone();
        move |_| {
            select_visible_files(&file_search_snapshot, file_selection, status, i18n);
        }
    };

    let clear_selected_files = {
        let file_search_snapshot = file_search_snapshot.clone();
        move |_| {
            clear_visible_file_selection(&file_search_snapshot, file_selection, status, i18n);
        }
    };

    let toggle_selected_file = move |(key, checked): (String, bool)| {
        toggle_selected_file_key(key, checked, file_selection);
    };

    let toggle_selected_path = move |(path, checked): (String, bool)| {
        toggle_selected_file_path(path, checked, file_selection);
    };

    let batch_export_selected: EventHandler<BatchExportMode> = {
        let file_list_items = file_panel.all_items.clone();
        EventHandler::new(move |mode: BatchExportMode| {
            batch_export_selected_file_items(
                mode,
                file_list_items.clone(),
                file_selection,
                tabs,
                loaded_files,
                compact_output,
                encrypt_output,
                status,
                i18n,
            );
        })
    };

    let update_hex_edit = move |edits: Vec<HexEdit>| {
        update_active_hex_edits(edits, tabs, active_tab_id);
    };

    let update_virtual_text_range = move |replacement: TextRangeReplacement| {
        update_active_text_range(replacement, tabs, active_tab_id);
    };

    let undo_edit: EventHandler<()> = EventHandler::new(move |_| {
        undo_active_edit(tabs, active_tab_id);
    });

    let redo_edit: EventHandler<()> = EventHandler::new(move |_| {
        redo_active_edit(tabs, active_tab_id);
    });

    let go_to_previous_editor_match = move |_| {
        go_to_previous_text_match(editor_search.match_count, editor_search_match_index);
        request_text_search_focus(editor_search.match_count, editor_search_focus_token);
    };

    let go_to_next_editor_match = move |_| {
        go_to_next_text_match(editor_search.match_count, editor_search_match_index);
        request_text_search_focus(editor_search.match_count, editor_search_focus_token);
    };

    let replace_current_editor_match = {
        let matches = editor_search.matches.clone();
        let replacement = editor_replace_text_snapshot.clone();
        move |_| {
            replace_current_text_match(
                &matches,
                &replacement,
                tabs,
                active_tab_id,
                editor_search_match_index,
            );
        }
    };

    let replace_all_editor_matches = {
        let query = editor_search_text_snapshot.clone();
        let replacement = editor_replace_text_snapshot.clone();
        let case_sensitive = editor_search_case_sensitive_snapshot;
        move |_| {
            replace_all_text_matches_in_editor(
                &query,
                &replacement,
                case_sensitive,
                tabs,
                active_tab_id,
                editor_search_match_index,
            );
        }
    };

    let handle_editor_search_key = move |event: KeyboardEvent| {
        if handle_text_find_key(
            event,
            editor_search.match_count,
            editor_search_panel_visible,
            editor_search_match_index,
        ) {
            request_text_search_focus(editor_search.match_count, editor_search_focus_token);
        }
    };

    let handle_editor_replace_key = {
        let matches = editor_search.matches.clone();
        let replacement = editor_replace_text_snapshot.clone();
        move |event: KeyboardEvent| {
            handle_text_replace_key(
                event,
                &matches,
                &replacement,
                editor_search_panel_visible,
                tabs,
                active_tab_id,
                editor_search_match_index,
            );
        }
    };

    let handle_panel_resize_start = move |drag: PanelResizeDrag| {
        panel_resize_drag.set(Some(drag));
        start_workspace_panel_resize_preview(drag);
    };

    let handle_workspace_mouse_up = move |event: MouseEvent| {
        commit_panel_resize_width(
            event,
            panel_resize_drag,
            left_panel_width,
            right_panel_width,
        );
        finish_workspace_drag_state(
            panel_resize_drag,
            tabs,
            dragged_tab_id,
            tab_drop_marker,
            toolbar_rows,
            dragged_toolbar_group_id,
            toolbar_drop_marker,
        );
    };

    let start_tab_drag = move |id: usize| {
        start_tab_drag_if_needed(id, tabs, dragged_tab_id, tab_drop_marker);
    };

    let update_tab_drop_marker = move |marker| {
        update_tab_drop_marker_for_drag(marker, dragged_tab_id, tab_drop_marker);
    };

    let finish_tab_drag = move |_| {
        finish_tab_drag_state(tabs, dragged_tab_id, tab_drop_marker);
    };

    let start_toolbar_group_drag = move |id| {
        start_toolbar_group_drag_state(id, dragged_toolbar_group_id, toolbar_drop_marker);
    };

    let update_toolbar_drop_target = move |target| {
        update_toolbar_drop_marker_for_drag(
            target,
            toolbar_rows,
            dragged_toolbar_group_id,
            toolbar_drop_marker,
        );
    };

    let finish_toolbar_drag = move |_| {
        finish_toolbar_drag_state(toolbar_rows, dragged_toolbar_group_id, toolbar_drop_marker);
    };

    let export_rton = move |_| {
        export_active_rton(
            tabs,
            active_tab_id,
            compact_output,
            encrypt_output,
            status,
            i18n,
        );
    };

    let export_text = move |format: TextFormat| {
        export_active_text(format, tabs, active_tab_id, status, i18n);
    };

    let open_native_files = move |_| {
        open_native_files_dialog(
            loaded_files,
            next_loaded_file_id,
            file_selection,
            status,
            i18n,
        );
    };

    let open_native_folder = move |_| {
        open_native_folder_dialog(
            loaded_files,
            next_loaded_file_id,
            file_selection,
            status,
            i18n,
        );
    };

    rsx! {
        document::Stylesheet {
            href: asset!("/assets/tailwind.css")
        }
        style { {APP_CSS} }
        main {
            class: theme_preference_snapshot.shell_class(),
            onmouseup: handle_workspace_mouse_up,
            ToolbarView {
                i18n,
                toolbar_rows_snapshot: toolbar_rows_snapshot.clone(),
                dragged_toolbar_group_id_snapshot,
                toolbar_drop_marker_snapshot,
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
                update_toolbar_drop_target: EventHandler::new(update_toolbar_drop_target),
                finish_toolbar_drag: EventHandler::new(finish_toolbar_drag),
                start_toolbar_group_drag: EventHandler::new(start_toolbar_group_drag),
                open_native_files: EventHandler::new(move |_| open_native_files(())),
                open_native_folder: EventHandler::new(move |_| open_native_folder(())),
                load_sample: EventHandler::new(move |_| load_sample(())),
                undo_edit,
                redo_edit,
                switch_mode: EventHandler::new(switch_mode),
                on_compact_change: EventHandler::new(handle_compact_change),
                export_text: EventHandler::new(export_text),
                parse_current: EventHandler::new(move |_| parse_current(())),
                export_rton: EventHandler::new(move |_| export_rton(()))
            }

            div {
                class: if dragging_snapshot { "rton-workspace-shell dragging-files" } else { "rton-workspace-shell" },
                style: "{workspace_style}",
                ondragover: move |event| {
                    event.prevent_default();
                    dragging_files.set(true);
                },
                ondragleave: move |_| {
                    dragging_files.set(false);
                },
                ondrop: move |event| async move {
                    handle_workspace_file_drop(
                        event,
                        dragging_files,
                        loaded_files,
                        next_loaded_file_id,
                        file_selection,
                        status,
                        i18n,
                    )
                    .await;
                },
                TabStrip {
                    tabs: tab_headers.clone(),
                    active_tab_id: active_id_snapshot,
                    dragged_tab_id: dragged_tab_id_snapshot,
                    drop_marker: tab_drop_marker_snapshot,
                    i18n,
                    on_activate: activate_tab,
                    on_close: close_tab,
                    on_drag_start: start_tab_drag,
                    on_drop_marker: update_tab_drop_marker,
                    on_drag_end: finish_tab_drag
                }

                section { class: "rton-main-content",
                    FilePanel {
                        i18n,
                        file_list_subtitle: file_panel.subtitle.clone(),
                        file_search: file_search_snapshot.clone(),
                        file_list_empty_message: file_panel.empty_message.clone(),
                        file_list_items_empty: file_panel.all_items.is_empty(),
                        filtered_file_list_items: file_panel.filtered_items.clone(),
                        file_selection: file_selection_snapshot.clone(),
                        selected_file_count: file_panel.selected_count,
                        selected_visible_file_count: file_panel.selected_visible_count,
                        on_select_all: EventHandler::new(move |_| select_all_visible_files(())),
                        on_clear_selected: EventHandler::new(move |_| clear_selected_files(())),
                        on_search_input: EventHandler::new(move |query: String| file_search_query.set(query)),
                        on_batch_export: batch_export_selected,
                        on_open_file: EventHandler::new(open_loaded_file),
                        on_activate: EventHandler::new(activate_tab),
                        on_remove: EventHandler::new(remove_file_list_item),
                        on_remove_path: EventHandler::new(remove_file_list_path),
                        on_toggle_selected: EventHandler::new(toggle_selected_file),
                        on_toggle_path: EventHandler::new(toggle_selected_path),
                        suppress_resize_observer: panel_resizing_snapshot
                    }

                    PanelResizeHandle {
                        side: PanelSide::Left,
                        width: left_panel_width_snapshot,
                        dragging: panel_resize_drag_snapshot.is_some_and(|drag| drag.side == PanelSide::Left),
                        i18n,
                        on_start: handle_panel_resize_start
                    }

                    EditorStage {
                        i18n,
                        active_tab: active_stage_tab_snapshot.clone(),
                        active_byte_doc: active_byte_doc_snapshot.clone(),
                        hex_jump_target: *hex_jump_target.read(),
                        text_jump_target: active_text_jump_target,
                        line_wrapping: line_wrapping_snapshot,
                        editor_search_panel_visible: editor_search_panel_visible_snapshot,
                        editor_search_text: editor_search_text_snapshot.clone(),
                        editor_replace_text: editor_replace_text_snapshot.clone(),
                        editor_search_case_sensitive: editor_search_case_sensitive_snapshot,
                        editor_search_controls_disabled: editor_search.controls_disabled,
                        editor_search_status_text: editor_search.status_text.clone(),
                        on_hex_change: EventHandler::new(update_hex_edit),
                        on_virtual_text_range_replace: EventHandler::new(update_virtual_text_range),
                        on_undo: undo_edit,
                        on_redo: redo_edit,
                        on_search_visible_change: EventHandler::new(move |visible| editor_search_panel_visible.set(visible)),
                        on_find_input: EventHandler::new(move |value: String| {
                            editor_search_text.set(value);
                            editor_search_match_index.set(0);
                        }),
                        on_case_sensitive_change: EventHandler::new(move |checked: bool| {
                            editor_search_case_sensitive.set(checked);
                            editor_search_match_index.set(0);
                        }),
                        on_replace_input: EventHandler::new(move |value: String| editor_replace_text.set(value)),
                        on_previous_match: EventHandler::new(go_to_previous_editor_match),
                        on_next_match: EventHandler::new(go_to_next_editor_match),
                        on_replace_current: EventHandler::new(replace_current_editor_match),
                        on_replace_all: EventHandler::new(replace_all_editor_matches),
                        on_find_key: EventHandler::new(handle_editor_search_key),
                        on_replace_key: EventHandler::new(handle_editor_replace_key),
                        suppress_resize_observer: panel_resizing_snapshot
                    }

                    PanelResizeHandle {
                        side: PanelSide::Right,
                        width: right_panel_width_snapshot,
                        dragging: panel_resize_drag_snapshot.is_some_and(|drag| drag.side == PanelSide::Right),
                        i18n,
                        on_start: handle_panel_resize_start
                    }

                    IndexPanel {
                        i18n,
                        active_file_label: active_file_label.clone(),
                        input_value: input_value_label,
                        output_value: output_value_label.clone(),
                        active_doc: active_doc_snapshot.clone(),
                        active_search: active_search.clone(),
                        selected_path: selected_path_snapshot.clone(),
                        tree_rows: tree_rows.clone(),
                        expanded_paths: expanded_paths_snapshot.clone(),
                        value_search_result,
                        on_search_change: EventHandler::new(move |query: String| update_active_search(query, tabs, active_tab_id)),
                        on_toggle_path: EventHandler::new(move |path: String| toggle_active_tree_path(path, tabs, active_tab_id)),
                        on_select_path: EventHandler::new(move |path: String| navigate_to_value_path(
                            path,
                            tabs,
                            active_tab_id,
                            next_jump_id,
                            text_jump_target,
                            hex_jump_target,
                            status,
                            i18n,
                        )),
                        suppress_resize_observer: panel_resizing_snapshot
                    }
                }
            }

            StatusBar {
                i18n,
                active_file_label,
                output_value: output_value_label,
                status: status_snapshot
            }
        }
    }
}

fn rton_display_encode_options(compact: bool) -> EncodeOptions {
    EncodeOptions {
        encoding: if compact {
            BinaryEncoding::Compact
        } else {
            BinaryEncoding::Standard
        },
        encrypted: false,
    }
}

fn file_input_summary(
    has_active_file: bool,
    byte_doc: Option<&ByteDocument>,
    text_byte_count: Option<usize>,
    i18n: I18n,
) -> String {
    if !has_active_file {
        return i18n.t("summary-no-output");
    }

    if let Some(byte_doc) = byte_doc {
        return format_panel_bytes(byte_doc.len());
    }

    if text_byte_count.is_some() {
        return i18n.t("summary-text-input");
    }

    i18n.t("summary-not-generated")
}

struct FileOutputSummaryInput<'a> {
    has_active_file: bool,
    active_tab_id: Option<usize>,
    mode: Option<EditorMode>,
    byte_doc: Option<&'a ByteDocument>,
    text_byte_count: Option<usize>,
    compact: bool,
    encrypted: bool,
    rton_output_size: Option<RtonOutputSize>,
    i18n: I18n,
}

fn file_output_summary(input: FileOutputSummaryInput<'_>) -> String {
    let FileOutputSummaryInput {
        has_active_file,
        active_tab_id,
        mode,
        byte_doc,
        text_byte_count,
        compact,
        encrypted,
        rton_output_size,
        i18n,
    } = input;
    if !has_active_file {
        return i18n.t("summary-no-output");
    }

    match mode {
        Some(EditorMode::RtonHex) if encrypted => {
            let encoding = rton_encoding_summary(compact, encrypted, i18n);
            rton_output_size
                .filter(|size| {
                    Some(size.tab_id) == active_tab_id
                        && size.compact == compact
                        && size.encrypted == encrypted
                })
                .map(|size| format!("{} · {encoding} RTON", format_panel_bytes(size.byte_count)))
                .unwrap_or_else(|| format!("{encoding} RTON"))
        }
        Some(EditorMode::RtonHex) => byte_doc
            .map(|bytes| {
                format!(
                    "{} · {} RTON",
                    format_panel_bytes(bytes.len()),
                    rton_encoding_summary(compact, false, i18n)
                )
            })
            .unwrap_or_else(|| i18n.t("summary-not-generated")),
        Some(mode @ (EditorMode::Json | EditorMode::Yaml | EditorMode::Toml)) => text_byte_count
            .filter(|byte_count| *byte_count > 0)
            .map(|byte_count| format!("{} · {}", format_panel_bytes(byte_count), mode.label()))
            .unwrap_or_else(|| i18n.t("summary-not-generated")),
        None => i18n.t("summary-no-output"),
    }
}

fn rton_encoding_summary(compact: bool, encrypted: bool, i18n: I18n) -> String {
    let base = if compact { "Compact" } else { "Standard" };
    if encrypted {
        format!("{base} · {}", i18n.t("toolbar-encrypted"))
    } else {
        base.to_string()
    }
}

fn format_panel_bytes(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.2} MB", bytes as f64 / 1024.0 / 1024.0)
    }
}

fn current_text_search_jump(
    buffer: &TextBuffer,
    matches: &[TextSearchMatch],
    match_index: usize,
    focus_token: u64,
) -> Option<crate::components::TextJumpTarget> {
    let match_ = matches.get(match_index.min(matches.len().saturating_sub(1)))?;
    let position = offset_to_text_buffer_position(buffer, match_.start);
    let end_position = offset_to_text_buffer_position(buffer, match_.end);
    let selection_end_column = if end_position.line == position.line {
        end_position.column
    } else {
        position.column
    };
    Some(crate::components::TextJumpTarget {
        id: focus_token,
        line: position.line,
        column: position.column,
        selection_end_column,
        line_count: buffer.line_count(),
        focus: focus_token > 0,
    })
}

fn request_text_search_focus(match_count: usize, mut focus_token: Signal<u64>) {
    if match_count > 0 {
        let next = focus_token.read().saturating_add(1);
        focus_token.set(next);
    }
}

fn offset_to_text_buffer_position(
    buffer: &TextBuffer,
    offset: usize,
) -> crate::domain::TextPosition {
    let bounded_offset = offset.min(buffer.text.len());
    let line_index = buffer
        .line_offsets
        .partition_point(|line_offset| *line_offset <= bounded_offset)
        .saturating_sub(1);
    let line_start = buffer.line_offsets.get(line_index).copied().unwrap_or(0);
    crate::domain::TextPosition {
        line: line_index + 1,
        column: bounded_offset.saturating_sub(line_start),
    }
}
