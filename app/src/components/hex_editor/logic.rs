use dioxus::prelude::*;

use crate::domain::{
    ByteDocument, HexEdit, HexSearchMatch, edited_len_after_edits, find_containing_match,
    measured_hex_viewport_height, prepare_hex_edits,
};

use super::state::{
    ByteSelection, HEX_INSPECTOR_MAX_WIDTH, HEX_INSPECTOR_MIN_WIDTH, PendingHexEdit,
};

#[derive(Clone, Copy)]
pub(super) struct HexCommitTargets {
    pub(super) on_change: EventHandler<Vec<HexEdit>>,
    pub(super) pending_hex_edit: Signal<Option<PendingHexEdit>>,
    pub(super) selection_range: Signal<Option<ByteSelection>>,
    pub(super) selected_offset: Signal<usize>,
}

pub(super) fn update_hex_viewport_height(mut viewport_height: Signal<usize>, height: f64) {
    let next = measured_hex_viewport_height(height);
    if next != *viewport_height.read() {
        viewport_height.set(next);
    }
}

pub(super) fn scroll_hex_editor_to(scroll_top: f64) {
    let scroll_top = if scroll_top.is_finite() && scroll_top > 0.0 {
        scroll_top
    } else {
        0.0
    };
    dioxus::document::eval(&format!(
        r#"
        const scroller = document.querySelector('.rton-hex-scroll');
        if (scroller) {{
            scroller.scrollTop = {scroll_top};
        }}
        document.querySelector('.rton-hex-editor')?.focus?.();
        "#
    ));
}

pub(super) fn hex_offset_width(byte_len: usize) -> usize {
    let max_offset = byte_len.saturating_sub(1);
    max_offset
        .to_string()
        .len()
        .max(format!("{max_offset:X}").len())
        .max(8)
        + 2
}

pub(super) fn normalize_selection(
    selection: Option<ByteSelection>,
    bytes_len: usize,
) -> Option<ByteSelection> {
    let selection = selection?;
    if bytes_len == 0 {
        return None;
    }
    let anchor = selection.anchor.min(bytes_len - 1);
    let focus = selection.focus.min(bytes_len - 1);
    (anchor != focus).then_some(ByteSelection { anchor, focus })
}

pub(super) fn to_offset_hex(offset: usize, width: usize) -> String {
    format!("{offset:0width$X}")
}

pub(super) fn byte_to_hex(byte: u8) -> String {
    format!("{byte:02X}")
}

pub(super) fn byte_to_ascii(byte: u8) -> char {
    if (0x20..=0x7e).contains(&byte) {
        byte as char
    } else {
        '.'
    }
}

pub(super) fn display_hex_cell(
    bytes: &ByteDocument,
    offset: usize,
    pending: Option<&PendingHexEdit>,
) -> String {
    pending
        .filter(|pending| pending.offset == offset)
        .map(|pending| pending.text.clone())
        .unwrap_or_else(|| byte_to_hex(bytes.byte_at(offset).unwrap_or_default()))
}

pub(super) fn is_hex_key(key: &str) -> bool {
    key.len() == 1 && key.chars().all(|ch| ch.is_ascii_hexdigit())
}

pub(super) fn key_to_latin1_byte(key: &str) -> Option<u8> {
    let mut chars = key.chars();
    let ch = chars.next()?;
    if chars.next().is_some() {
        return None;
    }
    let code = ch as u32;
    (code <= 0xff).then_some(code as u8)
}

pub(super) fn commit_hex_edit(
    current: &ByteDocument,
    edit: HexEdit,
    next_focus_offset: usize,
    targets: HexCommitTargets,
) {
    commit_hex_edits(current, vec![edit], next_focus_offset, targets);
}

pub(super) fn commit_hex_edits(
    current: &ByteDocument,
    edits: Vec<HexEdit>,
    next_focus_offset: usize,
    mut targets: HexCommitTargets,
) {
    let Some((effective_edits, _undo)) = prepare_hex_edits(current, edits) else {
        return;
    };

    let next_len = edited_len_after_edits(current.len(), &effective_edits);
    targets.on_change.call(effective_edits);
    targets.pending_hex_edit.set(None);
    targets.selection_range.set(None);
    targets.selected_offset.set(if next_len == 0 {
        0
    } else {
        next_focus_offset.min(next_len - 1)
    });
}

