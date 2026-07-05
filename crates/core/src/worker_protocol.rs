use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::{
    DecodedDocument, EncodeOptions, Result, TextFormat, TreeRows, ValueSearchResult,
    decode_rton_bytes, encode_rton_bytes, flatten_expanded_value_tree, parse_text,
    search_value_tree, value_to_text,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkerEditorMode {
    RtonHex,
    Json,
    Yaml,
    Toml,
}

impl WorkerEditorMode {
    pub fn text_format(self) -> Option<TextFormat> {
        match self {
            Self::RtonHex => None,
            Self::Json => Some(TextFormat::Json),
            Self::Yaml => Some(TextFormat::Yaml),
            Self::Toml => Some(TextFormat::Toml),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WorkerDocumentSource {
    RtonBytes(#[serde(with = "serde_bytes")] Vec<u8>),
    Text { text: String, format: TextFormat },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WorkerSurface {
    RtonBytes(#[serde(with = "serde_bytes")] Vec<u8>),
    Text {
        text: String,
        line_offsets: Vec<usize>,
        byte_count: usize,
        line_count: usize,
        format: TextFormat,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkerModeSwitchRequest {
    pub source: WorkerDocumentSource,
    pub target_mode: WorkerEditorMode,
    pub search_query: String,
    pub encode_options: EncodeOptions,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkerModeSwitchResponse {
    pub doc: DecodedDocument,
    pub surface: WorkerSurface,
    pub tree_rows: TreeRows,
    pub search_result: Option<ValueSearchResult>,
    pub search_query: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkerParseRequest {
    pub source: WorkerDocumentSource,
    pub search_query: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkerParseResponse {
    pub doc: DecodedDocument,
    pub tree_rows: TreeRows,
    pub search_result: Option<ValueSearchResult>,
    pub search_query: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkerRtonSizeRequest {
    pub source: WorkerDocumentSource,
    pub encode_options: EncodeOptions,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkerRtonSizeResponse {
    pub byte_len: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkerTextSurfaceRequest {
    #[serde(with = "serde_bytes")]
    pub bytes: Vec<u8>,
    pub format: TextFormat,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkerTextSurfaceResponse {
    pub surface: WorkerSurface,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkerOpenTextRequest {
    #[serde(with = "serde_bytes")]
    pub bytes: Vec<u8>,
    pub format: TextFormat,
    pub search_query: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkerOpenTextResponse {
    pub doc: Option<DecodedDocument>,
    pub surface: WorkerSurface,
    pub tree_rows: TreeRows,
    pub search_result: Option<ValueSearchResult>,
    pub search_query: String,
}

pub fn perform_worker_mode_switch(
    request: WorkerModeSwitchRequest,
) -> Result<WorkerModeSwitchResponse> {
    let doc = decode_worker_source(request.source)?;
    let surface = worker_surface_for_document(&doc, request.target_mode, request.encode_options)?;
    let tree_rows =
        flatten_expanded_value_tree(&doc.value, &default_worker_expanded_paths(), usize::MAX);
    let search_result = if request.search_query.trim().is_empty() {
        None
    } else {
        Some(search_value_tree(
            &doc.value,
            &request.search_query,
            usize::MAX,
        ))
    };

    Ok(WorkerModeSwitchResponse {
        doc,
        surface,
        tree_rows,
        search_result,
        search_query: request.search_query,
    })
}

pub fn perform_worker_parse(request: WorkerParseRequest) -> Result<WorkerParseResponse> {
    let doc = decode_worker_source(request.source)?;
    let tree_rows =
        flatten_expanded_value_tree(&doc.value, &default_worker_expanded_paths(), usize::MAX);
    let search_result = if request.search_query.trim().is_empty() {
        None
    } else {
        Some(search_value_tree(
            &doc.value,
            &request.search_query,
            usize::MAX,
        ))
    };

    Ok(WorkerParseResponse {
        doc,
        tree_rows,
        search_result,
        search_query: request.search_query,
    })
}

pub fn perform_worker_rton_size(request: WorkerRtonSizeRequest) -> Result<WorkerRtonSizeResponse> {
    let doc = decode_worker_source(request.source)?;
    let bytes = encode_rton_bytes(&doc.value, request.encode_options)?;
    Ok(WorkerRtonSizeResponse {
        byte_len: bytes.len(),
    })
}

pub fn perform_worker_text_surface(
    request: WorkerTextSurfaceRequest,
) -> Result<WorkerTextSurfaceResponse> {
    let text = String::from_utf8_lossy(&request.bytes).to_string();
    let line_offsets = text_line_offsets(&text);
    let byte_count = text.len();
    let line_count = line_offsets.len();
    Ok(WorkerTextSurfaceResponse {
        surface: WorkerSurface::Text {
            text,
            line_offsets,
            byte_count,
            line_count,
            format: request.format,
        },
    })
}

pub fn perform_worker_open_text(request: WorkerOpenTextRequest) -> Result<WorkerOpenTextResponse> {
    let text = String::from_utf8_lossy(&request.bytes).to_string();
    let line_offsets = text_line_offsets(&text);
    let byte_count = text.len();
    let line_count = line_offsets.len();
    let parsed = parse_text(&text, request.format).ok();
    let tree_rows = parsed
        .as_ref()
        .map(|doc| {
            flatten_expanded_value_tree(&doc.value, &default_worker_expanded_paths(), usize::MAX)
        })
        .unwrap_or_else(|| TreeRows {
            rows: Vec::new(),
            truncated: false,
        });
    let search_result = parsed.as_ref().and_then(|doc| {
        (!request.search_query.trim().is_empty())
            .then(|| search_value_tree(&doc.value, &request.search_query, usize::MAX))
    });

    Ok(WorkerOpenTextResponse {
        doc: parsed,
        surface: WorkerSurface::Text {
            text,
            line_offsets,
            byte_count,
            line_count,
            format: request.format,
        },
        tree_rows,
        search_result,
        search_query: request.search_query,
    })
}

fn decode_worker_source(source: WorkerDocumentSource) -> Result<DecodedDocument> {
    match source {
        WorkerDocumentSource::RtonBytes(bytes) => decode_rton_bytes(&bytes),
        WorkerDocumentSource::Text { text, format } => parse_text(&text, format),
    }
}

fn worker_surface_for_document(
    doc: &DecodedDocument,
    mode: WorkerEditorMode,
    encode_options: EncodeOptions,
) -> Result<WorkerSurface> {
    match mode {
        WorkerEditorMode::RtonHex => Ok(WorkerSurface::RtonBytes(encode_rton_bytes(
            &doc.value,
            encode_options,
        )?)),
        WorkerEditorMode::Json | WorkerEditorMode::Yaml | WorkerEditorMode::Toml => {
            let format = mode.text_format().expect("text worker mode has format");
            let text = value_to_text(&doc.value, format)?;
            let line_offsets = text_line_offsets(&text);
            let byte_count = text.len();
            let line_count = line_offsets.len();
            Ok(WorkerSurface::Text {
                text,
                line_offsets,
                byte_count,
                line_count,
                format,
            })
        }
    }
}

fn default_worker_expanded_paths() -> HashSet<String> {
    HashSet::from(["$".to_string()])
}

fn text_line_offsets(text: &str) -> Vec<usize> {
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
