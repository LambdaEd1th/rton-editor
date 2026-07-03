use dioxus::prelude::*;
use rton_editor_core::{
    BinaryEncoding, DecodedDocument, EncodeOptions, TreeRows, ValueSearchResult,
};
#[cfg(target_arch = "wasm32")]
use rton_editor_core::{
    WorkerDocumentSource, WorkerEditorMode, WorkerModeSwitchRequest, WorkerModeSwitchResponse,
    WorkerParseRequest, WorkerParseResponse, WorkerSurface,
};
use std::sync::Arc;

use crate::components::{HexJumpTarget, TextJumpTarget};
#[cfg(not(target_arch = "wasm32"))]
use crate::domain::document_for_owned_tab;
#[cfg(not(target_arch = "wasm32"))]
use crate::domain::tab_surface_for_document;
#[cfg(target_arch = "wasm32")]
use crate::domain::{ByteDocument, TextBuffer, TextContentState, empty_editor_text};
use crate::domain::{
    EditorMode, EditorTabState, HexHistory, Status, TabTaskState, TextHistory, TextSurfaceCache,
    Tone, default_expanded_paths, locate_rton_value_offset, locate_value_path_in_text,
    value_search_result_for_doc, value_tree_rows_for_doc, value_tree_rows_for_doc_with_expansion,
};
use crate::i18n::I18n;
use crate::platform::{run_cpu_task, sleep_ms};
#[cfg(target_arch = "wasm32")]
use crate::platform::{run_mode_switch_worker, run_parse_worker};

use super::tabs::{active_tab, update_tab};

#[derive(Debug)]
struct ModeSwitchPayload {
    doc: Arc<DecodedDocument>,
    surface: crate::domain::editor_tab::TabSurface,
    tree_rows: Arc<TreeRows>,
    search_result: Option<Arc<ValueSearchResult>>,
    search_query: String,
    was_dirty: bool,
}

