use crate::app_constants::TEXT_SEARCH_MATCH_DISPLAY_LIMIT;
use crate::i18n::I18n;

#[cfg(not(target_arch = "wasm32"))]
const PARALLEL_TEXT_SEARCH_MIN_BYTES: usize = 512 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TextSearchMatch {
    pub(crate) start: usize,
    pub(crate) end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TextSearchResult {
    pub(crate) matches: Vec<TextSearchMatch>,
    pub(crate) capped: bool,
}

pub(crate) fn find_text_search_result(
    text: &str,
    query: &str,
    case_sensitive: bool,
) -> TextSearchResult {
    if query.is_empty() || text.len() < query.len() {
        return TextSearchResult {
            matches: Vec::new(),
            capped: false,
        };
    }

    if case_sensitive && should_parallelize_ascii_text_search(text, query) {
        collect_ascii_case_sensitive_text_search_matches_parallel(text, query)
    } else if case_sensitive {
        collect_text_search_matches(text, query)
    } else if should_parallelize_ascii_text_search(text, query) {
        collect_ascii_case_insensitive_text_search_matches_parallel(text, query)
    } else {
        collect_ascii_case_insensitive_text_search_matches(text, query)
    }
}

fn should_parallelize_ascii_text_search(text: &str, query: &str) -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    {
        text.len() >= PARALLEL_TEXT_SEARCH_MIN_BYTES && query.is_ascii()
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (text, query);
        false
    }
}

fn collect_text_search_matches(haystack: &str, needle: &str) -> TextSearchResult {
    let mut matches = Vec::new();
    let mut offset = 0;
    let mut capped = false;

    while offset <= haystack.len() {
        let Some(relative_start) = haystack[offset..].find(needle) else {
            break;
        };
        let start = offset + relative_start;
        let end = start + needle.len();
        matches.push(TextSearchMatch { start, end });
        if matches.len() >= TEXT_SEARCH_MATCH_DISPLAY_LIMIT {
            capped = end < haystack.len() && haystack[end..].contains(needle);
            break;
        }
        offset = end;
    }

    TextSearchResult { matches, capped }
}

fn collect_ascii_case_insensitive_text_search_matches(
    haystack: &str,
    needle: &str,
) -> TextSearchResult {
    let haystack_bytes = haystack.as_bytes();
    let needle_bytes = needle.as_bytes();
    let mut matches = Vec::new();
    let mut offset = 0;
    let mut capped = false;

    while offset + needle_bytes.len() <= haystack_bytes.len() {
        let Some(start) =
            find_ascii_case_insensitive_match(haystack, haystack_bytes, needle_bytes, offset)
        else {
            break;
        };
        let end = start + needle_bytes.len();
        matches.push(TextSearchMatch { start, end });
        if matches.len() >= TEXT_SEARCH_MATCH_DISPLAY_LIMIT {
            capped = find_ascii_case_insensitive_match(haystack, haystack_bytes, needle_bytes, end)
                .is_some();
            break;
        }
        offset = end;
    }

    TextSearchResult { matches, capped }
}

#[cfg(not(target_arch = "wasm32"))]
fn collect_ascii_case_sensitive_text_search_matches_parallel(
    haystack: &str,
    needle: &str,
) -> TextSearchResult {
    use rayon::prelude::*;

    let haystack_bytes = haystack.as_bytes();
    let needle_bytes = needle.as_bytes();
    let max_start = haystack_bytes.len() - needle_bytes.len();
    let chunk_count = std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(2)
        .max(1);
    let chunk_size = (max_start + 1).div_ceil(chunk_count).max(1);
    let mut chunks = (0..=max_start)
        .step_by(chunk_size)
        .collect::<Vec<_>>()
        .into_par_iter()
        .map(|start| {
            let end = (start + chunk_size).min(max_start + 1);
            collect_ascii_case_sensitive_text_search_chunk(
                haystack,
                haystack_bytes,
                needle_bytes,
                start,
                end,
            )
        })
        .collect::<Vec<_>>();

    chunks.sort_by_key(|chunk| chunk.start);
    finalize_parallel_text_search_chunks(chunks, haystack.len())
}

#[cfg(target_arch = "wasm32")]
fn collect_ascii_case_sensitive_text_search_matches_parallel(
    haystack: &str,
    needle: &str,
) -> TextSearchResult {
    collect_text_search_matches(haystack, needle)
}

