use dioxus::prelude::*;
use rton_editor_core::format_bytes;

use crate::domain::{
    ByteDocument, HexEdit, HexSearchMode, HexSearchResult, find_hex_search_result,
    hex_scroll_top_for_row, parse_search_pattern, relative_search_match, replace_all_byte_edits,
};
use crate::i18n::I18n;
use crate::platform::run_cpu_task;

mod inspector;
mod keyboard;
mod logic;
mod row;
mod search_panel;
mod snapshot;
mod state;

pub(crate) use state::HexJumpTarget;

use inspector::HexByteInspector;
use keyboard::handle_hex_key;
use logic::*;
use row::HexRow;
use search_panel::HexSearchPanel;
use snapshot::hex_editor_snapshot;
use state::{
    ByteSelection, HEX_BYTES_PER_ROW, HexInspectorResizeDrag, HexPane, use_hex_editor_signals,
};

#[component]
pub(crate) fn HexEditor(
    bytes: ByteDocument,
    jump_target: Option<HexJumpTarget>,
    search_panel_visible: bool,
    i18n: I18n,
    on_change: EventHandler<Vec<HexEdit>>,
    on_undo: EventHandler<()>,
    on_redo: EventHandler<()>,
    on_search_visible_change: EventHandler<bool>,
) -> Element {
    let hex_signals = use_hex_editor_signals();
    let mut selected_offset = hex_signals.selected_offset;
    let mut selection_anchor = hex_signals.selection_anchor;
    let mut selection_range = hex_signals.selection_range;
    let mut pending_hex_edit = hex_signals.pending_hex_edit;
    let mut insert_mode = hex_signals.insert_mode;
    let mut active_pane = hex_signals.active_pane;
    let mut pointer_selecting = hex_signals.pointer_selecting;
    let mut scroll_top = hex_signals.scroll_top;
    let viewport_height = hex_signals.viewport_height;
    let mut inspector_width = hex_signals.inspector_width;
    let mut inspector_drag = hex_signals.inspector_drag;
    let search_mode = hex_signals.search_mode;
    let search_query = hex_signals.search_query;
    let replace_query = hex_signals.replace_query;
    let case_sensitive = hex_signals.case_sensitive;
    let mut hex_search_result = use_signal(empty_hex_search_result);
    let mut hex_search_generation = use_signal(|| 0_u64);

    let search_mode_input = *search_mode.read();
    let search_query_input = search_query.read().clone();
    let case_sensitive_input = *case_sensitive.read();
    use_effect(use_reactive(
        (
            &bytes,
            &search_panel_visible,
            &search_mode_input,
            &search_query_input,
            &case_sensitive_input,
        ),
        move |(bytes_for_search, panel_visible, mode, query, case_sensitive)| {
            let pattern = parse_search_pattern(mode, &query, i18n);
            if !panel_visible || !pattern.valid || pattern.bytes.is_empty() {
                clear_hex_search_result(hex_search_result);
                return;
            }

            let generation = (*hex_search_generation.peek()).saturating_add(1);
            hex_search_generation.set(generation);
            clear_hex_search_result(hex_search_result);
            spawn(async move {
                let ascii_insensitive = mode == HexSearchMode::Ascii && !case_sensitive;
                let bytes = pattern.bytes;
                let result = run_cpu_task(move || {
                    find_hex_search_result(&bytes_for_search, &bytes, ascii_insensitive)
                })
                .await;
                if *hex_search_generation.peek() == generation {
                    hex_search_result.set(result);
                }
            });
        },
    ));

    let snapshot = hex_editor_snapshot(
        &bytes,
        *selected_offset.read(),
        *selection_range.read(),
        *insert_mode.read(),
        *scroll_top.read(),
        *viewport_height.read(),
        *inspector_width.read(),
        search_panel_visible,
        *search_mode.read(),
        search_query.read().clone(),
        replace_query.read().clone(),
        *case_sensitive.read(),
        hex_search_result.read().clone(),
        i18n,
    );

    let bytes_len = snapshot.bytes_len;
    let safe_selected_offset = snapshot.safe_selected_offset;
    if safe_selected_offset != *selected_offset.read() {
        selected_offset.set(safe_selected_offset);
    }

    let selected_byte = snapshot.selected_byte;
    let offset_width = snapshot.offset_width;
    let row_count = snapshot.row_count;
    let visible_rows = snapshot.visible_rows;
    let normalized_selection = snapshot.normalized_selection;
    let search_mode_snapshot = snapshot.search_mode;
    let search_query_snapshot = snapshot.search_query;
    let replace_query_snapshot = snapshot.replace_query;
    let case_sensitive_snapshot = snapshot.case_sensitive;
    let search_matches = snapshot.search_matches;
    let current_match = snapshot.current_match;
    let search_status_text = snapshot.search_status_text;
    let search_controls_disabled = snapshot.search_controls_disabled;
    let replace_controls_disabled = snapshot.replace_controls_disabled;
    let search_pattern_bytes = snapshot.search_pattern_bytes;
    let replace_pattern_bytes = snapshot.replace_pattern_bytes;
    let virtual_scroll_content_height = snapshot.content_height;
    let style = snapshot.style;
    let mode_label = snapshot.mode_label;
    let selection_label = snapshot.selection_label;
    let inspector_drag_snapshot = *inspector_drag.read();
    let inspector_width_snapshot = *inspector_width.read();

    use_effect(use_reactive((&jump_target,), move |(jump_target,)| {
        let Some(target) = jump_target else {
            return;
        };
        if bytes_len == 0 {
            return;
        }

        let offset = target.offset.min(bytes_len.saturating_sub(1));
        active_pane.set(HexPane::Hex);
        pending_hex_edit.set(None);
        selection_anchor.set(offset);
        selection_range.set(None);
        selected_offset.set(offset);

        let target_row = offset / HEX_BYTES_PER_ROW;
        let target_scroll_top =
            hex_scroll_top_for_row(target_row, row_count, *viewport_height.read());
        scroll_top.set(target_scroll_top);
        scroll_hex_editor_to(target_scroll_top);
    }));

    let bytes_for_key = bytes.clone();
    let handle_key = move |event: KeyboardEvent| {
        handle_hex_key(
            event,
            &bytes_for_key,
            bytes_len,
            safe_selected_offset,
            normalized_selection,
            insert_mode,
            active_pane,
            pending_hex_edit,
            selection_range,
            selection_anchor,
            selected_offset,
            on_change,
            on_undo,
            on_redo,
            on_search_visible_change,
        );
    };

    let mut update_selection_from_pointer = move |offset: usize, pane: HexPane, extend: bool| {
        active_pane.set(pane);
        pending_hex_edit.set(None);
        if extend {
            let anchor = *selection_anchor.read();
            selection_range.set(if anchor == offset {
                None
            } else {
                Some(ByteSelection {
                    anchor,
                    focus: offset,
                })
            });
        } else {
            selection_anchor.set(offset);
            selection_range.set(None);
        }
        selected_offset.set(offset);
        pointer_selecting.set(true);
    };

    let search_matches_for_next = search_matches.clone();
    let go_to_next_match = move |_| {
        if let Some(next) = relative_search_match(
            &search_matches_for_next,
            safe_selected_offset,
            current_match,
            true,
        ) {
            active_pane.set(if search_mode_snapshot == HexSearchMode::Ascii {
                HexPane::Ascii
            } else {
                HexPane::Hex
            });
            set_hex_focus(
                next.offset as isize,
                false,
                bytes_len,
                selection_range,
                selection_anchor,
                selected_offset,
            );
        }
    };

    let search_matches_for_previous = search_matches.clone();
    let go_to_previous_match = move |_| {
        if let Some(previous) = relative_search_match(
            &search_matches_for_previous,
            safe_selected_offset,
            current_match,
            false,
        ) {
            active_pane.set(if search_mode_snapshot == HexSearchMode::Ascii {
                HexPane::Ascii
            } else {
                HexPane::Hex
            });
            set_hex_focus(
                previous.offset as isize,
                false,
                bytes_len,
                selection_range,
                selection_anchor,
                selected_offset,
            );
        }
    };

    let commit_targets = hex_signals.commit_targets(on_change);
    let search_matches_for_replace_current = search_matches.clone();
    let replace_pattern_for_current = replace_pattern_bytes.clone();
    let bytes_for_replace_current = bytes.clone();
    let replace_current_match = move |_| {
        if replace_controls_disabled {
            return;
        }
        if let Some(target) = current_match.or_else(|| {
            relative_search_match(
                &search_matches_for_replace_current,
                safe_selected_offset,
                None,
                true,
            )
        }) {
            commit_hex_replace_span(
                &bytes_for_replace_current,
                target.offset,
                target.length,
                replace_pattern_for_current.clone(),
                commit_targets,
            );
        }
    };

    let bytes_for_replace_all = bytes.clone();
    let search_pattern_for_replace_all = search_pattern_bytes.clone();
    let replace_pattern_for_replace_all = replace_pattern_bytes.clone();
    let replace_all_matches = move |_| {
        if replace_controls_disabled || search_pattern_for_replace_all.is_empty() {
            return;
        }
        let Some(edits) = replace_all_byte_edits(
            &bytes_for_replace_all,
            &search_pattern_for_replace_all,
            &replace_pattern_for_replace_all,
            search_mode_snapshot == HexSearchMode::Ascii && !case_sensitive_snapshot,
        ) else {
            return;
        };
        commit_hex_edits(
            &bytes_for_replace_all,
            edits,
            safe_selected_offset,
            commit_targets,
        );
    };

    let handle_inspector_mouse_move = move |event: MouseEvent| {
        let Some(drag) = *inspector_drag.read() else {
            return;
        };
        event.prevent_default();
        let delta = drag.start_x - event.client_coordinates().x;
        inspector_width.set(clamp_hex_inspector_width(drag.start_width as f64 + delta));
    };

    if bytes.is_empty() {
        return rsx! {
            div {
                class: "rton-hex-editor",
                style: "{style}",
                div { class: "rton-hex-empty", {i18n.t("hex-empty")} }
            }
        };
    }

    rsx! {
        div {
            class: "rton-hex-editor",
            style: "{style}",
            tabindex: "0",
            onkeydown: handle_key,
            onmouseup: move |_| {
                pointer_selecting.set(false);
                inspector_drag.set(None);
            },
            onmousemove: handle_inspector_mouse_move,
            div { class: "rton-hex-summary",
                span { "{format_bytes(bytes_len)}" }
                span { "Offset {to_offset_hex(safe_selected_offset, offset_width - 2)}" }
                span { "Value 0x{byte_to_hex(selected_byte)}" }
                if let Some(selection_label) = selection_label {
                    span { "{selection_label}" }
                }
                button {
                    r#type: "button",
                    class: if *insert_mode.read() { "rton-hex-mode-button is-active" } else { "rton-hex-mode-button" },
                    onclick: move |_| {
                        pending_hex_edit.set(None);
                        let next_insert_mode = !*insert_mode.read();
                        insert_mode.set(next_insert_mode);
                    },
                    "{mode_label}"
                }
            }
            div { class: "rton-hex-body",
                div { class: "rton-hex-table-pane",
                    div { class: "rton-hex-header", aria_hidden: "true",
                        span { class: "rton-hex-offset", "OFFSET" }
                        div { class: "rton-hex-grid",
                            for column in 0..HEX_BYTES_PER_ROW {
                                span { class: "rton-hex-column-label", "{column:02X}" }
                            }
                        }
                        span { class: "rton-hex-ascii-label", "ASCII" }
                    }
                    div {
                        class: "rton-hex-scroll",
                        onmounted: move |event| async move {
                            if let Ok(rect) = event.get_client_rect().await {
                                update_hex_viewport_height(viewport_height, rect.height());
                            }
                        },
                        onresize: move |event| {
                            if let Ok(size) = event.get_content_box_size() {
                                update_hex_viewport_height(viewport_height, size.height);
                            }
                        },
                        onscroll: move |event| scroll_top.set(event.scroll_top()),
                        div {
                            class: "rton-hex-virtual-space",
                            style: "height: {virtual_scroll_content_height}px",
                            for (row_index, row_top) in visible_rows {
                                HexRow {
                                    bytes: bytes.clone(),
                                    row_index,
                                    row_top,
                                    offset_width,
                                    selected_offset: safe_selected_offset,
                                    normalized_selection,
                                    pending_hex_edit: pending_hex_edit.read().clone(),
                                    search_matches: search_matches.clone(),
                                    current_match,
                                    on_select: move |selection: (usize, HexPane, bool)| {
                                        update_selection_from_pointer(selection.0, selection.1, selection.2);
                                    },
                                    on_enter: move |selection: (usize, HexPane)| {
                                        if *pointer_selecting.read() {
                                            active_pane.set(selection.1);
                                            let anchor = *selection_anchor.read();
                                            selection_range.set(if anchor == selection.0 {
                                                None
                                            } else {
                                                Some(ByteSelection { anchor, focus: selection.0 })
                                            });
                                            selected_offset.set(selection.0);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                div {
                    class: if inspector_drag_snapshot.is_some() { "rton-hex-inspector-resize-handle dragging" } else { "rton-hex-inspector-resize-handle" },
                    role: "separator",
                    aria_orientation: "vertical",
                    aria_label: i18n.t("hex-inspector-resize"),
                    onmousedown: move |event| {
                        event.prevent_default();
                        inspector_drag.set(Some(HexInspectorResizeDrag {
                            start_x: event.client_coordinates().x,
                            start_width: inspector_width_snapshot,
                        }));
                    }
                }
                HexByteInspector {
                    bytes: bytes.clone(),
                    offset: safe_selected_offset,
                    i18n
                }
            }
            if search_panel_visible {
                HexSearchPanel {
                    i18n,
                    search_mode_snapshot,
                    search_query_snapshot,
                    replace_query_snapshot,
                    case_sensitive_snapshot,
                    search_controls_disabled,
                    replace_controls_disabled,
                    search_status_text,
                    search_mode,
                    search_query,
                    replace_query,
                    case_sensitive,
                    go_to_previous_match,
                    go_to_next_match,
                    replace_current_match,
                    replace_all_matches,
                    on_close: move |_| on_search_visible_change.call(false)
                }
            }
        }
    }
}

fn empty_hex_search_result() -> HexSearchResult {
    HexSearchResult {
        matches: Vec::new(),
        capped: false,
    }
}

fn clear_hex_search_result(mut result: Signal<HexSearchResult>) {
    if *result.peek() != empty_hex_search_result() {
        result.set(empty_hex_search_result());
    }
}
