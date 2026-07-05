pub use serde_rton::Value as RtonValue;

mod batch_export;
mod byte_read;
mod document;
mod error;
mod formats;
mod rton;
mod rton_inspector;
mod scalar_edit;
mod text;
mod value_path;
mod value_tree;
mod worker_protocol;

pub use batch_export::{
    BatchExportMode, ZipArchiveBuilder, ZipFileEntry, batch_output_path, create_zip_archive,
    encode_batch_export_document, unique_zip_path,
};
pub use byte_read::ByteRead;
pub use document::{DecodedDocument, ValueStats};
pub use error::{CoreError, Result};
pub use formats::{BinaryEncoding, EncodeOptions, SourceFormat, TextFormat};
pub use rton::{
    ENCRYPTED_RTON_PREFIX, bytes_to_hex, decode_hex_rton, decode_rton_bytes, decode_rton_reader,
    decrypt_rton_bytes_if_needed, encode_rton_bytes, format_bytes, hex_to_bytes,
};
pub use rton_inspector::{
    RtonAsciiRun, RtonPayloadInfo, RtonStringInfo, RtonStringMode, RtonStringTables, RtonTagInfo,
    RtonVarintInfo, format_inspector_offset, inspect_ascii_run, inspect_rton_payload,
    inspect_rton_string_info, inspect_special_region, locate_rton_value_offset,
    maybe_collect_string_tables, read_rton_varint, rton_tag_info,
};
pub use scalar_edit::{edit_value_at_path, parse_scalar_edit, scalar_edit_text};
pub use text::{TextRender, parse_text, value_to_text, value_to_text_limited};
pub use value_path::{
    ValuePathSegment, parse_value_path_segments, replace_value_at_path, value_at_path,
};
pub use value_tree::{
    TREE_ROW_LIMIT, TreeRows, ValueRow, ValueSearchMatch, ValueSearchResult,
    flatten_expanded_value_tree, flatten_value_tree, search_value_tree, value_kind, value_preview,
};
pub use worker_protocol::{
    WorkerDocumentSource, WorkerEditorMode, WorkerModeSwitchRequest, WorkerModeSwitchResponse,
    WorkerOpenTextRequest, WorkerOpenTextResponse, WorkerParseRequest, WorkerParseResponse,
    WorkerRtonSizeRequest, WorkerRtonSizeResponse, WorkerSurface, WorkerTextSurfaceRequest,
    WorkerTextSurfaceResponse, perform_worker_mode_switch, perform_worker_open_text,
    perform_worker_parse, perform_worker_rton_size, perform_worker_text_surface,
};

#[cfg(test)]
mod tests;