#[derive(Debug)]
struct ParsePayload {
    doc: Arc<DecodedDocument>,
    tree_rows: Arc<TreeRows>,
    search_result: Option<Arc<ValueSearchResult>>,
    search_query: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParseReport {
    Foreground,
    Background,
}

pub(crate) fn validate_active_tab(
    tabs: Signal<Vec<EditorTabState>>,
    active_tab_id: Signal<usize>,
    mut status: Signal<Status>,
    i18n: I18n,
) {
    let Some(tab) = active_tab(tabs, active_tab_id) else {
        status.set(Status::new(i18n.t("status-no-active-tab"), Tone::Warn));
        return;
    };

    parse_tab(tab, tabs, status, i18n, ParseReport::Foreground);
}

pub(crate) fn parse_tab_by_id(
    tab_id: usize,
    tabs: Signal<Vec<EditorTabState>>,
    status: Signal<Status>,
    i18n: I18n,
) {
    let tab = {
        let tabs_snapshot = tabs.read();
        let Some(tab) = tabs_snapshot.iter().find(|tab| tab.id == tab_id) else {
            return;
        };
        if tab.doc.is_some() || tab.dirty {
            return;
        }
        tab.clone()
    };

    parse_tab(tab, tabs, status, i18n, ParseReport::Background);
}

fn parse_tab(
    tab: EditorTabState,
    tabs: Signal<Vec<EditorTabState>>,
    mut status: Signal<Status>,
    i18n: I18n,
    report: ParseReport,
) {
    let tab_id = tab.id;
    let file_name = tab.file_name.clone();
    let was_dirty = tab.dirty;
    let task_id = tab.task_generation.saturating_add(1);
    let target_mode = tab.mode;
    update_tab(tabs, tab_id, |active| {
        active.task_generation = task_id;
        if report == ParseReport::Foreground {
            active.task_state = Some(TabTaskState {
                id: task_id,
                target_mode,
            });
        }
    });

    spawn(async move {
        let result = parse_tab_payload(tab).await;
        match result {
            Ok(payload) => {
                if !parse_task_is_current(tabs, tab_id, task_id, was_dirty, report) {
                    return;
                }
                apply_parse_payload(tabs, tab_id, payload);
                if report == ParseReport::Foreground {
                    status.set(Status::new(i18n.t("status-parsed"), Tone::Ok));
                }
            }
            Err(error) => {
                if parse_task_is_current(tabs, tab_id, task_id, was_dirty, report) {
                    if report == ParseReport::Foreground {
                        update_tab(tabs, tab_id, |active| active.task_state = None);
                        status.set(Status::new(error, Tone::Error));
                    } else {
                        status.set(Status::new(
                            i18n.t_args(
                                "status-file-error",
                                &[("name", file_name), ("error", error)],
                            ),
                            Tone::Warn,
                        ));
                    }
                }
            }
        }
    });
}

pub(crate) fn switch_active_mode(
    next_mode: EditorMode,
    encode_options: EncodeOptions,
    tabs: Signal<Vec<EditorTabState>>,
    active_tab_id: Signal<usize>,
    mut status: Signal<Status>,
    i18n: I18n,
) {
    let Some(tab) = active_tab(tabs, active_tab_id) else {
        return;
    };
    if tab.mode == next_mode {
        return;
    }

    let tab_id = tab.id;
    if let Some((doc, cache, was_dirty)) = cached_text_surface(&tab, next_mode) {
        let task_id = tab.task_generation.saturating_add(1);
        let search_query = tab.search_query.clone();
        update_tab(tabs, tab_id, |active| {
            active.task_generation = task_id;
            active.task_state = Some(TabTaskState {
                id: task_id,
                target_mode: next_mode,
            });
        });
        status.set(Status::new(
            i18n.t_args(
                "status-switching",
                &[("mode", next_mode.label().to_string())],
            ),
            Tone::Warn,
        ));
        spawn(async move {
            let result = cached_mode_switch_payload(doc, cache, was_dirty, search_query).await;
            if !task_is_current(tabs, tab_id, task_id) {
                return;
            }
            apply_mode_switch_payload(tabs, tab_id, next_mode, result);
            status.set(Status::new(
                i18n.t_args(
                    "status-switched-cached",
                    &[("mode", next_mode.label().to_string())],
                ),
                Tone::Info,
            ));
        });
        return;
    }

    let task_id = tab.task_generation.saturating_add(1);
    update_tab(tabs, tab_id, |active| {
        active.task_generation = task_id;
        active.task_state = Some(TabTaskState {
            id: task_id,
            target_mode: next_mode,
        });
    });
    status.set(Status::new(
        i18n.t_args(
            "status-switching",
            &[("mode", next_mode.label().to_string())],
        ),
        Tone::Warn,
    ));

    spawn(async move {
        let result = convert_tab_mode(tab, next_mode, encode_options).await;
        match result {
            Ok(payload) => {
                if !task_is_current(tabs, tab_id, task_id) {
                    return;
                }
                apply_mode_switch_payload(tabs, tab_id, next_mode, payload);
                status.set(Status::new(
                    i18n.t_args(
                        "status-switched",
                        &[("mode", next_mode.label().to_string())],
                    ),
                    Tone::Info,
                ));
            }
            Err(error) => {
                if task_is_current(tabs, tab_id, task_id) {
                    update_tab(tabs, tab_id, |active| active.task_state = None);
                    status.set(Status::new(
                        i18n.t_args("status-cannot-switch", &[("error", error)]),
                        Tone::Error,
                    ));
                }
            }
        }
    });
}

pub(crate) fn update_compact_output(
    compact: bool,
    mut compact_output: Signal<bool>,
    tabs: Signal<Vec<EditorTabState>>,
    active_tab_id: Signal<usize>,
    status: Signal<Status>,
    i18n: I18n,
) {
    if *compact_output.peek() == compact {
        return;
    }
    compact_output.set(compact);
    refresh_active_rton_encoding(
        EncodeOptions {
            encoding: if compact {
                BinaryEncoding::Compact
            } else {
                BinaryEncoding::Standard
            },
            encrypted: false,
        },
        tabs,
        active_tab_id,
        status,
        i18n,
    );
}

pub(crate) fn refresh_active_rton_encoding(
    encode_options: EncodeOptions,
    tabs: Signal<Vec<EditorTabState>>,
    active_tab_id: Signal<usize>,
    mut status: Signal<Status>,
    i18n: I18n,
) {
    let Some(tab) = active_tab(tabs, active_tab_id) else {
        return;
    };
    if tab.mode != EditorMode::RtonHex {
        return;
    }

    let tab_id = tab.id;
    let task_id = tab.task_generation.saturating_add(1);
    let selected_path = tab.selected_path.clone();
    update_tab(tabs, tab_id, |active| {
        active.task_generation = task_id;
        active.task_state = Some(TabTaskState {
            id: task_id,
            target_mode: EditorMode::RtonHex,
        });
    });
    status.set(Status::new(
        i18n.t_args(
            "status-switching",
            &[("mode", EditorMode::RtonHex.label().to_string())],
        ),
        Tone::Warn,
    ));

    spawn(async move {
        let result = convert_tab_mode(tab, EditorMode::RtonHex, encode_options).await;
        match result {
            Ok(payload) => {
                if !task_is_current(tabs, tab_id, task_id) {
                    return;
                }
                apply_mode_switch_payload_with_selection(
                    tabs,
                    tab_id,
                    EditorMode::RtonHex,
                    payload,
                    selected_path,
                );
                status.set(Status::new(
                    i18n.t_args(
                        "status-switched",
                        &[("mode", encode_options.encoding.label().to_string())],
                    ),
                    Tone::Info,
                ));
            }
            Err(error) => {
                if task_is_current(tabs, tab_id, task_id) {
                    update_tab(tabs, tab_id, |active| active.task_state = None);
                    status.set(Status::new(
                        i18n.t_args("status-cannot-switch", &[("error", error)]),
                        Tone::Error,
                    ));
                }
            }
        }
    });
}

pub(crate) fn update_active_search(
    query: String,
    tabs: Signal<Vec<EditorTabState>>,
    active_tab_id: Signal<usize>,
) {
    let active_id = *active_tab_id.read();
    let (doc, generation) = {
        let tabs_snapshot = tabs.read();
        let Some(tab) = tabs_snapshot.iter().find(|tab| tab.id == active_id) else {
            return;
        };
        (tab.doc.clone(), tab.search_generation.saturating_add(1))
    };
    update_tab(tabs, active_id, |tab| {
        tab.search_query = query;
        tab.search_result = None;
        tab.search_generation = generation;
    });

    let query = {
        let tabs_snapshot = tabs.read();
        tabs_snapshot
            .iter()
            .find(|tab| tab.id == active_id)
            .map(|tab| tab.search_query.clone())
            .unwrap_or_default()
    };
    let Some(doc) = doc else {
        return;
    };
    if query.trim().is_empty() {
        return;
    }

    spawn(async move {
        sleep_ms(160).await;
        if !search_task_is_current(tabs, active_id, generation, &query) {
            return;
        }
        let query_for_task = query.clone();
        let result = run_cpu_task(move || {
            value_search_result_for_doc(&doc, &query_for_task).map(Arc::unwrap_or_clone)
        })
        .await;
        update_tab(tabs, active_id, |tab| {
            if tab.search_generation == generation && tab.search_query == query {
                tab.search_result = result.map(Arc::new);
            }
        });
    });
}

pub(crate) fn update_selected_path(
    path: String,
    tabs: Signal<Vec<EditorTabState>>,
    active_tab_id: Signal<usize>,
) {
    let active_id = *active_tab_id.read();
    update_tab(tabs, active_id, |tab| {
        tab.selected_path = path;
    });
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn navigate_to_value_path(
    path: String,
    tabs: Signal<Vec<EditorTabState>>,
    active_tab_id: Signal<usize>,
    mut next_jump_id: Signal<u64>,
    mut text_jump_target: Signal<Option<TextJumpTarget>>,
    mut hex_jump_target: Signal<Option<HexJumpTarget>>,
    mut status: Signal<Status>,
    i18n: I18n,
) {
    update_selected_path(path.clone(), tabs, active_tab_id);

    let Some(tab) = active_tab(tabs, active_tab_id) else {
        status.set(Status::new(i18n.t("status-no-active-tab"), Tone::Warn));
        return;
    };
    let Some(doc) = tab.doc.as_ref() else {
        status.set(Status::new(i18n.t("status-no-parsed-document"), Tone::Warn));
        return;
    };

    match tab.mode {
        EditorMode::RtonHex => {
            let Some(byte_doc) = tab.byte_doc.as_ref() else {
                status.set(Status::new(i18n.t("status-no-jump-bytes"), Tone::Warn));
                return;
            };

            let Some(offset) = locate_rton_value_offset(byte_doc, &path) else {
                status.set(Status::new(i18n.t("status-offset-not-found"), Tone::Warn));
                return;
            };
            let id = *next_jump_id.read();
            next_jump_id.set(id.saturating_add(1));
            hex_jump_target.set(Some(HexJumpTarget { id, offset }));
            status.set(Status::new(
                i18n.t_args("status-jumped-offset", &[("offset", format!("{offset:X}"))]),
                Tone::Ok,
            ));
        }
        EditorMode::Json | EditorMode::Yaml | EditorMode::Toml => {
            let text_buffer = tab.text_buffer.as_ref();
            let text = text_buffer
                .map(|buffer| buffer.text.clone())
                .unwrap_or_else(|| tab.editor_text.clone());
            let line_count = text_buffer
                .map(|buffer| buffer.line_count())
                .unwrap_or_else(|| text_line_count(text.as_ref()));
            let Some(format) = tab.mode.text_format() else {
                return;
            };
            let Some(position) =
                locate_value_path_in_text(&doc.value, &path, text.as_ref(), format)
            else {
                status.set(Status::new(
                    i18n.t("status-text-line-not-found"),
                    Tone::Warn,
                ));
                return;
            };

            let id = *next_jump_id.read();
            next_jump_id.set(id.saturating_add(1));
            text_jump_target.set(Some(TextJumpTarget {
                id,
                line: position.line,
                column: position.column,
                selection_end_column: position.column,
                line_count,
                focus: true,
            }));
            status.set(Status::new(
                i18n.t_args("status-jumped-line", &[("line", position.line.to_string())]),
                Tone::Ok,
            ));
        }
    }
}

fn text_line_count(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }

    text.bytes()
        .enumerate()
        .filter(|(index, byte)| *byte == b'\n' && index + 1 < text.len())
        .count()
        + 1
}

pub(crate) fn cache_parsed_document(
    tabs: Signal<Vec<EditorTabState>>,
    tab_id: usize,
    selected_path: String,
    doc: Arc<DecodedDocument>,
    clear_dirty: bool,
) {
    update_tab(tabs, tab_id, |active| {
        active.tree_rows = value_tree_rows_for_doc_with_expansion(&doc, &active.expanded_paths);
        active.search_result = value_search_result_for_doc(&doc, &active.search_query);
        active.doc = Some(doc);
        active.selected_path = selected_path;
        active.task_state = None;
        if clear_dirty {
            active.dirty = false;
        }
    });
}

pub(crate) fn toggle_active_tree_path(
    path: String,
    tabs: Signal<Vec<EditorTabState>>,
    active_tab_id: Signal<usize>,
) {
    let active_id = *active_tab_id.read();
    let payload = {
        let tabs_snapshot = tabs.read();
        let Some(tab) = tabs_snapshot.iter().find(|tab| tab.id == active_id) else {
            return;
        };
        let mut expanded = (*tab.expanded_paths).clone();
        if !expanded.insert(path.clone()) {
            expanded.remove(&path);
        }
        let expanded_paths = Arc::new(expanded);
        let generation = tab.tree_generation.saturating_add(1);
        (tab.doc.clone(), expanded_paths, generation)
    };

    let (doc, expanded_paths, generation) = payload;
    update_tab(tabs, active_id, |tab| {
        tab.expanded_paths = expanded_paths.clone();
        tab.tree_generation = generation;
    });

    let Some(doc) = doc else {
        return;
    };
    spawn(async move {
        let expanded_paths_for_task = expanded_paths.clone();
        let tree_rows = run_cpu_task(move || {
            value_tree_rows_for_doc_with_expansion(&doc, &expanded_paths_for_task)
        })
        .await;
        update_tab(tabs, active_id, |tab| {
            if tab.tree_generation == generation {
                tab.tree_rows = tree_rows;
            }
        });
    });
}

fn cached_text_surface(
    tab: &EditorTabState,
    next_mode: EditorMode,
) -> Option<(Arc<DecodedDocument>, TextSurfaceCache, bool)> {
    let doc = tab.doc.clone()?;
    let cache = tab
        .text_cache
        .iter()
        .find(|cache| cache.mode == next_mode)
        .cloned()?;
    Some((doc, cache, tab.dirty))
}

async fn parse_tab_payload(tab: EditorTabState) -> Result<ParsePayload, String> {
    let search_query = tab.search_query.clone();

    if !tab.dirty
        && let Some(doc) = tab.doc.clone()
    {
        return Ok(parse_payload_for_doc(doc, search_query).await);
    }

    #[cfg(target_arch = "wasm32")]
    {
        let request = worker_parse_request(&tab)?;
        let response = run_parse_worker(request).await?;
        return Ok(worker_parse_payload(response));
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let doc =
            run_cpu_task(move || document_for_owned_tab(tab).map_err(|error| error.to_string()))
                .await?;
        Ok(parse_payload_for_doc(doc, search_query).await)
    }
}

async fn parse_payload_for_doc(doc: Arc<DecodedDocument>, search_query: String) -> ParsePayload {
    let tree_rows = {
        let doc = doc.clone();
        run_cpu_task(move || value_tree_rows_for_doc(&doc)).await
    };
    let search_result = if search_query.trim().is_empty() {
        None
    } else {
        let doc = doc.clone();
        let query = search_query.clone();
        run_cpu_task(move || value_search_result_for_doc(&doc, &query)).await
    };
    ParsePayload {
        doc,
        tree_rows,
        search_result,
        search_query,
    }
}

async fn cached_mode_switch_payload(
    doc: Arc<DecodedDocument>,
    cache: TextSurfaceCache,
    was_dirty: bool,
    search_query: String,
) -> ModeSwitchPayload {
    let tree_rows = {
        let doc = doc.clone();
        run_cpu_task(move || value_tree_rows_for_doc(&doc)).await
    };
    let search_result = if search_query.trim().is_empty() {
        None
    } else {
        let doc = doc.clone();
        let query = search_query.clone();
        run_cpu_task(move || value_search_result_for_doc(&doc, &query)).await
    };
    ModeSwitchPayload {
        doc,
        surface: crate::domain::editor_tab::TabSurface {
            byte_doc: None,
            editor_text: cache.editor_text,
            text_buffer: cache.text_buffer,
            text_state: cache.text_state,
        },
        tree_rows,
        search_result,
        search_query,
        was_dirty,
    }
}

async fn convert_tab_mode(
    tab: EditorTabState,
    next_mode: EditorMode,
    encode_options: EncodeOptions,
) -> Result<ModeSwitchPayload, String> {
    #[cfg(target_arch = "wasm32")]
    {
        let was_dirty = tab.dirty;
        let request = worker_mode_switch_request(&tab, next_mode, encode_options)?;
        let response = run_mode_switch_worker(request).await?;
        return worker_mode_switch_payload(response, was_dirty);
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let was_dirty = tab.dirty;
        let search_query = tab.search_query.clone();
        let doc =
            run_cpu_task(move || document_for_owned_tab(tab).map_err(|error| error.to_string()))
                .await?;
        let surface = {
            let doc = doc.clone();
            run_cpu_task(move || {
                tab_surface_for_document(&doc, next_mode, encode_options)
                    .map_err(|error| error.to_string())
            })
            .await?
        };
        let tree_rows = {
            let doc = doc.clone();
            run_cpu_task(move || value_tree_rows_for_doc(&doc)).await
        };
        let search_result = if search_query.trim().is_empty() {
            None
        } else {
            let doc = doc.clone();
            let query = search_query.clone();
            run_cpu_task(move || value_search_result_for_doc(&doc, &query)).await
        };
        Ok(ModeSwitchPayload {
            doc,
            surface,
            tree_rows,
            search_result,
            search_query,
            was_dirty,
        })
    }
}

#[cfg(target_arch = "wasm32")]
fn worker_mode_switch_request(
    tab: &EditorTabState,
    next_mode: EditorMode,
    encode_options: EncodeOptions,
) -> Result<WorkerModeSwitchRequest, String> {
    Ok(WorkerModeSwitchRequest {
        source: worker_document_source(tab)?,
        target_mode: worker_editor_mode(next_mode),
        search_query: tab.search_query.clone(),
        encode_options,
    })
}

#[cfg(target_arch = "wasm32")]
fn worker_parse_request(tab: &EditorTabState) -> Result<WorkerParseRequest, String> {
    Ok(WorkerParseRequest {
        source: worker_document_source(tab)?,
        search_query: tab.search_query.clone(),
    })
}

#[cfg(target_arch = "wasm32")]
fn worker_document_source(tab: &EditorTabState) -> Result<WorkerDocumentSource, String> {
    match tab.mode {
        EditorMode::RtonHex => {
            let bytes = tab
                .byte_doc
                .as_ref()
                .map(|byte_doc| byte_doc.as_cow().into_owned())
                .ok_or_else(|| "Missing RTON bytes".to_string())?;
            Ok(WorkerDocumentSource::RtonBytes(bytes))
        }
        EditorMode::Json | EditorMode::Yaml | EditorMode::Toml => {
            let format = tab
                .mode
                .text_format()
                .ok_or_else(|| "Missing text format".to_string())?;
            let text = tab
                .text_buffer
                .as_ref()
                .map(|buffer| buffer.text.to_string())
                .unwrap_or_else(|| tab.editor_text.to_string());
            Ok(WorkerDocumentSource::Text { text, format })
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn worker_editor_mode(mode: EditorMode) -> WorkerEditorMode {
    match mode {
        EditorMode::RtonHex => WorkerEditorMode::RtonHex,
        EditorMode::Json => WorkerEditorMode::Json,
        EditorMode::Yaml => WorkerEditorMode::Yaml,
        EditorMode::Toml => WorkerEditorMode::Toml,
    }
}

#[cfg(target_arch = "wasm32")]
fn worker_mode_switch_payload(
    response: WorkerModeSwitchResponse,
    was_dirty: bool,
) -> Result<ModeSwitchPayload, String> {
    Ok(ModeSwitchPayload {
        doc: Arc::new(response.doc),
        surface: worker_surface_to_tab_surface(response.surface),
        tree_rows: Arc::new(response.tree_rows),
        search_result: response.search_result.map(Arc::new),
        search_query: response.search_query,
        was_dirty,
    })
}

#[cfg(target_arch = "wasm32")]
fn worker_parse_payload(response: WorkerParseResponse) -> ParsePayload {
    ParsePayload {
        doc: Arc::new(response.doc),
        tree_rows: Arc::new(response.tree_rows),
        search_result: response.search_result.map(Arc::new),
        search_query: response.search_query,
    }
}

#[cfg(target_arch = "wasm32")]
fn worker_surface_to_tab_surface(surface: WorkerSurface) -> crate::domain::editor_tab::TabSurface {
    match surface {
        WorkerSurface::RtonBytes(bytes) => crate::domain::editor_tab::TabSurface {
            byte_doc: Some(ByteDocument::from_vec(bytes)),
            editor_text: empty_editor_text(),
            text_buffer: None,
            text_state: TextContentState::None,
        },
        WorkerSurface::Text {
            text,
            line_offsets,
            byte_count,
            line_count,
            format,
        } => {
            let text_buffer = Arc::new(TextBuffer::from_parts(text, line_offsets));
            crate::domain::editor_tab::TabSurface {
                byte_doc: None,
                editor_text: text_buffer.text.clone(),
                text_buffer: Some(text_buffer),
                text_state: TextContentState::Text {
                    byte_count,
                    line_count,
                    format,
                },
            }
        }
    }
}

fn task_is_current(tabs: Signal<Vec<EditorTabState>>, tab_id: usize, task_id: u64) -> bool {
    tabs.read().iter().any(|tab| {
        tab.id == tab_id
            && tab
                .task_state
                .as_ref()
                .is_some_and(|state| state.id == task_id)
    })
}

fn parse_task_is_current(
    tabs: Signal<Vec<EditorTabState>>,
    tab_id: usize,
    task_id: u64,
    was_dirty: bool,
    report: ParseReport,
) -> bool {
    match report {
        ParseReport::Foreground => task_is_current(tabs, tab_id, task_id),
        ParseReport::Background => tabs.read().iter().any(|tab| {
            tab.id == tab_id && tab.task_generation == task_id && tab.dirty == was_dirty
        }),
    }
}

fn search_task_is_current(
    tabs: Signal<Vec<EditorTabState>>,
    tab_id: usize,
    generation: u64,
    query: &str,
) -> bool {
    tabs.read().iter().any(|tab| {
        tab.id == tab_id && tab.search_generation == generation && tab.search_query == query
    })
}

fn apply_parse_payload(tabs: Signal<Vec<EditorTabState>>, tab_id: usize, payload: ParsePayload) {
    update_tab(tabs, tab_id, |active| {
        active.expanded_paths = default_expanded_paths();
        active.tree_rows = payload.tree_rows;
        active.search_result = if active.search_query == payload.search_query {
            payload.search_result
        } else {
            None
        };
        active.doc = Some(payload.doc);
        active.selected_path = "$".to_string();
        active.task_state = None;
    });
}

fn apply_mode_switch_payload(
    tabs: Signal<Vec<EditorTabState>>,
    tab_id: usize,
    next_mode: EditorMode,
    payload: ModeSwitchPayload,
) {
    apply_mode_switch_payload_with_selection(tabs, tab_id, next_mode, payload, "$".to_string());
}

fn apply_mode_switch_payload_with_selection(
    tabs: Signal<Vec<EditorTabState>>,
    tab_id: usize,
    next_mode: EditorMode,
    payload: ModeSwitchPayload,
    selected_path: String,
) {
    update_tab(tabs, tab_id, |active| {
        active.expanded_paths = default_expanded_paths();
        active.tree_rows = payload.tree_rows;
        active.search_result = if active.search_query == payload.search_query {
            payload.search_result
        } else {
            None
        };
        active.doc = Some(payload.doc);
        active.byte_doc = payload.surface.byte_doc;
        active.editor_text = payload.surface.editor_text;
        active.text_buffer = payload.surface.text_buffer;
        active.text_state = payload.surface.text_state;
        active.mode = next_mode;
        active.selected_path = selected_path;
        active.text_history = TextHistory::default();
        active.hex_history = HexHistory::default();
        active.task_state = None;
        active.dirty = payload.was_dirty;
        if next_mode.text_format().is_some() {
            active.text_cache.retain(|cache| cache.mode != next_mode);
            active.text_cache.push(TextSurfaceCache {
                mode: next_mode,
                editor_text: active.editor_text.clone(),
                text_buffer: active.text_buffer.clone(),
                text_state: active.text_state.clone(),
            });
        }
    });
}
