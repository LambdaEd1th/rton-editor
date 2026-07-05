use rton_editor_core::{
    BinaryEncoding, CoreError, DecodedDocument, ENCRYPTED_RTON_PREFIX, EncodeOptions, SourceFormat,
    TextFormat, TreeRows, ValueSearchResult, decode_hex_rton, decode_rton_reader,
    decrypt_rton_bytes_if_needed, detect_rton_binary_encoding, flatten_expanded_value_tree,
    parse_text, search_value_tree,
};
#[cfg(any(not(target_arch = "wasm32"), test))]
use rton_editor_core::{encode_rton_bytes, value_to_text};
use std::collections::HashSet;
use std::sync::Arc;

use super::{ByteDocument, EditorMode, HexHistory, TextHistory};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EditorTabState {
    pub(crate) id: usize,
    pub(crate) file_name: String,
    pub(crate) doc: Option<Arc<DecodedDocument>>,
    pub(crate) byte_doc: Option<ByteDocument>,
    pub(crate) source_encode_options: EncodeOptions,
    pub(crate) tree_rows: Arc<TreeRows>,
    pub(crate) search_result: Option<Arc<ValueSearchResult>>,
    pub(crate) editor_text: Arc<str>,
    pub(crate) text_buffer: Option<Arc<TextBuffer>>,
    pub(crate) mode: EditorMode,
    pub(crate) search_query: String,
    pub(crate) selected_path: String,
    pub(crate) text_history: TextHistory,
    pub(crate) hex_history: HexHistory,
    pub(crate) text_state: TextContentState,
    pub(crate) task_state: Option<TabTaskState>,
    pub(crate) task_generation: u64,
    pub(crate) tree_generation: u64,
    pub(crate) search_generation: u64,
    pub(crate) expanded_paths: Arc<HashSet<String>>,
    pub(crate) text_cache: Vec<TextSurfaceCache>,
    pub(crate) dirty: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct TabSurface {
    pub(crate) byte_doc: Option<ByteDocument>,
    pub(crate) editor_text: Arc<str>,
    pub(crate) text_buffer: Option<Arc<TextBuffer>>,
    pub(crate) text_state: TextContentState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TextBuffer {
    pub(crate) text: Arc<str>,
    pub(crate) line_offsets: Arc<[usize]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TextRangeReplacement {
    pub(crate) start_line: usize,
    pub(crate) start_column_utf16: usize,
    pub(crate) end_line: usize,
    pub(crate) end_column_utf16: usize,
    pub(crate) replacement: String,
}

impl TextBuffer {
    pub(crate) fn new(text: String) -> Self {
        let line_offsets = Arc::<[usize]>::from(text_line_offsets(&text));
        Self {
            text: Arc::from(text),
            line_offsets,
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn from_parts(text: String, line_offsets: Vec<usize>) -> Self {
        Self::from_arc_parts(Arc::from(text), line_offsets)
    }

    pub(crate) fn from_arc_parts(text: Arc<str>, line_offsets: Vec<usize>) -> Self {
        Self {
            text,
            line_offsets: Arc::from(line_offsets),
        }
    }

    pub(crate) fn line_count(&self) -> usize {
        self.line_offsets.len()
    }

    pub(crate) fn byte_count(&self) -> usize {
        self.text.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TextContentState {
    None,
    Text {
        byte_count: usize,
        line_count: usize,
        format: TextFormat,
    },
}

impl TextContentState {
    pub(crate) fn text_format(&self) -> Option<TextFormat> {
        match self {
            Self::Text { format, .. } => Some(*format),
            Self::None => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TabTaskState {
    pub(crate) id: u64,
    pub(crate) target_mode: EditorMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TextSurfaceCache {
    pub(crate) mode: EditorMode,
    pub(crate) editor_text: Arc<str>,
    pub(crate) text_buffer: Option<Arc<TextBuffer>>,
    pub(crate) text_state: TextContentState,
}

#[cfg(test)]
pub(crate) fn create_tab_from_bytes(
    id: usize,
    name: String,
    bytes: &[u8],
) -> Result<EditorTabState, CoreError> {
    create_tab_from_byte_document(id, name, ByteDocument::from_vec(bytes.to_vec()))
}

pub(crate) fn create_tab_from_byte_document(
    id: usize,
    name: String,
    byte_doc: ByteDocument,
) -> Result<EditorTabState, CoreError> {
    match SourceFormat::from_file_name(&name) {
        SourceFormat::Rton | SourceFormat::Unknown => {
            let display_source = rton_hex_byte_document_for_display(byte_doc);
            Ok(EditorTabState {
                id,
                file_name: name,
                doc: None,
                byte_doc: Some(display_source.byte_doc),
                source_encode_options: display_source.encode_options,
                tree_rows: empty_tree_rows(),
                search_result: None,
                editor_text: empty_editor_text(),
                text_buffer: None,
                mode: EditorMode::RtonHex,
                search_query: String::new(),
                selected_path: "$".to_string(),
                text_history: TextHistory::default(),
                hex_history: HexHistory::default(),
                text_state: TextContentState::None,
                task_state: None,
                task_generation: 0,
                tree_generation: 0,
                search_generation: 0,
                expanded_paths: default_expanded_paths(),
                text_cache: Vec::new(),
                dirty: false,
            })
        }
        SourceFormat::Json => create_text_tab(
            id,
            name,
            String::from_utf8_lossy(byte_doc.as_cow().as_ref()).to_string(),
            TextFormat::Json,
        ),
        SourceFormat::Yaml => create_text_tab(
            id,
            name,
            String::from_utf8_lossy(byte_doc.as_cow().as_ref()).to_string(),
            TextFormat::Yaml,
        ),
        SourceFormat::Toml => create_text_tab(
            id,
            name,
            String::from_utf8_lossy(byte_doc.as_cow().as_ref()).to_string(),
            TextFormat::Toml,
        ),
    }
}

struct RtonDisplaySource {
    byte_doc: ByteDocument,
    encode_options: EncodeOptions,
}

fn rton_hex_byte_document_for_display(byte_doc: ByteDocument) -> RtonDisplaySource {
    let source_encoding = rton_binary_encoding_from_byte_document(&byte_doc);
    let encrypted = ENCRYPTED_RTON_PREFIX
        .iter()
        .enumerate()
        .all(|(index, byte)| byte_doc.byte_at(index) == Some(*byte));
    if !encrypted {
        return RtonDisplaySource {
            byte_doc,
            encode_options: EncodeOptions {
                encoding: source_encoding,
                encrypted: false,
            },
        };
    }

    let decrypted = {
        let bytes = byte_doc.as_cow();
        decrypt_rton_bytes_if_needed(bytes.as_ref())
    };

    match decrypted {
        Ok(Some(bytes)) => {
            let encoding = detect_rton_binary_encoding(&bytes);
            RtonDisplaySource {
                byte_doc: ByteDocument::from_vec(bytes),
                encode_options: EncodeOptions {
                    encoding,
                    encrypted: true,
                },
            }
        }
        Ok(None) | Err(_) => RtonDisplaySource {
            byte_doc,
            encode_options: EncodeOptions {
                encoding: BinaryEncoding::Standard,
                encrypted: true,
            },
        },
    }
}

fn rton_binary_encoding_from_byte_document(byte_doc: &ByteDocument) -> BinaryEncoding {
    let mut header = [0_u8; 8];
    if byte_doc.copy_range_to(0, &mut header) == Some(header.len()) {
        detect_rton_binary_encoding(&header)
    } else {
        BinaryEncoding::Standard
    }
}

pub(crate) fn create_text_tab(
    id: usize,
    file_name: String,
    text: String,
    format: TextFormat,
) -> Result<EditorTabState, CoreError> {
    let surface = text_surface_from_text(text, format);
    Ok(create_text_tab_from_surface(id, file_name, surface, format))
}

pub(crate) fn create_text_tab_from_surface(
    id: usize,
    file_name: String,
    surface: TabSurface,
    format: TextFormat,
) -> EditorTabState {
    let mode = match format {
        TextFormat::Json => EditorMode::Json,
        TextFormat::Yaml => EditorMode::Yaml,
        TextFormat::Toml => EditorMode::Toml,
    };
    EditorTabState {
        id,
        file_name,
        tree_rows: empty_tree_rows(),
        doc: None,
        byte_doc: surface.byte_doc,
        source_encode_options: EncodeOptions::default(),
        search_result: None,
        editor_text: surface.editor_text,
        text_buffer: surface.text_buffer,
        mode,
        search_query: String::new(),
        selected_path: "$".to_string(),
        text_history: TextHistory::default(),
        hex_history: HexHistory::default(),
        text_state: surface.text_state,
        task_state: None,
        task_generation: 0,
        tree_generation: 0,
        search_generation: 0,
        expanded_paths: default_expanded_paths(),
        text_cache: Vec::new(),
        dirty: false,
    }
}

pub(crate) fn empty_tree_rows() -> Arc<TreeRows> {
    Arc::new(TreeRows {
        rows: Vec::new(),
        truncated: false,
    })
}

pub(crate) fn empty_editor_text() -> Arc<str> {
    Arc::<str>::from("")
}

pub(crate) fn value_tree_rows_for_doc(doc: &DecodedDocument) -> Arc<TreeRows> {
    Arc::new(flatten_expanded_value_tree(
        &doc.value,
        default_expanded_paths().as_ref(),
        usize::MAX,
    ))
}

pub(crate) fn value_tree_rows_for_doc_with_expansion(
    doc: &DecodedDocument,
    expanded_paths: &HashSet<String>,
) -> Arc<TreeRows> {
    Arc::new(flatten_expanded_value_tree(
        &doc.value,
        expanded_paths,
        usize::MAX,
    ))
}

pub(crate) fn default_expanded_paths() -> Arc<HashSet<String>> {
    Arc::new(HashSet::from(["$".to_string()]))
}

pub(crate) fn value_search_result_for_doc(
    doc: &DecodedDocument,
    query: &str,
) -> Option<Arc<ValueSearchResult>> {
    if query.trim().is_empty() {
        None
    } else {
        Some(Arc::new(search_value_tree(&doc.value, query, usize::MAX)))
    }
}

pub(crate) fn parse_editor_text(
    text: &str,
    mode: EditorMode,
) -> Result<DecodedDocument, CoreError> {
    match mode {
        EditorMode::RtonHex => decode_hex_rton(text),
        EditorMode::Json => parse_text(text, TextFormat::Json),
        EditorMode::Yaml => parse_text(text, TextFormat::Yaml),
        EditorMode::Toml => parse_text(text, TextFormat::Toml),
    }
}

pub(crate) fn document_for_tab(tab: &EditorTabState) -> Result<Arc<DecodedDocument>, CoreError> {
    match tab.mode {
        EditorMode::RtonHex => tab
            .byte_doc
            .as_ref()
            .map(|bytes| decode_rton_reader(bytes.reader()).map(Arc::new))
            .unwrap_or_else(|| decode_hex_rton(tab.editor_text.as_ref()).map(Arc::new)),
        EditorMode::Json | EditorMode::Yaml | EditorMode::Toml => {
            let text = tab
                .text_buffer
                .as_ref()
                .map(|buffer| buffer.text.as_ref())
                .unwrap_or_else(|| tab.editor_text.as_ref());
            parse_editor_text(text, tab.mode).map(Arc::new)
        }
    }
}

pub(crate) fn document_for_owned_tab(
    tab: EditorTabState,
) -> Result<Arc<DecodedDocument>, CoreError> {
    if !tab.dirty
        && let Some(doc) = tab.doc
    {
        return Ok(doc);
    }

    document_for_tab(&tab)
}

#[cfg(any(not(target_arch = "wasm32"), test))]
pub(crate) fn tab_surface_for_document(
    doc: &DecodedDocument,
    mode: EditorMode,
    encode_options: EncodeOptions,
) -> Result<TabSurface, CoreError> {
    match mode {
        EditorMode::RtonHex => {
            let bytes = encode_rton_bytes(&doc.value, encode_options)?;
            Ok(TabSurface {
                byte_doc: Some(ByteDocument::from_vec(bytes)),
                editor_text: empty_editor_text(),
                text_buffer: None,
                text_state: TextContentState::None,
            })
        }
        EditorMode::Json | EditorMode::Yaml | EditorMode::Toml => {
            text_surface_for_document(doc, mode)
        }
    }
}

#[cfg(any(not(target_arch = "wasm32"), test))]
pub(crate) fn text_surface_for_document(
    doc: &DecodedDocument,
    mode: EditorMode,
) -> Result<TabSurface, CoreError> {
    let format = mode.text_format().expect("text mode has format");
    let editor_text = value_to_text(&doc.value, format)?;
    Ok(text_surface_from_text(editor_text, format))
}

pub(crate) fn text_surface_from_text(editor_text: String, format: TextFormat) -> TabSurface {
    let text_buffer = Arc::new(TextBuffer::new(editor_text));
    text_surface_from_buffer(text_buffer, format)
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn text_surface_from_arc_parts(
    editor_text: Arc<str>,
    line_offsets: Vec<usize>,
    format: TextFormat,
) -> TabSurface {
    let text_buffer = Arc::new(TextBuffer::from_arc_parts(editor_text, line_offsets));
    text_surface_from_buffer(text_buffer, format)
}

fn text_surface_from_buffer(text_buffer: Arc<TextBuffer>, format: TextFormat) -> TabSurface {
    let editor_text = text_buffer.text.clone();
    let byte_count = text_buffer.byte_count();
    let line_count = text_buffer.line_count();
    TabSurface {
        byte_doc: None,
        editor_text,
        text_buffer: Some(text_buffer),
        text_state: TextContentState::Text {
            byte_count,
            line_count,
            format,
        },
    }
}

pub(crate) fn text_line_offsets(text: &str) -> Vec<usize> {
    if text.is_empty() {
        return Vec::new();
    }

    let mut offsets = vec![0];
    for (index, byte) in text.bytes().enumerate() {
        if byte == b'\n' && index + 1 < text.len() {
            offsets.push(index + 1);
        }
    }
    offsets
}
