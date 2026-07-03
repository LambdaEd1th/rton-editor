use dioxus::prelude::*;

use crate::domain::{
    EditorMode, EditorTabState, HexEdit, TextBuffer, TextHistory, can_record_text_undo,
    empty_editor_text, empty_tree_rows, prepare_hex_edits, push_hex_past_batch,
    push_hex_redo_batch, push_hex_undo_batch, push_text_past_snapshot, push_text_redo_snapshot,
    push_text_undo_snapshot, text_surface_from_text,
};

use super::tabs::update_tab;

pub(crate) fn update_active_text(
    text: String,
    tabs: Signal<Vec<EditorTabState>>,
    active_tab_id: Signal<usize>,
) {
    let active_id = *active_tab_id.read();
    update_tab(tabs, active_id, |tab| {
        if tab.editor_text.as_ref() == text.as_str() {
            return;
        }
        if can_record_text_undo(tab.editor_text.as_ref(), &text) {
            push_text_undo_snapshot(&mut tab.text_history, tab.editor_text.to_string());
        } else {
            tab.text_history = TextHistory::default();
        }
        apply_text_surface(tab, text);
        tab.byte_doc = None;
        tab.doc = None;
        tab.tree_rows = empty_tree_rows();
        tab.search_result = None;
        tab.text_cache.clear();
        tab.task_state = None;
        tab.dirty = true;
    });
}

pub(crate) fn update_active_text_line(
    line_index: usize,
    replacement: String,
    tabs: Signal<Vec<EditorTabState>>,
    active_tab_id: Signal<usize>,
) {
    let active_id = *active_tab_id.read();
    let next_text = {
        let tabs_snapshot = tabs.read();
        let Some(tab) = tabs_snapshot.iter().find(|tab| tab.id == active_id) else {
            return;
        };
        let Some(buffer) = tab.text_buffer.as_ref() else {
            return;
        };
        let Some(next_text) = text_with_replaced_line(buffer, line_index, &replacement) else {
            return;
        };
        if tab.editor_text.as_ref() == next_text.as_str() {
            return;
        }
        next_text
    };

    update_active_text(next_text, tabs, active_tab_id);
}

pub(crate) fn update_active_hex_edits(
    edits: Vec<HexEdit>,
    tabs: Signal<Vec<EditorTabState>>,
    active_tab_id: Signal<usize>,
) {
    if edits.is_empty() {
        return;
    }
    let active_id = *active_tab_id.read();
    update_tab(tabs, active_id, |tab| {
        let Some(byte_doc) = tab.byte_doc.clone() else {
            return;
        };
        let Some((effective_edits, undo)) = prepare_hex_edits(&byte_doc, edits) else {
            return;
        };

        push_hex_undo_batch(&mut tab.hex_history, undo);
        tab.byte_doc = Some(byte_doc.apply_edits(&effective_edits));
        tab.doc = None;
        tab.tree_rows = empty_tree_rows();
        tab.search_result = None;
        tab.editor_text = empty_editor_text();
        tab.text_buffer = None;
        tab.text_state = crate::domain::TextContentState::None;
        tab.text_cache.clear();
        tab.task_state = None;
        tab.dirty = true;
    });
}

pub(crate) fn undo_active_edit(tabs: Signal<Vec<EditorTabState>>, active_tab_id: Signal<usize>) {
    let active_id = *active_tab_id.read();
    update_tab(tabs, active_id, |tab| match tab.mode {
        EditorMode::RtonHex => undo_hex_tab(tab),
        EditorMode::Json | EditorMode::Yaml | EditorMode::Toml => undo_text_tab(tab),
    });
}

pub(crate) fn redo_active_edit(tabs: Signal<Vec<EditorTabState>>, active_tab_id: Signal<usize>) {
    let active_id = *active_tab_id.read();
    update_tab(tabs, active_id, |tab| match tab.mode {
        EditorMode::RtonHex => redo_hex_tab(tab),
        EditorMode::Json | EditorMode::Yaml | EditorMode::Toml => redo_text_tab(tab),
    });
}

pub(crate) fn tab_can_undo(tab: &EditorTabState) -> bool {
    match tab.mode {
        EditorMode::RtonHex => !tab.hex_history.past.is_empty(),
        EditorMode::Json | EditorMode::Yaml | EditorMode::Toml => !tab.text_history.past.is_empty(),
    }
}

pub(crate) fn tab_can_redo(tab: &EditorTabState) -> bool {
    match tab.mode {
        EditorMode::RtonHex => !tab.hex_history.future.is_empty(),
        EditorMode::Json | EditorMode::Yaml | EditorMode::Toml => {
            !tab.text_history.future.is_empty()
        }
    }
}