#[cfg(not(target_arch = "wasm32"))]
fn collect_ascii_case_insensitive_text_search_matches_parallel(
    haystack: &str,
    needle: &str,
) -> TextSearchResult {
    use rayon::prelude::*;

    let haystack_bytes = haystack.as_bytes();
    let needle_bytes = needle.as_bytes();
    let max_start = haystack_bytes.len() - needle_bytes.len();
    let chunk_count = std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(2)
        .max(1);
    let chunk_size = (max_start + 1).div_ceil(chunk_count).max(1);
    let mut chunks = (0..=max_start)
        .step_by(chunk_size)
        .collect::<Vec<_>>()
        .into_par_iter()
        .map(|start| {
            let end = (start + chunk_size).min(max_start + 1);
            collect_ascii_case_insensitive_text_search_chunk(
                haystack,
                haystack_bytes,
                needle_bytes,
                start,
                end,
            )
        })
        .collect::<Vec<_>>();

    chunks.sort_by_key(|chunk| chunk.start);
    finalize_parallel_text_search_chunks(chunks, haystack.len())
}

#[cfg(target_arch = "wasm32")]
fn collect_ascii_case_insensitive_text_search_matches_parallel(
    haystack: &str,
    needle: &str,
) -> TextSearchResult {
    collect_ascii_case_insensitive_text_search_matches(haystack, needle)
}

#[cfg(not(target_arch = "wasm32"))]
fn finalize_parallel_text_search_chunks(
    chunks: Vec<TextSearchChunkResult>,
    haystack_len: usize,
) -> TextSearchResult {
    let mut raw_matches = Vec::new();
    let mut capped = false;
    for chunk in chunks {
        capped |= chunk.capped;
        raw_matches.extend(chunk.matches);
    }
    raw_matches.sort_by_key(|match_| match_.start);
    let mut matches = Vec::new();
    let mut next_allowed = 0usize;
    for match_ in raw_matches {
        if match_.start < next_allowed {
            continue;
        }
        next_allowed = match_.end;
        matches.push(match_);
        if matches.len() >= TEXT_SEARCH_MATCH_DISPLAY_LIMIT {
            capped = next_allowed < haystack_len;
            break;
        }
    }
    if matches.len() > TEXT_SEARCH_MATCH_DISPLAY_LIMIT {
        matches.truncate(TEXT_SEARCH_MATCH_DISPLAY_LIMIT);
        capped = true;
    }
    TextSearchResult { matches, capped }
}

#[cfg(not(target_arch = "wasm32"))]
struct TextSearchChunkResult {
    start: usize,
    matches: Vec<TextSearchMatch>,
    capped: bool,
}

