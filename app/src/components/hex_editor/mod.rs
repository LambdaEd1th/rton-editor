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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct HexContextMenu {
    x: i32,
    y: i32,
}

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
    suppress_resize_observer: bool,
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
    let mut scroll_mounted = use_signal(|| None::<MountedEvent>);
    let mut editor_mounted = use_signal(|| None::<MountedEvent>);
    let mut input_sink_value = use_signal(String::new);
    let mut context_menu = use_signal(|| None::<HexContextMenu>);

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

    use_effect(use_reactive(&suppress_resize_observer, move |suppressed| {
        if suppressed {
            return;
        }
        let Some(event) = scroll_mounted.peek().clone() else {
            return;
        };
        spawn(async move {
            if let Ok(rect) = event.get_client_rect().await {
                update_hex_viewport_height(viewport_height, rect.height());
            }
        });
    }));

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
    let context_menu_snapshot = *context_menu.read();
    let context_menu_open = context_menu_snapshot.is_some();

    use_effect(use_reactive(&context_menu_open, move |open| {
        set_hex_context_menu_document_dismiss_listener(open);
    }));

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
        if event.key().to_string() == "Escape" && context_menu.peek().is_some() {
            event.prevent_default();
            context_menu.set(None);
            return;
        }
        context_menu.set(None);
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
        context_menu.set(None);
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

    let mut open_context_menu = move |offset: usize, pane: HexPane, menu: HexContextMenu| {
        active_pane.set(pane);
        pending_hex_edit.set(None);
        pointer_selecting.set(false);
        if !hex_selection_contains(normalized_selection, offset) {
            selection_anchor.set(offset);
            selection_range.set(None);
        }
        selected_offset.set(offset);
        let mounted = editor_mounted.peek().clone();
        spawn(async move {
            context_menu.set(Some(
                hex_context_menu_from_client_position(menu, mounted).await,
            ));
        });
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
            onmounted: move |event| editor_mounted.set(Some(event)),
            onkeydown: handle_key,
            onmousedown: move |_| context_menu.set(None),
            onmouseup: move |_| {
                pointer_selecting.set(false);
                inspector_drag.set(None);
            },
            onmousemove: handle_inspector_mouse_move,
            button {
                r#type: "button",
                class: "rton-hex-context-close-sink",
                tabindex: "-1",
                aria_hidden: "true",
                onclick: move |_| context_menu.set(None)
            }
            textarea {
                id: "rton-hex-input-sink",
                class: "rton-hex-input-sink",
                aria_label: "HEX editor clipboard input",
                spellcheck: "false",
                autocapitalize: "off",
                autocomplete: "off",
                value: "{input_sink_value}",
                oninput: {
                    let bytes = bytes.clone();
                    move |event| {
                        context_menu.set(None);
                        let text = event.value();
                        input_sink_value.set(String::new());
                        paste_hex_text_from_clipboard(
                            &bytes,
                            text,
                            *active_pane.read(),
                            *insert_mode.read(),
                            bytes_len,
                            safe_selected_offset,
                            hex_signals.commit_targets(on_change),
                        );
                    }
                },
                onpaste: {
                    let bytes = bytes.clone();
                    move |event| {
                        if let Some(text) = hex_clipboard_event_text(&event) {
                            event.prevent_default();
                            paste_hex_text_from_clipboard(
                                &bytes,
                                text,
                                *active_pane.read(),
                                *insert_mode.read(),
                                bytes_len,
                                safe_selected_offset,
                                hex_signals.commit_targets(on_change),
                            );
                        } else {
                            input_sink_value.set(String::new());
                        }
                    }
                }
            }
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
                        onwheel: move |_| {
                            context_menu.set(None);
                            pointer_selecting.set(false);
                        },
                        onmounted: move |event| {
                            scroll_mounted.set(Some(event.clone()));
                            async move {
                                if let Ok(rect) = event.get_client_rect().await {
                                    update_hex_viewport_height(viewport_height, rect.height());
                                }
                            }
                        },
                        onresize: move |event| {
                            if suppress_resize_observer {
                                return;
                            }
                            if let Ok(size) = event.get_content_box_size() {
                                update_hex_viewport_height(viewport_height, size.height);
                            }
                        },
                        onscroll: move |event| {
                            context_menu.set(None);
                            pointer_selecting.set(false);
                            scroll_top.set(event.scroll_top());
                        },
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
                                    },
                                    on_context_menu: move |(offset, pane, menu): (usize, HexPane, HexContextMenu)| {
                                        open_context_menu(offset, pane, menu);
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
            if let Some(menu) = context_menu_snapshot {
                div {
                    class: "rton-hex-context-backdrop",
                    onmousedown: move |event| {
                        event.prevent_default();
                        context_menu.set(None);
                    },
                    oncontextmenu: move |event| {
                        event.prevent_default();
                        context_menu.set(None);
                    }
                }
                div {
                    class: "rton-hex-context-menu",
                    role: "menu",
                    style: "left: {menu.x}px; top: {menu.y}px",
                    onmousedown: move |event| {
                        event.prevent_default();
                        event.stop_propagation();
                    },
                    button {
                        r#type: "button",
                        role: "menuitem",
                        onclick: {
                            let bytes = bytes.clone();
                            move |_| {
                                copy_hex_selection_to_clipboard(
                                    &bytes,
                                    normalized_selection,
                                    safe_selected_offset,
                                    *active_pane.read(),
                                );
                                context_menu.set(None);
                                focus_hex_editor();
                            }
                        },
                        {i18n.t("editor-context-copy")}
                    }
                    button {
                        r#type: "button",
                        role: "menuitem",
                        onclick: {
                            let bytes = bytes.clone();
                            move |_| {
                                cut_hex_selection_to_clipboard(
                                    &bytes,
                                    normalized_selection,
                                    safe_selected_offset,
                                    *active_pane.read(),
                                    *insert_mode.read(),
                                    hex_signals.commit_targets(on_change),
                                );
                                context_menu.set(None);
                                focus_hex_editor();
                            }
                        },
                        {i18n.t("editor-context-cut")}
                    }
                    button {
                        r#type: "button",
                        role: "menuitem",
                        onclick: move |_| {
                            context_menu.set(None);
                            paste_hex_from_clipboard();
                        },
                        {i18n.t("editor-context-paste")}
                    }
                    div { class: "rton-hex-context-menu-separator", role: "separator" }
                    button {
                        r#type: "button",
                        role: "menuitem",
                        onclick: move |_| {
                            select_all_hex_bytes(bytes_len, selection_anchor, selection_range, selected_offset);
                            context_menu.set(None);
                            focus_hex_editor();
                        },
                        {i18n.t("editor-context-select-all")}
                    }
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

async fn hex_context_menu_from_client_position(
    menu: HexContextMenu,
    mounted: Option<MountedEvent>,
) -> HexContextMenu {
    let Some(event) = mounted else {
        return menu;
    };
    let Ok(rect) = event.get_client_rect().await else {
        return menu;
    };
    HexContextMenu {
        x: (menu.x as f64 - rect.origin.x).round().max(0.0) as i32,
        y: (menu.y as f64 - rect.origin.y).round().max(0.0) as i32,
    }
}

fn hex_selection_contains(selection: Option<ByteSelection>, offset: usize) -> bool {
    selection.is_some_and(|selection| {
        let start = selection.anchor.min(selection.focus);
        let end = selection.anchor.max(selection.focus);
        (start..=end).contains(&offset)
    })
}

fn copy_hex_selection_to_clipboard(
    bytes: &ByteDocument,
    selection: Option<ByteSelection>,
    fallback_offset: usize,
    pane: HexPane,
) {
    let Some(selected) = selected_hex_bytes(bytes, selection, fallback_offset) else {
        return;
    };
    write_hex_clipboard(&format_hex_clipboard_text(&selected, pane));
}

fn cut_hex_selection_to_clipboard(
    bytes: &ByteDocument,
    selection: Option<ByteSelection>,
    fallback_offset: usize,
    pane: HexPane,
    insert_mode: bool,
    commit_targets: HexCommitTargets,
) {
    let Some((target, selected)) = selected_hex_target_and_bytes(bytes, selection, fallback_offset)
    else {
        return;
    };
    write_hex_clipboard(&format_hex_clipboard_text(&selected, pane));
    commit_hex_clear_target(bytes, target, insert_mode, commit_targets);
}

fn selected_hex_bytes(
    bytes: &ByteDocument,
    selection: Option<ByteSelection>,
    fallback_offset: usize,
) -> Option<Vec<u8>> {
    selected_hex_range_and_bytes(bytes, selection, fallback_offset).map(|(_, _, selected)| selected)
}

fn selected_hex_range_and_bytes(
    bytes: &ByteDocument,
    selection: Option<ByteSelection>,
    fallback_offset: usize,
) -> Option<(usize, usize, Vec<u8>)> {
    let (target, selected) = selected_hex_target_and_bytes(bytes, selection, fallback_offset)?;
    Some((target.offset, target.length, selected))
}

fn selected_hex_target_and_bytes(
    bytes: &ByteDocument,
    selection: Option<ByteSelection>,
    fallback_offset: usize,
) -> Option<(HexSelectionTarget, Vec<u8>)> {
    let target = hex_selection_target(selection, bytes.len(), fallback_offset)?;
    let selected = hex_target_bytes(bytes, target)?;
    Some((target, selected))
}

fn format_hex_clipboard_text(bytes: &[u8], pane: HexPane) -> String {
    match pane {
        HexPane::Hex => bytes
            .iter()
            .map(|byte| format!("{byte:02X}"))
            .collect::<Vec<_>>()
            .join(" "),
        HexPane::Ascii => bytes.iter().map(|byte| *byte as char).collect(),
    }
}

fn paste_hex_text_from_clipboard(
    bytes: &ByteDocument,
    text: String,
    pane: HexPane,
    insert_mode: bool,
    bytes_len: usize,
    safe_selected_offset: usize,
    commit_targets: HexCommitTargets,
) {
    let Some(values) = parse_hex_clipboard_text(&text, pane) else {
        return;
    };
    if values.is_empty() {
        return;
    }
    let Some(target) = hex_selection_target(
        *commit_targets.selection_range.read(),
        bytes_len,
        safe_selected_offset,
    ) else {
        return;
    };
    commit_hex_write_target(bytes, target, values, insert_mode, commit_targets);
}

fn parse_hex_clipboard_text(text: &str, pane: HexPane) -> Option<Vec<u8>> {
    match pane {
        HexPane::Hex => parse_hex_bytes_text(text),
        HexPane::Ascii => text
            .chars()
            .map(|ch| {
                let code = ch as u32;
                (code <= 0xff).then_some(code as u8)
            })
            .collect(),
    }
}

fn parse_hex_bytes_text(text: &str) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();
    for token in text.split(|ch: char| ch.is_ascii_whitespace() || ",;:-".contains(ch)) {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }
        let token = token
            .strip_prefix("0x")
            .or_else(|| token.strip_prefix("0X"))
            .unwrap_or(token);
        if token.is_empty()
            || token.len() % 2 != 0
            || !token.chars().all(|ch| ch.is_ascii_hexdigit())
        {
            return None;
        }
        for chunk_start in (0..token.len()).step_by(2) {
            bytes.push(u8::from_str_radix(&token[chunk_start..chunk_start + 2], 16).ok()?);
        }
    }
    (!bytes.is_empty()).then_some(bytes)
}

fn select_all_hex_bytes(
    bytes_len: usize,
    mut selection_anchor: Signal<usize>,
    mut selection_range: Signal<Option<ByteSelection>>,
    mut selected_offset: Signal<usize>,
) {
    if bytes_len == 0 {
        return;
    }
    selection_anchor.set(0);
    selected_offset.set(bytes_len - 1);
    selection_range.set((bytes_len > 1).then_some(ByteSelection {
        anchor: 0,
        focus: bytes_len - 1,
    }));
}

#[cfg(target_arch = "wasm32")]
fn hex_clipboard_event_text(event: &dioxus_html::ClipboardEvent) -> Option<String> {
    use wasm_bindgen::JsCast;

    let web_event = event.data.downcast::<web_sys::Event>()?;
    let clipboard_event = web_event.dyn_ref::<web_sys::ClipboardEvent>()?;
    clipboard_event
        .clipboard_data()?
        .get_data("text/plain")
        .ok()
        .filter(|text| !text.is_empty())
}

#[cfg(not(target_arch = "wasm32"))]
fn hex_clipboard_event_text(_event: &dioxus_html::ClipboardEvent) -> Option<String> {
    None
}

fn write_hex_clipboard(text: &str) {
    let Ok(text_json) = serde_json::to_string(text) else {
        return;
    };
    dioxus::document::eval(&format!(
        r#"
        (() => {{
            const text = {text_json};
            const fallbackCopy = () => {{
                const previous = document.activeElement;
                const textarea = document.createElement("textarea");
                textarea.value = text;
                textarea.setAttribute("readonly", "true");
                textarea.style.position = "fixed";
                textarea.style.left = "-10000px";
                textarea.style.top = "0";
                document.body.appendChild(textarea);
                textarea.focus();
                textarea.select();
                try {{
                    document.execCommand("copy");
                }} catch (_error) {{}}
                textarea.remove();
                previous?.focus?.({{ preventScroll: true }});
            }};
            if (navigator.clipboard?.writeText) {{
                navigator.clipboard.writeText(text).catch(fallbackCopy);
            }} else {{
                fallbackCopy();
            }}
        }})();
        "#
    ));
}

fn paste_hex_from_clipboard() {
    dioxus::document::eval(
        r#"
        (() => {
            const input = document.getElementById("rton-hex-input-sink");
            if (!input) return;
            try {
                input.focus({ preventScroll: true });
            } catch (_error) {
                input.focus();
            }

            const dispatchText = (text) => {
                if (typeof text !== "string" || text.length === 0) return;
                input.value = text;
                let event;
                try {
                    event = new InputEvent("input", {
                        bubbles: true,
                        data: text,
                        inputType: "insertFromPaste",
                    });
                } catch (_error) {
                    event = new Event("input", { bubbles: true });
                }
                input.dispatchEvent(event);
            };

            if (navigator.clipboard?.readText) {
                navigator.clipboard.readText().then(dispatchText).catch(() => {
                    try {
                        document.execCommand?.("paste");
                    } catch (_error) {}
                });
            } else {
                try {
                    document.execCommand?.("paste");
                } catch (_error) {}
            }
        })();
        "#,
    );
}

fn focus_hex_editor() {
    dioxus::document::eval(
        r#"
        (() => {
            const editor = document.querySelector(".rton-hex-editor");
            if (!editor) return;
            try {
                editor.focus({ preventScroll: true });
            } catch (_error) {
                editor.focus();
            }
        })();
        "#,
    );
}

fn set_hex_context_menu_document_dismiss_listener(enabled: bool) {
    if !enabled {
        dioxus::document::eval(
            r#"
            (() => {
                window.__rtonHexContextMenuDismiss?.cleanup?.();
                window.__rtonHexContextMenuDismiss = undefined;
            })();
            "#,
        );
        return;
    }

    dioxus::document::eval(
        r#"
        (() => {
            window.__rtonHexContextMenuDismiss?.cleanup?.();

            const closeMenu = (event) => {
                if (event.target?.closest?.(".rton-hex-context-menu")) {
                    return;
                }
                cleanup();
                if (event.type === "contextmenu") {
                    event.preventDefault();
                }
                event.stopPropagation();
                event.stopImmediatePropagation?.();
                document.querySelector(".rton-hex-context-close-sink")?.click();
            };

            const cleanup = () => {
                document.removeEventListener("mousedown", closeMenu, true);
                document.removeEventListener("contextmenu", closeMenu, true);
                if (window.__rtonHexContextMenuDismiss?.cleanup === cleanup) {
                    window.__rtonHexContextMenuDismiss = undefined;
                }
            };

            document.addEventListener("mousedown", closeMenu, true);
            document.addEventListener("contextmenu", closeMenu, true);
            window.__rtonHexContextMenuDismiss = { cleanup };
        })();
        "#,
    );
}

#[cfg(test)]
mod context_menu_tests {
    use crate::domain::ByteDocument;

    use super::state::ByteSelection;
    use super::{
        HexPane, format_hex_clipboard_text, parse_hex_clipboard_text, selected_hex_range_and_bytes,
    };

    #[test]
    fn parses_hex_clipboard_tokens() {
        assert_eq!(
            parse_hex_clipboard_text("0xDE AD-be:ef", HexPane::Hex),
            Some(vec![0xde, 0xad, 0xbe, 0xef])
        );
        assert_eq!(parse_hex_clipboard_text("ABC", HexPane::Hex), None);
        assert_eq!(parse_hex_clipboard_text("GG", HexPane::Hex), None);
    }

    #[test]
    fn formats_hex_clipboard_by_active_pane() {
        assert_eq!(
            format_hex_clipboard_text(&[0xde, 0xad, 0xbe, 0xef], HexPane::Hex),
            "DE AD BE EF"
        );
        assert_eq!(format_hex_clipboard_text(b"RTON", HexPane::Ascii), "RTON");
    }

    #[test]
    fn selected_hex_range_falls_back_to_current_byte() {
        let bytes = ByteDocument::from_vec(vec![0x52, 0x54, 0x4f, 0x4e]);
        assert_eq!(
            selected_hex_range_and_bytes(&bytes, None, 2),
            Some((2, 1, vec![0x4f]))
        );
    }

    #[test]
    fn selected_hex_range_uses_explicit_selection() {
        let bytes = ByteDocument::from_vec(vec![0x52, 0x54, 0x4f, 0x4e]);
        assert_eq!(
            selected_hex_range_and_bytes(
                &bytes,
                Some(ByteSelection {
                    anchor: 3,
                    focus: 1,
                }),
                0,
            ),
            Some((1, 3, vec![0x54, 0x4f, 0x4e]))
        );
    }
}
