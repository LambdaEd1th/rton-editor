use super::edit::HexEdit;
use crate::domain::byte_document::ByteRead;
use crate::i18n::I18n;

pub(crate) const HEX_SEARCH_MATCH_DISPLAY_LIMIT: usize = 5000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HexSearchMode {
    Hex,
    Ascii,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BytePattern {
    pub(crate) bytes: Vec<u8>,
    pub(crate) valid: bool,
    pub(crate) message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HexSearchMatch {
    pub(crate) offset: usize,
    pub(crate) length: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HexSearchResult {
    pub(crate) matches: Vec<HexSearchMatch>,
    pub(crate) capped: bool,
}

pub(crate) fn parse_search_pattern(mode: HexSearchMode, text: &str, i18n: I18n) -> BytePattern {
    match mode {
        HexSearchMode::Hex => parse_hex_pattern(text, false, i18n),
        HexSearchMode::Ascii => parse_ascii_pattern(text, false, i18n),
    }
}

pub(crate) fn parse_replace_pattern(mode: HexSearchMode, text: &str, i18n: I18n) -> BytePattern {
    match mode {
        HexSearchMode::Hex => parse_hex_pattern(text, true, i18n),
        HexSearchMode::Ascii => parse_ascii_pattern(text, true, i18n),
    }
}

pub(crate) fn parse_hex_pattern(text: &str, allow_empty: bool, i18n: I18n) -> BytePattern {
    let compact = text
        .trim()
        .replace("0x", "")
        .replace("0X", "")
        .chars()
        .filter(|ch| !matches!(ch, ' ' | '\n' | '\r' | '\t' | ',' | '_' | ':' | '-'))
        .collect::<String>();
    if compact.is_empty() {
        return BytePattern {
            bytes: Vec::new(),
            valid: allow_empty,
            message: if allow_empty {
                String::new()
            } else {
                i18n.t("hex-enter-hex")
            },
        };
    }
    if !compact.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return BytePattern {
            bytes: Vec::new(),
            valid: false,
            message: i18n.t("hex-hex-chars"),
        };
    }
    if compact.len() % 2 != 0 {
        return BytePattern {
            bytes: Vec::new(),
            valid: false,
            message: i18n.t("hex-hex-even"),
        };
    }
    let bytes = compact
        .as_bytes()
        .chunks(2)
        .filter_map(|pair| std::str::from_utf8(pair).ok())
        .filter_map(|pair| u8::from_str_radix(pair, 16).ok())
        .collect::<Vec<_>>();
    BytePattern {
        bytes,
        valid: true,
        message: String::new(),
    }
}

pub(crate) fn parse_ascii_pattern(text: &str, allow_empty: bool, i18n: I18n) -> BytePattern {
    if text.is_empty() {
        return BytePattern {
            bytes: Vec::new(),
            valid: allow_empty,
            message: if allow_empty {
                String::new()
            } else {
                i18n.t("hex-enter-ascii")
            },
        };
    }
    let mut bytes = Vec::with_capacity(text.len());
    for ch in text.chars() {
        let code = ch as u32;
        if code > 0xff {
            return BytePattern {
                bytes: Vec::new(),
                valid: false,
                message: i18n.t("hex-ascii-chars"),
            };
        }
        bytes.push(code as u8);
    }
    BytePattern {
        bytes,
        valid: true,
        message: String::new(),
    }
}

#[cfg(test)]
pub(crate) fn find_hex_search_matches<B: ByteRead + ?Sized>(
    bytes: &B,
    pattern: &[u8],
    ascii_insensitive: bool,
) -> Vec<HexSearchMatch> {
    find_hex_search_result(bytes, pattern, ascii_insensitive).matches
}

pub(crate) fn find_hex_search_result<B: ByteRead + ?Sized>(
    bytes: &B,
    pattern: &[u8],
    ascii_insensitive: bool,
) -> HexSearchResult {
    if pattern.is_empty() || bytes.len() < pattern.len() {
        return HexSearchResult {
            matches: Vec::new(),
            capped: false,
        };
    }
    let mut matches = Vec::new();
    let mut offset = 0;
    let mut capped = false;
    while offset + pattern.len() <= bytes.len() {
        if matches_at(bytes, offset, pattern, ascii_insensitive) {
            matches.push(HexSearchMatch {
                offset,
                length: pattern.len(),
            });
            if matches.len() >= HEX_SEARCH_MATCH_DISPLAY_LIMIT {
                capped = offset + pattern.len() < bytes.len();
                break;
            }
            offset += pattern.len().max(1);
        } else {
            offset += 1;
        }
    }
    HexSearchResult { matches, capped }
}

fn matches_at<B: ByteRead + ?Sized>(
    bytes: &B,
    offset: usize,
    pattern: &[u8],
    ascii_insensitive: bool,
) -> bool {
    if offset > bytes.len().saturating_sub(pattern.len()) {
        return false;
    }
    pattern.iter().enumerate().all(|(index, right)| {
        let Some(left) = bytes.byte_at(offset + index) else {
            return false;
        };
        if ascii_insensitive {
            left.eq_ignore_ascii_case(right)
        } else {
            left == *right
        }
    })
}

pub(crate) fn find_containing_match(
    matches: &[HexSearchMatch],
    offset: usize,
) -> Option<HexSearchMatch> {
    matches.iter().copied().find(|candidate| {
        offset >= candidate.offset && offset < candidate.offset + candidate.length
    })
}

pub(crate) fn relative_search_match(
    matches: &[HexSearchMatch],
    selected_offset: usize,
    current: Option<HexSearchMatch>,
    next: bool,
) -> Option<HexSearchMatch> {
    if matches.is_empty() {
        return None;
    }
    if next {
        let threshold = current
            .map(|match_| match_.offset)
            .unwrap_or(selected_offset.saturating_sub(1));
        matches
            .iter()
            .copied()
            .find(|match_| match_.offset > threshold)
            .or_else(|| matches.first().copied())
    } else {
        let threshold = current
            .map(|match_| match_.offset)
            .unwrap_or(selected_offset + 1);
        matches
            .iter()
            .rev()
            .copied()
            .find(|match_| match_.offset < threshold)
            .or_else(|| matches.last().copied())
    }
}

pub(crate) fn hex_search_status_text(
    query: &str,
    pattern: &BytePattern,
    matches: &[HexSearchMatch],
    current_match_index: Option<usize>,
    capped: bool,
    i18n: I18n,
) -> String {
    if query.is_empty() {
        return i18n.t("hex-type-search");
    }
    if !pattern.valid {
        return pattern.message.clone();
    }
    if matches.is_empty() {
        return i18n.t("hex-no-matches");
    }
    if let Some(index) = current_match_index {
        let total = if capped {
            format!("{}+", matches.len())
        } else {
            matches.len().to_string()
        };
        format!("{} / {}", index + 1, total)
    } else {
        let count = if capped {
            format!("{}+", matches.len())
        } else {
            matches.len().to_string()
        };
        i18n.t_args("hex-matches", &[("count", count)])
    }
}

pub(crate) fn replace_all_byte_edits<B: ByteRead + ?Sized>(
    bytes: &B,
    pattern: &[u8],
    replacement: &[u8],
    ascii_insensitive: bool,
) -> Option<Vec<HexEdit>> {
    if pattern.is_empty() || bytes.len() < pattern.len() {
        return None;
    }
    let mut offset = 0;
    let mut offset_delta = 0_i128;
    let mut edits = Vec::new();
    while offset < bytes.len() {
        if offset + pattern.len() <= bytes.len()
            && matches_at(bytes, offset, pattern, ascii_insensitive)
        {
            if replacement_changes_bytes(bytes, offset, pattern.len(), replacement) {
                edits.push(HexEdit {
                    offset: offset_with_delta(offset, offset_delta),
                    delete_length: pattern.len(),
                    insert: replacement.to_vec(),
                });
                offset_delta += replacement.len() as i128 - pattern.len() as i128;
            }
            offset += pattern.len();
        } else {
            offset += 1;
        }
    }
    (!edits.is_empty()).then_some(edits)
}

fn offset_with_delta(offset: usize, delta: i128) -> usize {
    if delta >= 0 {
        offset.saturating_add(delta.min(usize::MAX as i128) as usize)
    } else {
        offset.saturating_sub((-delta).min(usize::MAX as i128) as usize)
    }
}

fn replacement_changes_bytes<B: ByteRead + ?Sized>(
    bytes: &B,
    offset: usize,
    replaced_len: usize,
    replacement: &[u8],
) -> bool {
    replaced_len != replacement.len()
        || replacement
            .iter()
            .enumerate()
            .any(|(index, byte)| bytes.byte_at(offset + index) != Some(*byte))
}
