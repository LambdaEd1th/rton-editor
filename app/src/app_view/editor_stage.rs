use dioxus::prelude::*;

use crate::app_layout::EmptyDropStage;
use crate::components::{HexEditor, HexJumpTarget};
use crate::domain::virtual_scroll::{TEXT_MAX_VIRTUAL_SCROLL_HEIGHT, TEXT_OVERSCAN_ROWS};
use crate::domain::{
    ByteDocument, EditorMode, HexEdit, TEXT_DEFAULT_VIEWPORT_HEIGHT, TEXT_ROW_HEIGHT, TabTaskState,
    TextBuffer, TextContentState, measured_text_viewport_height, text_virtual_row_top,
    text_virtual_scroll,
};
use crate::i18n::I18n;
use std::sync::Arc;

const TEXT_WRAP_CHAR_WIDTH: usize = 8;
const TEXT_WRAP_LINE_NUMBER_WIDTH: usize = 76;
const TEXT_WRAP_HORIZONTAL_PADDING: usize = 30;
const TEXT_DEFAULT_VIEWPORT_WIDTH: usize = 960;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EditingTextLine {
    line_index: usize,
    initial_column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TextWrapLayout {
    line_tops: Arc<Vec<usize>>,
    line_heights: Arc<Vec<usize>>,
    logical_height: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WrappedTextScroll {
    content_height: usize,
    logical_scroll_top: usize,
    display_scroll_top: usize,
    start_row: usize,
    end_row: usize,
    scaled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct VisibleTextLine {
    top: i64,
    height: usize,
    line_index: usize,
    line_number: usize,
    line_text: String,
}

#[derive(Clone, PartialEq)]
pub(super) struct EditorStageTab {
    pub(super) id: usize,
    pub(super) mode: EditorMode,
    pub(super) text_buffer: Option<Arc<TextBuffer>>,
    pub(super) text_state: TextContentState,
    pub(super) task_state: Option<TabTaskState>,
}

#[component]
pub(super) fn EditorStage(
    i18n: I18n,
    active_tab: Option<EditorStageTab>,
    active_byte_doc: Option<ByteDocument>,
    hex_jump_target: Option<HexJumpTarget>,
    line_wrapping: bool,
    editor_search_panel_visible: bool,
    editor_search_text: String,
    editor_replace_text: String,
    editor_search_case_sensitive: bool,
    editor_search_controls_disabled: bool,
    editor_search_status_text: String,
    on_hex_change: EventHandler<Vec<HexEdit>>,
    on_virtual_text_line_change: EventHandler<(usize, String)>,
    on_undo: EventHandler<()>,
    on_redo: EventHandler<()>,
    on_search_visible_change: EventHandler<bool>,
    on_find_input: EventHandler<String>,
    on_case_sensitive_change: EventHandler<bool>,
    on_replace_input: EventHandler<String>,
    on_previous_match: EventHandler<()>,
    on_next_match: EventHandler<()>,
    on_replace_current: EventHandler<()>,
    on_replace_all: EventHandler<()>,
    on_find_key: EventHandler<KeyboardEvent>,
    on_replace_key: EventHandler<KeyboardEvent>,
) -> Element {
    let handle_text_editor_key = {
        let on_search_visible_change = on_search_visible_change;
        move |event: KeyboardEvent| {
            if is_find_shortcut(&event) {
                event.prevent_default();
                on_search_visible_change.call(true);
                focus_editor_find_input();
            }
        }
    };

    rsx! {
        section { class: "rton-editor-stage",
            if let Some(active_tab) = active_tab.as_ref() {
                if let Some(task) = active_tab.task_state.as_ref() {
                    div { class: "editor-surface",
                        div { class: "empty-state",
                            strong { {i18n.t_args("editor-converting-title", &[("mode", task.target_mode.label().to_string())])} }
                            p { {i18n.t("editor-converting-subtitle")} }
                        }
                    }
                } else if active_tab.mode == EditorMode::RtonHex {
                    if let Some(byte_doc) = active_byte_doc.clone() {
                        HexEditor {
                            key: "{active_tab.id}-hex",
                            bytes: byte_doc,
                            jump_target: hex_jump_target,
                            search_panel_visible: editor_search_panel_visible,
                            i18n,
                            on_change: on_hex_change,
                            on_undo,
                            on_redo,
                            on_search_visible_change
                        }
                    } else {
                        div { class: "editor-surface",
                            div { class: "empty-state",
                                strong { {i18n.t("hex-invalid-title")} }
                                p { {i18n.t("hex-empty")} }
                            }
                        }
                    }
                } else if active_tab.text_state.text_format().is_some() {
                    div {
                        class: "editor-surface",
                        onkeydown: handle_text_editor_key,
                        if let Some(buffer) = active_tab.text_buffer.clone() {
                            VirtualTextEditor {
                                key: "{active_tab.id}-{active_tab.mode.label()}-virtual-text",
                                buffer,
                                line_wrapping,
                                on_line_change: on_virtual_text_line_change
                            }
                        } else {
                            div { class: "empty-state",
                                strong { {i18n.t("hex-empty")} }
                                p { {i18n.t("hex-empty")} }
                            }
                        }
                        if editor_search_panel_visible {
                            div { class: "editor-search-panel",
                                input {
                                    id: "rton-editor-find-input",
                                    class: "editor-search-field",
                                    r#type: "search",
                                    placeholder: i18n.t("editor-find-placeholder"),
                                    value: "{editor_search_text}",
                                    spellcheck: "false",
                                    onmounted: move |_| focus_editor_find_input(),
                                    oninput: move |event| on_find_input.call(event.value()),
                                    onkeydown: move |event| on_find_key.call(event)
                                }
                                button {
                                    r#type: "button",
                                    class: "editor-search-button",
                                    disabled: editor_search_controls_disabled,
                                    onclick: move |_| on_previous_match.call(()),
                                    {i18n.t("editor-previous")}
                                }
                                button {
                                    r#type: "button",
                                    class: "editor-search-button",
                                    disabled: editor_search_controls_disabled,
                                    onclick: move |_| on_next_match.call(()),
                                    {i18n.t("editor-next")}
                                }
                                label { class: "editor-search-check",
                                    input {
                                        r#type: "checkbox",
                                        checked: editor_search_case_sensitive,
                                        onchange: move |event| on_case_sensitive_change.call(event.checked())
                                    }
                                    span { {i18n.t("editor-case")} }
                                }
                                input {
                                    class: "editor-search-field editor-replace-field",
                                    value: "{editor_replace_text}",
                                    placeholder: i18n.t("editor-replace-placeholder"),
                                    spellcheck: "false",
                                    oninput: move |event| on_replace_input.call(event.value()),
                                    onkeydown: move |event| on_replace_key.call(event)
                                }
                                button {
                                    r#type: "button",
                                    class: "editor-search-button",
                                    disabled: editor_search_controls_disabled,
                                    onclick: move |_| on_replace_current.call(()),
                                    {i18n.t("editor-replace")}
                                }
                                button {
                                    r#type: "button",
                                    class: "editor-search-button",
                                    disabled: editor_search_controls_disabled,
                                    onclick: move |_| on_replace_all.call(()),
                                    {i18n.t("editor-replace-all")}
                                }
                                span { class: "editor-search-status", "{editor_search_status_text}" }
                                button {
                                    r#type: "button",
                                    class: "editor-search-close",
                                    title: i18n.t("editor-close-search"),
                                    onclick: move |_| on_search_visible_change.call(false),
                                    "×"
                                }
                            }
                        }
                    }
                }
            } else {
                EmptyDropStage { i18n }
            }
        }
    }
}

#[component]
fn VirtualTextEditor(
    buffer: Arc<TextBuffer>,
    line_wrapping: bool,
    on_line_change: EventHandler<(usize, String)>,
) -> Element {
    let mut scroll_top = use_signal(|| 0_f64);
    let mut editing_line = use_signal(|| None::<EditingTextLine>);
    let viewport_height = use_signal(|| TEXT_DEFAULT_VIEWPORT_HEIGHT);
    let viewport_width = use_signal(|| TEXT_DEFAULT_VIEWPORT_WIDTH);
    let line_count = buffer.line_count();
    let scroll_top_snapshot = *scroll_top.read();
    let viewport_height_snapshot = *viewport_height.read();
    let viewport_width_snapshot = *viewport_width.read();
    let wrap_layout = use_memo(use_reactive(
        &(buffer.clone(), line_wrapping, viewport_width_snapshot),
        move |(buffer, line_wrapping, viewport_width)| {
            line_wrapping.then(|| {
                Arc::new(text_wrap_layout(
                    buffer.text.as_ref(),
                    buffer.line_offsets.as_ref(),
                    viewport_width,
                ))
            })
        },
    ));
    let wrap_layout_snapshot = wrap_layout.read().clone();
    let wrapped_scroll = wrap_layout_snapshot
        .as_deref()
        .map(|layout| wrapped_text_scroll(layout, scroll_top_snapshot, viewport_height_snapshot));
    let virtual_scroll = wrap_layout_snapshot
        .is_none()
        .then(|| text_virtual_scroll(line_count, scroll_top_snapshot, viewport_height_snapshot));
    let content_height = wrapped_scroll
        .map(|scroll| scroll.content_height)
        .or_else(|| virtual_scroll.map(|scroll| scroll.content_height))
        .unwrap_or(0);
    let visible_lines = text_buffer_visible_lines(
        buffer.text.as_ref(),
        buffer.line_offsets.as_ref(),
        scroll_top_snapshot,
        virtual_scroll,
        wrap_layout_snapshot.as_deref(),
        wrapped_scroll,
    );
    let preview_class = if line_wrapping {
        "virtual-text-preview wrap"
    } else {
        "virtual-text-preview"
    };

    rsx! {
        div { class: preview_class,
            div {
                class: "virtual-text-preview-content",
                style: "--virtual-text-row-height: {TEXT_ROW_HEIGHT}px",
                onmounted: move |event| async move {
                    if let Ok(rect) = event.get_client_rect().await {
                        update_text_buffer_viewport_size(
                            viewport_height,
                            viewport_width,
                            rect.height(),
                            rect.width(),
                        );
                    }
                },
                onresize: move |event| {
                    if let Ok(size) = event.get_content_box_size() {
                        update_text_buffer_viewport_size(
                            viewport_height,
                            viewport_width,
                            size.height,
                            size.width,
                        );
                    }
                },
                onscroll: move |event| scroll_top.set(event.scroll_top()),
                div {
                    class: "virtual-text-virtual-space",
                    style: "height: {content_height}px",
                    for line in visible_lines {
                        div {
                            class: "virtual-text-virtual-row",
                            style: "transform: translateY({line.top}px); height: {line.height}px",
                            span { class: "virtual-text-line-number", "{line.line_number}" }
                            if let Some(editing) = *editing_line.read()
                                && editing.line_index == line.line_index
                            {
                                VirtualTextLineEditor {
                                    key: "{line.line_index}-editor",
                                    line_index: line.line_index,
                                    line_number: line.line_number,
                                    line_text: line.line_text,
                                    initial_column: editing.initial_column,
                                    on_commit: on_line_change,
                                    on_exit: EventHandler::new(move |_| editing_line.set(None))
                                }
                            } else {
                                VirtualTextLineView {
                                    key: "{line.line_index}-view",
                                    line_number: line.line_number,
                                    line_text: line.line_text,
                                    on_begin_edit: EventHandler::new(move |initial_column| {
                                        editing_line.set(Some(EditingTextLine {
                                            line_index: line.line_index,
                                            initial_column,
                                        }));
                                    }),
                                    on_select_start: EventHandler::new(move |_| editing_line.set(None))
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn update_text_buffer_viewport_size(
    viewport_height: Signal<usize>,
    viewport_width: Signal<usize>,
    height: f64,
    width: f64,
) {
    update_text_buffer_viewport_height(viewport_height, height);
    update_text_buffer_viewport_width(viewport_width, width);
}

fn update_text_buffer_viewport_height(mut viewport_height: Signal<usize>, height: f64) {
    let next = measured_text_viewport_height(height);
    if next != *viewport_height.read() {
        viewport_height.set(next);
    }
}

fn update_text_buffer_viewport_width(mut viewport_width: Signal<usize>, width: f64) {
    let next = measured_text_viewport_width(width);
    if next != *viewport_width.read() {
        viewport_width.set(next);
    }
}

fn measured_text_viewport_width(width: f64) -> usize {
    if width.is_finite() && width > 0.0 {
        width.ceil().min(usize::MAX as f64) as usize
    } else {
        TEXT_DEFAULT_VIEWPORT_WIDTH
    }
}

fn is_find_shortcut(event: &KeyboardEvent) -> bool {
    let modifiers = event.modifiers();
    (modifiers.ctrl() || modifiers.meta()) && event.key().to_string().eq_ignore_ascii_case("f")
}

fn focus_editor_find_input() {
    dioxus::document::eval(
        r#"
        (() => {
            const focusInput = () => {
                const input = document.getElementById("rton-editor-find-input");
                if (!input) return false;
                try {
                    input.focus({ preventScroll: true });
                } catch (_error) {
                    input.focus();
                }
                input.select?.();
                return true;
            };
            requestAnimationFrame(() => {
                if (!focusInput()) {
                    requestAnimationFrame(focusInput);
                }
            });
        })();
        "#,
    );
}

fn text_buffer_visible_lines(
    text: &str,
    line_offsets: &[usize],
    scroll_top: f64,
    virtual_scroll: Option<crate::domain::virtual_scroll::TextVirtualScroll>,
    wrap_layout: Option<&TextWrapLayout>,
    wrapped_scroll: Option<WrappedTextScroll>,
) -> Vec<VisibleTextLine> {
    if let (Some(layout), Some(scroll)) = (wrap_layout, wrapped_scroll) {
        return wrapped_text_buffer_visible_lines(text, line_offsets, layout, scroll);
    }

    let Some(virtual_scroll) = virtual_scroll else {
        return Vec::new();
    };

    fixed_text_buffer_visible_lines(
        text,
        line_offsets,
        virtual_scroll.start_row,
        virtual_scroll.end_row,
        scroll_top,
        virtual_scroll,
    )
}

fn fixed_text_buffer_visible_lines(
    text: &str,
    line_offsets: &[usize],
    start_row: usize,
    end_row: usize,
    scroll_top: f64,
    virtual_scroll: crate::domain::virtual_scroll::TextVirtualScroll,
) -> Vec<VisibleTextLine> {
    (start_row..end_row)
        .filter_map(|row_index| {
            let line_text = text_line_at(text, line_offsets, row_index)?;
            Some(VisibleTextLine {
                top: text_virtual_row_top(row_index, scroll_top, virtual_scroll),
                height: TEXT_ROW_HEIGHT,
                line_index: row_index,
                line_number: row_index + 1,
                line_text,
            })
        })
        .collect()
}

fn wrapped_text_buffer_visible_lines(
    text: &str,
    line_offsets: &[usize],
    layout: &TextWrapLayout,
    scroll: WrappedTextScroll,
) -> Vec<VisibleTextLine> {
    (scroll.start_row..scroll.end_row)
        .filter_map(|row_index| {
            let line_top = *layout.line_tops.get(row_index)?;
            let line_height = *layout.line_heights.get(row_index)?;
            let top = wrapped_text_row_top(line_top, scroll);
            let line_text = text_line_at(text, line_offsets, row_index)?;
            Some(VisibleTextLine {
                top,
                height: line_height,
                line_index: row_index,
                line_number: row_index + 1,
                line_text,
            })
        })
        .collect()
}

fn text_line_at(text: &str, line_offsets: &[usize], row_index: usize) -> Option<String> {
    let start = *line_offsets.get(row_index)?;
    let mut end = line_offsets
        .get(row_index + 1)
        .copied()
        .map(|offset| offset.saturating_sub(1))
        .unwrap_or(text.len());
    if end > start && text.as_bytes().get(end.saturating_sub(1)) == Some(&b'\r') {
        end = end.saturating_sub(1);
    }
    Some(text[start..end].to_string())
}

fn text_wrap_layout(text: &str, line_offsets: &[usize], viewport_width: usize) -> TextWrapLayout {
    let chars_per_row = text_wrap_chars_per_row(viewport_width);
    let mut line_tops = Vec::with_capacity(line_offsets.len());
    let mut line_heights = Vec::with_capacity(line_offsets.len());
    let mut top = 0_usize;

    for row_index in 0..line_offsets.len() {
        line_tops.push(top);
        let line_len = text_line_byte_len(text, line_offsets, row_index);
        let visual_rows = line_len.max(1).div_ceil(chars_per_row).max(1);
        let height = visual_rows.saturating_mul(TEXT_ROW_HEIGHT);
        line_heights.push(height);
        top = top.saturating_add(height);
    }

    TextWrapLayout {
        line_tops: Arc::new(line_tops),
        line_heights: Arc::new(line_heights),
        logical_height: top,
    }
}

fn text_wrap_chars_per_row(viewport_width: usize) -> usize {
    viewport_width
        .saturating_sub(TEXT_WRAP_LINE_NUMBER_WIDTH)
        .saturating_sub(TEXT_WRAP_HORIZONTAL_PADDING)
        .checked_div(TEXT_WRAP_CHAR_WIDTH)
        .unwrap_or(1)
        .max(1)
}

fn text_line_byte_len(text: &str, line_offsets: &[usize], row_index: usize) -> usize {
    let Some(start) = line_offsets.get(row_index).copied() else {
        return 0;
    };
    let mut end = line_offsets
        .get(row_index + 1)
        .copied()
        .map(|offset| offset.saturating_sub(1))
        .unwrap_or(text.len());
    if end > start && text.as_bytes().get(end.saturating_sub(1)) == Some(&b'\r') {
        end = end.saturating_sub(1);
    }
    end.saturating_sub(start)
}

fn wrapped_text_scroll(
    layout: &TextWrapLayout,
    scroll_top: f64,
    viewport_height: usize,
) -> WrappedTextScroll {
    let line_count = layout.line_tops.len();
    let content_height = layout.logical_height.min(TEXT_MAX_VIRTUAL_SCROLL_HEIGHT);
    if line_count == 0 {
        return WrappedTextScroll {
            content_height,
            logical_scroll_top: 0,
            display_scroll_top: 0,
            start_row: 0,
            end_row: 0,
            scaled: false,
        };
    }

    let safe_scroll_top = safe_text_scroll_top(scroll_top);
    let scaled = layout.logical_height > TEXT_MAX_VIRTUAL_SCROLL_HEIGHT;
    let display_scroll_top = safe_scroll_top
        .round()
        .min(content_height.saturating_sub(viewport_height) as f64)
        .max(0.0) as usize;
    let logical_scroll_top = if scaled {
        let display_scrollable = content_height.saturating_sub(viewport_height).max(1);
        let logical_scrollable = layout.logical_height.saturating_sub(viewport_height).max(1);
        let ratio = (safe_scroll_top / display_scrollable as f64).clamp(0.0, 1.0);
        (ratio * logical_scrollable as f64).round() as usize
    } else {
        safe_scroll_top
            .round()
            .min(layout.logical_height.saturating_sub(viewport_height) as f64)
            .max(0.0) as usize
    };
    let overscan_px = TEXT_OVERSCAN_ROWS.saturating_mul(TEXT_ROW_HEIGHT);
    let start_px = logical_scroll_top.saturating_sub(overscan_px);
    let end_px = logical_scroll_top
        .saturating_add(viewport_height)
        .saturating_add(overscan_px)
        .min(layout.logical_height);
    let start_row = line_for_logical_top(&layout.line_tops, start_px);
    let end_row = layout
        .line_tops
        .partition_point(|top| *top < end_px)
        .saturating_add(1)
        .min(line_count);

    WrappedTextScroll {
        content_height,
        logical_scroll_top,
        display_scroll_top,
        start_row,
        end_row,
        scaled,
    }
}

fn line_for_logical_top(line_tops: &[usize], logical_top: usize) -> usize {
    line_tops
        .partition_point(|top| *top <= logical_top)
        .saturating_sub(1)
}

fn wrapped_text_row_top(line_top: usize, scroll: WrappedTextScroll) -> i64 {
    if !scroll.scaled {
        return line_top.min(i64::MAX as usize) as i64;
    }
    let top =
        scroll.display_scroll_top as i128 + line_top as i128 - scroll.logical_scroll_top as i128;
    top.clamp(0, i64::MAX as i128) as i64
}

fn safe_text_scroll_top(scroll_top: f64) -> f64 {
    if scroll_top.is_finite() && scroll_top > 0.0 {
        scroll_top
    } else {
        0.0
    }
}

#[component]
fn VirtualTextLineView(
    line_number: usize,
    line_text: String,
    on_begin_edit: EventHandler<usize>,
    on_select_start: EventHandler<()>,
) -> Element {
    let line_text_for_click = line_text.clone();

    rsx! {
        span {
            class: "virtual-text-line-view",
            tabindex: "0",
            "data-line-number": "{line_number}",
            onmousedown: move |_| on_select_start.call(()),
            onclick: move |event| {
                on_begin_edit.call(virtual_text_click_column(
                    event.element_coordinates().x,
                    &line_text_for_click,
                ));
            },
            "{line_text}"
        }
    }
}

#[component]
fn VirtualTextLineEditor(
    line_index: usize,
    line_number: usize,
    line_text: String,
    initial_column: usize,
    on_commit: EventHandler<(usize, String)>,
    on_exit: EventHandler<()>,
) -> Element {
    let mut draft = use_signal(|| line_text.clone());
    let mut source = use_signal(|| line_text.clone());
    let mut source_line_index = use_signal(|| line_index);
    let mut dirty = use_signal(|| false);
    let source_line_text = line_text.clone();

    use_effect(move || {
        if *source_line_index.peek() != line_index
            || source.peek().as_str() != source_line_text.as_str()
        {
            source_line_index.set(line_index);
            source.set(source_line_text.clone());
            draft.set(source_line_text.clone());
            dirty.set(false);
        }
    });

    let original_for_blur = line_text.clone();
    let original_for_key = line_text.clone();
    let on_commit_blur = on_commit;
    let on_commit_key = on_commit;
    let on_exit_blur = on_exit;
    let on_exit_key = on_exit;
    let focus_column = initial_column.min(line_text.len());

    rsx! {
        input {
            class: "virtual-text-line-editor",
            aria_label: "Line {line_number}",
            "data-line-number": "{line_number}",
            spellcheck: "false",
            value: "{draft}",
            onmounted: move |_| {
                focus_virtual_text_input(line_number, focus_column);
            },
            oninput: move |event| {
                dirty.set(true);
                draft.set(event.value());
            },
            onblur: move |_| {
                commit_virtual_text_line(
                    line_index,
                    original_for_blur.clone(),
                    draft,
                    dirty,
                    on_commit_blur,
                );
                on_exit_blur.call(());
            },
            onkeydown: move |event| {
                if event.key().to_string() == "Enter" {
                    event.prevent_default();
                    commit_virtual_text_line(
                        line_index,
                        original_for_key.clone(),
                        draft,
                        dirty,
                        on_commit_key,
                    );
                    on_exit_key.call(());
                }
            }
        }
    }
}

fn virtual_text_click_column(x: f64, line_text: &str) -> usize {
    const LINE_PADDING_LEFT: f64 = 12.0;
    const MONOSPACE_CHAR_WIDTH: f64 = 7.8;

    if !x.is_finite() {
        return 0;
    }

    let column = ((x - LINE_PADDING_LEFT) / MONOSPACE_CHAR_WIDTH).round();
    column.max(0.0).min(line_text.len() as f64) as usize
}

fn focus_virtual_text_input(line_number: usize, column: usize) {
    dioxus::document::eval(&format!(
        r#"
        (() => {{
            const input = document.querySelector(`.virtual-text-line-editor[data-line-number="{line_number}"]`);
            if (!input) return;
            requestAnimationFrame(() => {{
                try {{
                    input.focus({{ preventScroll: true }});
                }} catch (_error) {{
                    input.focus();
                }}
                const column = Math.max(0, Math.min({column}, input.value?.length ?? 0));
                input.setSelectionRange?.(column, column);
            }});
        }})();
        "#
    ));
}

fn commit_virtual_text_line(
    line_index: usize,
    original: String,
    draft: Signal<String>,
    mut dirty: Signal<bool>,
    on_commit: EventHandler<(usize, String)>,
) {
    let was_dirty = *dirty.read();
    if !was_dirty {
        return;
    }
    let next = draft.read().clone();
    dirty.set(false);
    if let Some(next) = virtual_text_line_commit_value(&original, &next, was_dirty) {
        on_commit.call((line_index, next));
    }
}

fn virtual_text_line_commit_value(original: &str, draft: &str, dirty: bool) -> Option<String> {
    if dirty && draft != original {
        Some(draft.to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::TEXT_ROW_HEIGHT;

    use super::{text_wrap_layout, virtual_text_line_commit_value};

    #[test]
    fn virtual_text_line_blur_without_input_does_not_commit() {
        assert_eq!(
            virtual_text_line_commit_value("current row", "stale reused draft", false),
            None
        );
    }

    #[test]
    fn virtual_text_line_input_matching_original_does_not_commit() {
        assert_eq!(virtual_text_line_commit_value("same", "same", true), None);
    }

    #[test]
    fn virtual_text_line_input_with_change_commits() {
        assert_eq!(
            virtual_text_line_commit_value("old", "new", true),
            Some("new".to_string())
        );
    }

    #[test]
    fn text_wrap_layout_expands_long_lines_for_narrow_viewports() {
        let text = format!("short\n{}", "a".repeat(200));
        let layout = text_wrap_layout(&text, &[0, 6], 220);

        assert_eq!(layout.line_tops.as_slice(), &[0, TEXT_ROW_HEIGHT]);
        assert_eq!(layout.line_heights[0], TEXT_ROW_HEIGHT);
        assert!(layout.line_heights[1] > TEXT_ROW_HEIGHT);
        assert!(layout.logical_height > TEXT_ROW_HEIGHT * 2);
    }
}