pub(crate) fn undo_text_tab(tab: &mut EditorTabState) {
    let Some(previous) = tab.text_history.past.pop() else {
        return;
    };
    let current = tab.editor_text.to_string();
    apply_text_surface(tab, previous);
    push_text_redo_snapshot(&mut tab.text_history, current);
    clear_tab_parse_cache(tab);
    tab.byte_doc = None;
    tab.dirty = true;
}

pub(crate) fn redo_text_tab(tab: &mut EditorTabState) {
    let Some(next) = tab.text_history.future.pop() else {
        return;
    };
    let current = tab.editor_text.to_string();
    apply_text_surface(tab, next);
    push_text_past_snapshot(&mut tab.text_history, current);
    clear_tab_parse_cache(tab);
    tab.byte_doc = None;
    tab.dirty = true;
}

pub(crate) fn undo_hex_tab(tab: &mut EditorTabState) {
    let Some(previous) = tab.hex_history.past.pop() else {
        return;
    };
    let Some(byte_doc) = tab.byte_doc.clone() else {
        tab.hex_history.past.push(previous);
        return;
    };
    let edits = previous.undo_edits();
    tab.byte_doc = Some(byte_doc.apply_edits(&edits));
    push_hex_redo_batch(&mut tab.hex_history, previous);
    clear_tab_parse_cache(tab);
    tab.editor_text = empty_editor_text();
    tab.text_buffer = None;
    tab.text_state = crate::domain::TextContentState::None;
    tab.dirty = true;
}

pub(crate) fn redo_hex_tab(tab: &mut EditorTabState) {
    let Some(next) = tab.hex_history.future.pop() else {
        return;
    };
    let Some(byte_doc) = tab.byte_doc.clone() else {
        tab.hex_history.future.push(next);
        return;
    };
    let edits = next.redo_edits();
    tab.byte_doc = Some(byte_doc.apply_edits(&edits));
    push_hex_past_batch(&mut tab.hex_history, next);
    clear_tab_parse_cache(tab);
    tab.editor_text = empty_editor_text();
    tab.text_buffer = None;
    tab.text_state = crate::domain::TextContentState::None;
    tab.dirty = true;
}

pub(crate) fn clear_tab_parse_cache(tab: &mut EditorTabState) {
    tab.doc = None;
    tab.tree_rows = empty_tree_rows();
    tab.search_result = None;
    tab.selected_path = "$".to_string();
    tab.text_cache.clear();
    tab.task_state = None;
}

fn apply_text_surface(tab: &mut EditorTabState, text: String) {
    let Some(format) = tab.mode.text_format() else {
        tab.editor_text = std::sync::Arc::from(text);
        tab.text_buffer = None;
        tab.text_state = crate::domain::TextContentState::None;
        return;
    };
    let surface = text_surface_from_text(text, format);
    tab.editor_text = surface.editor_text;
    tab.text_buffer = surface.text_buffer;
    tab.text_state = surface.text_state;
}

fn text_with_replaced_line(
    buffer: &TextBuffer,
    line_index: usize,
    replacement: &str,
) -> Option<String> {
    let text = buffer.text.as_ref();
    let start = *buffer.line_offsets.get(line_index)?;
    let full_end = buffer
        .line_offsets
        .get(line_index + 1)
        .copied()
        .unwrap_or(text.len());
    let content_end = line_content_end(text, start, full_end);
    let mut next = String::with_capacity(text.len() + replacement.len());
    next.push_str(&text[..start]);
    next.push_str(replacement);
    next.push_str(&text[content_end..full_end]);
    next.push_str(&text[full_end..]);
    Some(next)
}

fn line_content_end(text: &str, start: usize, full_end: usize) -> usize {
    let bytes = text.as_bytes();
    let mut end = full_end;
    if end > start && bytes.get(end - 1) == Some(&b'\n') {
        end -= 1;
        if end > start && bytes.get(end - 1) == Some(&b'\r') {
            end -= 1;
        }
    }
    end
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn virtual_text_line_edit_preserves_lf_line_endings() {
        let buffer = TextBuffer::new("one\ntwo\nthree".to_string());

        assert_eq!(
            text_with_replaced_line(&buffer, 1, "TWO").as_deref(),
            Some("one\nTWO\nthree")
        );
    }

    #[test]
    fn virtual_text_line_edit_preserves_crlf_line_endings() {
        let buffer = TextBuffer::new("one\r\ntwo\r\nthree".to_string());

        assert_eq!(
            text_with_replaced_line(&buffer, 1, "TWO").as_deref(),
            Some("one\r\nTWO\r\nthree")
        );
    }
}
