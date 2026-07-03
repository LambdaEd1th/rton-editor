use dioxus::prelude::*;

use crate::domain::{ByteDocument, HexSearchMatch};

use super::logic::{
    byte_to_ascii, display_hex_cell, hex_ascii_class, hex_byte_class, to_offset_hex,
};
use super::state::{ByteSelection, HEX_BYTES_PER_ROW, HexPane, PendingHexEdit};

#[component]
pub(super) fn HexRow(
    bytes: ByteDocument,
    row_index: usize,
    row_top: i64,
    offset_width: usize,
    selected_offset: usize,
    normalized_selection: Option<ByteSelection>,
    pending_hex_edit: Option<PendingHexEdit>,
    search_matches: Vec<HexSearchMatch>,
    current_match: Option<HexSearchMatch>,
    on_select: EventHandler<(usize, HexPane, bool)>,
    on_enter: EventHandler<(usize, HexPane)>,
) -> Element {
    let row_start = row_index * HEX_BYTES_PER_ROW;
    let row_offsets = (row_start..row_start + HEX_BYTES_PER_ROW).collect::<Vec<_>>();

    rsx! {
        div {
            class: "rton-hex-row",
            style: "transform: translateY({row_top}px)",
            button {
                r#type: "button",
                class: "rton-hex-offset rton-hex-offset-button",
                onclick: move |_| on_select.call((row_start.min(bytes.len().saturating_sub(1)), HexPane::Hex, false)),
                "{to_offset_hex(row_start, offset_width - 2)}"
            }
            div { class: "rton-hex-grid",
                for offset in row_offsets.clone() {
                    if offset >= bytes.len() {
                        span { class: "rton-hex-byte-placeholder" }
                    } else {
                        button {
                            r#type: "button",
                            class: hex_byte_class(offset, selected_offset, normalized_selection, &search_matches, current_match),
                            aria_label: "Byte {to_offset_hex(offset, offset_width - 2)}",
                            onmousedown: move |event| {
                                event.prevent_default();
                                on_select.call((offset, HexPane::Hex, event.modifiers().shift()));
                            },
                            onmouseenter: move |_| on_enter.call((offset, HexPane::Hex)),
                            "{display_hex_cell(&bytes, offset, pending_hex_edit.as_ref())}"
                        }
                    }
                }
            }
            div { class: "rton-hex-ascii",
                for offset in row_offsets {
                    if offset >= bytes.len() {
                        span { class: "rton-hex-ascii-placeholder" }
                    } else {
                        button {
                            r#type: "button",
                            class: hex_ascii_class(offset, selected_offset, normalized_selection, &search_matches, current_match),
                            aria_label: "ASCII byte {to_offset_hex(offset, offset_width - 2)}",
                            onmousedown: move |event| {
                                event.prevent_default();
                                on_select.call((offset, HexPane::Ascii, event.modifiers().shift()));
                            },
                            onmouseenter: move |_| on_enter.call((offset, HexPane::Ascii)),
                            "{byte_to_ascii(bytes.byte_at(offset).unwrap_or_default())}"
                        }
                    }
                }
            }
        }
    }
}