pub(super) fn commit_hex_replace_span(
    current: &ByteDocument,
    offset: usize,
    delete_length: usize,
    values: Vec<u8>,
    targets: HexCommitTargets,
) {
    let offset = offset.min(current.len());
    let focus = if values.is_empty() {
        offset.saturating_sub(1)
    } else {
        offset + values.len().saturating_sub(1)
    };
    commit_hex_edit(
        current,
        HexEdit {
            offset,
            delete_length,
            insert: values,
        },
        focus,
        targets,
    );
}

pub(super) fn commit_hex_set_or_insert(
    current: &ByteDocument,
    offset: usize,
    values: Vec<u8>,
    insert_mode: bool,
    targets: HexCommitTargets,
) {
    let offset = offset.min(current.len());
    let delete_length = if insert_mode {
        0
    } else {
        values.len().min(current.len().saturating_sub(offset))
    };
    let focus = offset + values.len().saturating_sub(1);
    commit_hex_edit(
        current,
        HexEdit {
            offset,
            delete_length,
            insert: values,
        },
        focus,
        targets,
    );
}

pub(super) fn commit_hex_replace_selection(
    current: &ByteDocument,
    bytes_len: usize,
    values: Vec<u8>,
    targets: HexCommitTargets,
) -> bool {
    let Some(selection) = normalize_selection(*targets.selection_range.read(), bytes_len) else {
        return false;
    };
    let start = selection.anchor.min(selection.focus);
    let length = selection.anchor.max(selection.focus) - start + 1;
    commit_hex_replace_span(current, start, length, values, targets);
    true
}

pub(super) fn set_hex_focus(
    offset: isize,
    extend_selection: bool,
    bytes_len: usize,
    mut selection_range: Signal<Option<ByteSelection>>,
    mut selection_anchor: Signal<usize>,
    mut selected_offset: Signal<usize>,
) {
    if bytes_len == 0 {
        selected_offset.set(0);
        selection_range.set(None);
        return;
    }
    let next = offset.clamp(0, bytes_len.saturating_sub(1) as isize) as usize;
    if extend_selection {
        let anchor = selection_range
            .read()
            .as_ref()
            .map(|selection| selection.anchor)
            .unwrap_or(*selection_anchor.read());
        selection_range.set(if anchor == next {
            None
        } else {
            Some(ByteSelection {
                anchor,
                focus: next,
            })
        });
    } else {
        selection_anchor.set(next);
        selection_range.set(None);
    }
    selected_offset.set(next);
}

pub(super) fn hex_byte_class(
    offset: usize,
    selected_offset: usize,
    selection: Option<ByteSelection>,
    matches: &[HexSearchMatch],
    current_match: Option<HexSearchMatch>,
) -> String {
    let mut class_name = "rton-hex-byte".to_string();
    append_hex_cell_state(
        &mut class_name,
        offset,
        selected_offset,
        selection,
        matches,
        current_match,
    );
    class_name
}

pub(super) fn hex_ascii_class(
    offset: usize,
    selected_offset: usize,
    selection: Option<ByteSelection>,
    matches: &[HexSearchMatch],
    current_match: Option<HexSearchMatch>,
) -> String {
    let mut class_name = "rton-hex-ascii-char".to_string();
    append_hex_cell_state(
        &mut class_name,
        offset,
        selected_offset,
        selection,
        matches,
        current_match,
    );
    class_name
}

pub(super) fn append_hex_cell_state(
    class_name: &mut String,
    offset: usize,
    selected_offset: usize,
    selection: Option<ByteSelection>,
    matches: &[HexSearchMatch],
    current_match: Option<HexSearchMatch>,
) {
    if let Some(selection) = selection {
        let start = selection.anchor.min(selection.focus);
        let end = selection.anchor.max(selection.focus);
        if (start..=end).contains(&offset) {
            class_name.push_str(" is-block-selected");
        }
    }
    if offset == selected_offset {
        class_name.push_str(" is-selected");
    }
    if find_containing_match(matches, offset).is_some() {
        class_name.push_str(" is-search-match");
    }
    if current_match
        .is_some_and(|match_| offset >= match_.offset && offset < match_.offset + match_.length)
    {
        class_name.push_str(" is-current-match");
    }
}

pub(super) fn clamp_hex_inspector_width(width: f64) -> i32 {
    width.round().clamp(
        HEX_INSPECTOR_MIN_WIDTH as f64,
        HEX_INSPECTOR_MAX_WIDTH as f64,
    ) as i32
}
