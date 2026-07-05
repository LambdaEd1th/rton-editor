use dioxus::prelude::*;

use crate::domain::{
    EditorMode, EditorTabState, HexEdit, TextBuffer, TextHistory, TextHistoryEntry,
    TextRangeHistoryEntry, TextRangeReplacement, can_record_text_undo, empty_editor_text,
    empty_tree_rows, prepare_hex_edits, push_hex_past_batch, push_hex_redo_batch,
    push_hex_undo_batch, push_text_past_range, push_text_past_snapshot, push_text_redo_range,
    push_text_redo_snapshot, push_text_undo_range, push_text_undo_snapshot, text_surface_from_text,
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

pub(crate) fn update_active_text_range(
    replacement: TextRangeReplacement,
    tabs: Signal<Vec<EditorTabState>>,
    active_tab_id: Signal<usize>,
) {
    let active_id = *active_tab_id.read();
    let outcome = {
        let tabs_snapshot = tabs.read();
        let Some(tab) = tabs_snapshot.iter().find(|tab| tab.id == active_id) else {
            return;
        };
        let Some(buffer) = tab.text_buffer.as_ref() else {
            return;
        };
        let Some(outcome) = text_with_replaced_range_and_history(buffer, &replacement) else {
            return;
        };
        if tab.editor_text.as_ref() == outcome.next_text.as_str() {
            return;
        }
        outcome
    };

    update_tab(tabs, active_id, |tab| {
        if tab.editor_text.as_ref() == outcome.next_text.as_str() {
            return;
        }
        push_text_undo_range(&mut tab.text_history, outcome.history);
        apply_text_surface(tab, outcome.next_text);
        tab.byte_doc = None;
        tab.doc = None;
        tab.tree_rows = empty_tree_rows();
        tab.search_result = None;
        tab.text_cache.clear();
        tab.task_state = None;
        tab.dirty = true;
    });
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
    match previous {
        TextHistoryEntry::Snapshot(previous_text) => {
            let current = tab.editor_text.to_string();
            apply_text_surface(tab, previous_text);
            push_text_redo_snapshot(&mut tab.text_history, current);
            clear_tab_parse_cache(tab);
            tab.byte_doc = None;
            tab.dirty = true;
        }
        TextHistoryEntry::Range(entry) => {
            if apply_text_history_range(tab, &entry.undo) {
                push_text_redo_range(&mut tab.text_history, entry);
            } else {
                tab.text_history.past.push(TextHistoryEntry::Range(entry));
            }
        }
    }
}

pub(crate) fn redo_text_tab(tab: &mut EditorTabState) {
    let Some(next) = tab.text_history.future.pop() else {
        return;
    };
    match next {
        TextHistoryEntry::Snapshot(next_text) => {
            let current = tab.editor_text.to_string();
            apply_text_surface(tab, next_text);
            push_text_past_snapshot(&mut tab.text_history, current);
            clear_tab_parse_cache(tab);
            tab.byte_doc = None;
            tab.dirty = true;
        }
        TextHistoryEntry::Range(entry) => {
            if apply_text_history_range(tab, &entry.redo) {
                push_text_past_range(&mut tab.text_history, entry);
            } else {
                tab.text_history.future.push(TextHistoryEntry::Range(entry));
            }
        }
    }
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

fn apply_text_history_range(tab: &mut EditorTabState, replacement: &TextRangeReplacement) -> bool {
    let Some(buffer) = tab.text_buffer.as_ref() else {
        return false;
    };
    let Some(next_text) = text_with_replaced_range(buffer, replacement) else {
        return false;
    };
    apply_text_surface(tab, next_text);
    clear_tab_parse_cache(tab);
    tab.byte_doc = None;
    tab.dirty = true;
    true
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

fn text_with_replaced_range(
    buffer: &TextBuffer,
    replacement: &TextRangeReplacement,
) -> Option<String> {
    text_with_replaced_range_and_history(buffer, replacement).map(|outcome| outcome.next_text)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TextRangeEditOutcome {
    next_text: String,
    history: TextRangeHistoryEntry,
}

fn text_with_replaced_range_and_history(
    buffer: &TextBuffer,
    replacement: &TextRangeReplacement,
) -> Option<TextRangeEditOutcome> {
    let text = buffer.text.as_ref();
    let start = text_position_to_byte_offset(
        buffer,
        replacement.start_line,
        replacement.start_column_utf16,
    )?;
    let end =
        text_position_to_byte_offset(buffer, replacement.end_line, replacement.end_column_utf16)?;
    let (start, end, start_point, end_point) = if start <= end {
        (
            start,
            end,
            (replacement.start_line, replacement.start_column_utf16),
            (replacement.end_line, replacement.end_column_utf16),
        )
    } else {
        (
            end,
            start,
            (replacement.end_line, replacement.end_column_utf16),
            (replacement.start_line, replacement.start_column_utf16),
        )
    };
    if start == end && replacement.replacement.is_empty() {
        return None;
    }

    let deleted_text = text[start..end].to_string();
    let mut next = String::with_capacity(
        text.len()
            .saturating_sub(end.saturating_sub(start))
            .saturating_add(replacement.replacement.len()),
    );
    next.push_str(&text[..start]);
    next.push_str(&replacement.replacement);
    next.push_str(&text[end..]);

    let undo_end_point = text_point_after_replacement(start_point, &replacement.replacement);
    Some(TextRangeEditOutcome {
        next_text: next,
        history: TextRangeHistoryEntry {
            undo: TextRangeReplacement {
                start_line: start_point.0,
                start_column_utf16: start_point.1,
                end_line: undo_end_point.0,
                end_column_utf16: undo_end_point.1,
                replacement: deleted_text,
            },
            redo: TextRangeReplacement {
                start_line: start_point.0,
                start_column_utf16: start_point.1,
                end_line: end_point.0,
                end_column_utf16: end_point.1,
                replacement: replacement.replacement.clone(),
            },
        },
    })
}

fn text_position_to_byte_offset(
    buffer: &TextBuffer,
    line_index: usize,
    column_utf16: usize,
) -> Option<usize> {
    let text = buffer.text.as_ref();
    let start = *buffer.line_offsets.get(line_index)?;
    let full_end = buffer
        .line_offsets
        .get(line_index + 1)
        .copied()
        .unwrap_or(text.len());
    let content_end = line_content_end(text, start, full_end);
    let line = &text[start..content_end];
    let mut utf16_offset = 0_usize;

    for (byte_offset, ch) in line.char_indices() {
        let next_utf16_offset = utf16_offset.saturating_add(ch.len_utf16());
        if next_utf16_offset > column_utf16 {
            return Some(start + byte_offset);
        }
        utf16_offset = next_utf16_offset;
    }

    Some(content_end)
}

fn text_point_after_replacement(start_point: (usize, usize), replacement: &str) -> (usize, usize) {
    let line_breaks = replacement
        .as_bytes()
        .iter()
        .filter(|byte| **byte == b'\n')
        .count();
    if line_breaks == 0 {
        return (
            start_point.0,
            start_point
                .1
                .saturating_add(replacement.encode_utf16().count()),
        );
    }

    let tail = replacement.rsplit('\n').next().unwrap_or_default();
    let tail = tail.strip_suffix('\r').unwrap_or(tail);
    (
        start_point.0.saturating_add(line_breaks),
        tail.encode_utf16().count(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn virtual_text_range_delete_crosses_lines() {
        let buffer = TextBuffer::new("one\ntwo\nthree".to_string());
        let replacement = TextRangeReplacement {
            start_line: 0,
            start_column_utf16: 1,
            end_line: 2,
            end_column_utf16: 2,
            replacement: String::new(),
        };

        assert_eq!(
            text_with_replaced_range(&buffer, &replacement).as_deref(),
            Some("oree")
        );
    }

    #[test]
    fn virtual_text_range_delete_preserves_utf8_boundaries() {
        let buffer = TextBuffer::new("a😀b\n中文c".to_string());
        let replacement = TextRangeReplacement {
            start_line: 0,
            start_column_utf16: 1,
            end_line: 1,
            end_column_utf16: 2,
            replacement: String::new(),
        };

        assert_eq!(
            text_with_replaced_range(&buffer, &replacement).as_deref(),
            Some("ac")
        );
    }

    #[test]
    fn virtual_text_range_history_undo_redo_restores_single_line_edit() {
        let buffer = TextBuffer::new("one\ntwo\nthree".to_string());
        let replacement = TextRangeReplacement {
            start_line: 1,
            start_column_utf16: 1,
            end_line: 1,
            end_column_utf16: 2,
            replacement: "W".to_string(),
        };

        let outcome = text_with_replaced_range_and_history(&buffer, &replacement)
            .expect("range edit outcome");
        assert_eq!(outcome.next_text, "one\ntWo\nthree");

        let undo_buffer = TextBuffer::new(outcome.next_text.clone());
        assert_eq!(
            text_with_replaced_range(&undo_buffer, &outcome.history.undo).as_deref(),
            Some("one\ntwo\nthree")
        );

        let redo_buffer = TextBuffer::new("one\ntwo\nthree".to_string());
        assert_eq!(
            text_with_replaced_range(&redo_buffer, &outcome.history.redo).as_deref(),
            Some("one\ntWo\nthree")
        );
    }

    #[test]
    fn virtual_text_range_history_undo_redo_restores_multiline_insert() {
        let buffer = TextBuffer::new("a😀b\n中文c".to_string());
        let replacement = TextRangeReplacement {
            start_line: 0,
            start_column_utf16: 3,
            end_line: 0,
            end_column_utf16: 3,
            replacement: "X\nY😀".to_string(),
        };

        let outcome = text_with_replaced_range_and_history(&buffer, &replacement)
            .expect("range edit outcome");
        assert_eq!(outcome.next_text, "a😀X\nY😀b\n中文c");
        assert_eq!(outcome.history.undo.start_line, 0);
        assert_eq!(outcome.history.undo.start_column_utf16, 3);
        assert_eq!(outcome.history.undo.end_line, 1);
        assert_eq!(outcome.history.undo.end_column_utf16, 3);

        let undo_buffer = TextBuffer::new(outcome.next_text.clone());
        assert_eq!(
            text_with_replaced_range(&undo_buffer, &outcome.history.undo).as_deref(),
            Some("a😀b\n中文c")
        );
    }

    #[test]
    fn text_range_history_works_above_snapshot_limit() {
        let mut tab = EditorTabState {
            id: 1,
            file_name: "large.json".to_string(),
            doc: None,
            byte_doc: None,
            tree_rows: empty_tree_rows(),
            search_result: None,
            editor_text: std::sync::Arc::from(format!("{}z", "x".repeat(1024 * 1024 + 1))),
            text_buffer: None,
            mode: EditorMode::Json,
            search_query: String::new(),
            selected_path: "$".to_string(),
            text_history: TextHistory::default(),
            hex_history: crate::domain::HexHistory::default(),
            text_state: crate::domain::TextContentState::None,
            task_state: None,
            task_generation: 0,
            tree_generation: 0,
            search_generation: 0,
            expanded_paths: crate::domain::default_expanded_paths(),
            text_cache: Vec::new(),
            dirty: false,
        };
        let initial_text = tab.editor_text.to_string();
        apply_text_surface(&mut tab, initial_text);
        let replacement = TextRangeReplacement {
            start_line: 0,
            start_column_utf16: 0,
            end_line: 0,
            end_column_utf16: 1,
            replacement: "y".to_string(),
        };
        let outcome =
            text_with_replaced_range_and_history(tab.text_buffer.as_ref().unwrap(), &replacement)
                .expect("range edit outcome");
        push_text_undo_range(&mut tab.text_history, outcome.history);
        apply_text_surface(&mut tab, outcome.next_text);

        assert!(tab_can_undo(&tab));
        undo_text_tab(&mut tab);
        assert!(tab.editor_text.starts_with('x'));
        assert!(tab_can_redo(&tab));
        redo_text_tab(&mut tab);
        assert!(tab.editor_text.starts_with('y'));
    }
}