#[cfg(not(target_arch = "wasm32"))]
fn collect_ascii_case_sensitive_text_search_chunk(
    haystack: &str,
    haystack_bytes: &[u8],
    needle_bytes: &[u8],
    start: usize,
    end: usize,
) -> TextSearchChunkResult {
    let mut matches = Vec::new();
    let mut offset = start;
    let mut capped = false;
    while offset < end {
        if ascii_case_sensitive_matches_at(haystack, haystack_bytes, needle_bytes, offset) {
            let match_end = offset + needle_bytes.len();
            matches.push(TextSearchMatch {
                start: offset,
                end: match_end,
            });
            if matches.len() >= TEXT_SEARCH_MATCH_DISPLAY_LIMIT {
                capped = match_end < haystack.len();
                break;
            }
            offset = match_end;
        } else {
            offset += 1;
        }
    }
    TextSearchChunkResult {
        start,
        matches,
        capped,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn collect_ascii_case_insensitive_text_search_chunk(
    haystack: &str,
    haystack_bytes: &[u8],
    needle_bytes: &[u8],
    start: usize,
    end: usize,
) -> TextSearchChunkResult {
    let mut matches = Vec::new();
    let mut offset = start;
    let mut capped = false;
    while offset < end {
        if ascii_case_insensitive_matches_at(haystack, haystack_bytes, needle_bytes, offset) {
            let match_end = offset + needle_bytes.len();
            matches.push(TextSearchMatch {
                start: offset,
                end: match_end,
            });
            if matches.len() >= TEXT_SEARCH_MATCH_DISPLAY_LIMIT {
                capped = match_end < haystack.len();
                break;
            }
            offset = match_end;
        } else {
            offset += 1;
        }
    }
    TextSearchChunkResult {
        start,
        matches,
        capped,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn ascii_case_sensitive_matches_at(
    haystack: &str,
    haystack_bytes: &[u8],
    needle_bytes: &[u8],
    index: usize,
) -> bool {
    if index > haystack_bytes.len().saturating_sub(needle_bytes.len()) {
        return false;
    }
    let end = index + needle_bytes.len();
    haystack.is_char_boundary(index)
        && haystack.is_char_boundary(end)
        && &haystack_bytes[index..end] == needle_bytes
}

fn find_ascii_case_insensitive_match(
    haystack: &str,
    haystack_bytes: &[u8],
    needle_bytes: &[u8],
    offset: usize,
) -> Option<usize> {
    if needle_bytes.is_empty() || haystack_bytes.len() < needle_bytes.len() {
        return None;
    }
    let max_start = haystack_bytes.len() - needle_bytes.len();
    if offset > max_start {
        return None;
    }
    let mut index = offset;
    while index <= max_start {
        if ascii_case_insensitive_matches_at(haystack, haystack_bytes, needle_bytes, index) {
            return Some(index);
        }
        index += 1;
    }
    None
}

fn ascii_case_insensitive_matches_at(
    haystack: &str,
    haystack_bytes: &[u8],
    needle_bytes: &[u8],
    index: usize,
) -> bool {
    if index > haystack_bytes.len().saturating_sub(needle_bytes.len()) {
        return false;
    }
    let end = index + needle_bytes.len();
    haystack_bytes[index].eq_ignore_ascii_case(&needle_bytes[0])
        && haystack.is_char_boundary(index)
        && haystack.is_char_boundary(end)
        && haystack_bytes[index..end]
            .iter()
            .zip(needle_bytes.iter())
            .all(|(left, right)| left.eq_ignore_ascii_case(right))
}

pub(crate) fn previous_text_search_index(current: usize, count: usize) -> usize {
    if count == 0 || current == 0 || current >= count {
        count.saturating_sub(1)
    } else {
        current - 1
    }
}

pub(crate) fn next_text_search_index(current: usize, count: usize) -> usize {
    if count == 0 { 0 } else { (current + 1) % count }
}

pub(crate) fn text_search_status_text(
    query: &str,
    match_count: usize,
    current_match_index: Option<usize>,
    capped: bool,
    i18n: I18n,
) -> String {
    if query.is_empty() {
        return i18n.t("editor-type-search");
    }
    if match_count == 0 {
        return i18n.t("editor-no-matches");
    }

    let total = if capped {
        format!("{match_count}+")
    } else {
        match_count.to_string()
    };
    if let Some(index) = current_match_index {
        i18n.t_args(
            "editor-current-match",
            &[
                ("current", (index + 1).to_string()),
                ("total", total.clone()),
            ],
        )
    } else {
        i18n.t_args("editor-matches", &[("count", total)])
    }
}

pub(crate) fn replace_text_span(text: &str, start: usize, end: usize, replacement: &str) -> String {
    if start > end
        || end > text.len()
        || !text.is_char_boundary(start)
        || !text.is_char_boundary(end)
    {
        return text.to_string();
    }

    let mut next = String::with_capacity(text.len() - (end - start) + replacement.len());
    next.push_str(&text[..start]);
    next.push_str(replacement);
    next.push_str(&text[end..]);
    next
}

#[cfg(test)]
pub(crate) fn replace_all_text_matches(
    text: &str,
    matches: &[TextSearchMatch],
    replacement: &str,
) -> String {
    if matches.is_empty() {
        return text.to_string();
    }

    let mut next = String::with_capacity(text.len());
    let mut copied_until = 0;
    for match_ in matches {
        if match_.start < copied_until
            || match_.end > text.len()
            || !text.is_char_boundary(match_.start)
            || !text.is_char_boundary(match_.end)
        {
            return text.to_string();
        }
        next.push_str(&text[copied_until..match_.start]);
        next.push_str(replacement);
        copied_until = match_.end;
    }
    next.push_str(&text[copied_until..]);
    next
}

pub(crate) fn replace_all_text_query(
    text: &str,
    query: &str,
    replacement: &str,
    case_sensitive: bool,
) -> String {
    if query.is_empty() || text.len() < query.len() {
        return text.to_string();
    }

    if case_sensitive {
        replace_all_text_pattern(text, text, query, replacement)
    } else {
        let haystack = text.to_ascii_lowercase();
        let needle = query.to_ascii_lowercase();
        replace_all_text_pattern(text, &haystack, &needle, replacement)
    }
}

fn replace_all_text_pattern(
    original: &str,
    haystack: &str,
    needle: &str,
    replacement: &str,
) -> String {
    let mut next = String::with_capacity(original.len());
    let mut offset = 0;
    let mut copied_until = 0;

    while offset <= haystack.len() {
        let Some(relative_start) = haystack[offset..].find(needle) else {
            break;
        };
        let start = offset + relative_start;
        let end = start + needle.len();
        if !original.is_char_boundary(start) || !original.is_char_boundary(end) {
            return original.to_string();
        }
        next.push_str(&original[copied_until..start]);
        next.push_str(replacement);
        copied_until = end;
        offset = end;
    }

    if copied_until == 0 {
        return original.to_string();
    }
    next.push_str(&original[copied_until..]);
    next
}
