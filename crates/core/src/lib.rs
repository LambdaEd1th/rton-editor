use serde::{Deserialize, Serialize};
use serde_rton::{
    BinaryBlob, Rtid, Value, VarInt, decrypt_data, encrypt_data, from_bytes, to_bytes,
    to_compact_bytes,
};
use std::fmt::Display;
use std::str::FromStr;
use thiserror::Error;

pub use serde_rton::Value as RtonValue;

pub const ENCRYPTED_RTON_PREFIX: &[u8] = &[0x10, 0x00];
pub const TREE_ROW_LIMIT: usize = 900;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("{0}")]
    Rton(String),
    #[error("Invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Invalid YAML: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("Invalid TOML: {0}")]
    TomlDe(#[from] toml::de::Error),
    #[error("Cannot write TOML: {0}")]
    TomlSer(#[from] toml::ser::Error),
    #[error("Invalid hex bytes: {0}")]
    Hex(#[from] hex::FromHexError),
    #[error("JSON does not support non-finite number: {0}")]
    NonFiniteJson(&'static str),
    #[error("Invalid value path: {0}")]
    InvalidPath(String),
    #[error("Value path not found: {0}")]
    PathNotFound(String),
    #[error("{0} values cannot be edited as scalars")]
    NonEditableValue(&'static str),
    #[error("Invalid {kind} value: {message}")]
    InvalidScalarValue { kind: &'static str, message: String },
}

pub type Result<T> = std::result::Result<T, CoreError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextFormat {
    Json,
    Yaml,
    Toml,
}

impl TextFormat {
    pub fn extension(self) -> &'static str {
        match self {
            TextFormat::Json => "json",
            TextFormat::Yaml => "yaml",
            TextFormat::Toml => "toml",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            TextFormat::Json => "JSON",
            TextFormat::Yaml => "YAML",
            TextFormat::Toml => "TOML",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceFormat {
    Rton,
    Json,
    Yaml,
    Toml,
    Unknown,
}

impl SourceFormat {
    pub fn from_file_name(name: &str) -> Self {
        let Some(extension) = name.rsplit('.').next() else {
            return SourceFormat::Unknown;
        };

        match extension.to_ascii_lowercase().as_str() {
            "rton" | "dat" => SourceFormat::Rton,
            "json" => SourceFormat::Json,
            "yaml" | "yml" => SourceFormat::Yaml,
            "toml" => SourceFormat::Toml,
            _ => SourceFormat::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinaryEncoding {
    Standard,
    Compact,
}

impl BinaryEncoding {
    pub fn label(self) -> &'static str {
        match self {
            BinaryEncoding::Standard => "Standard RTON",
            BinaryEncoding::Compact => "Compact RTON",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncodeOptions {
    pub encoding: BinaryEncoding,
    pub encrypted: bool,
}

impl Default for EncodeOptions {
    fn default() -> Self {
        Self {
            encoding: BinaryEncoding::Standard,
            encrypted: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecodedDocument {
    pub value: Value,
    pub encrypted_source: bool,
    pub byte_len: Option<usize>,
    pub stats: ValueStats,
}

impl DecodedDocument {
    pub fn new(value: Value, encrypted_source: bool, byte_len: Option<usize>) -> Self {
        let stats = ValueStats::from_value(&value);
        Self {
            value,
            encrypted_source,
            byte_len,
            stats,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValueStats {
    pub nodes: usize,
    pub objects: usize,
    pub arrays: usize,
    pub strings: usize,
    pub numbers: usize,
    pub bools: usize,
    pub nulls: usize,
    pub binaries: usize,
    pub rtids: usize,
    pub max_depth: usize,
}

impl ValueStats {
    pub fn from_value(value: &Value) -> Self {
        let mut stats = ValueStats::default();
        stats.visit(value, 0);
        stats
    }

    fn visit(&mut self, value: &Value, depth: usize) {
        self.nodes += 1;
        self.max_depth = self.max_depth.max(depth);

        match value {
            Value::Null => self.nulls += 1,
            Value::Bool(_) => self.bools += 1,
            Value::Int8(_)
            | Value::UInt8(_)
            | Value::Int16(_)
            | Value::UInt16(_)
            | Value::Int32(_)
            | Value::UInt32(_)
            | Value::Int64(_)
            | Value::UInt64(_)
            | Value::VarIntI32(_)
            | Value::VarIntU32(_)
            | Value::VarIntI64(_)
            | Value::VarIntU64(_)
            | Value::Float(_)
            | Value::Double(_) => self.numbers += 1,
            Value::String(_) => self.strings += 1,
            Value::Binary(_) => self.binaries += 1,
            Value::Rtid(_) => self.rtids += 1,
            Value::Array(items) => {
                self.arrays += 1;
                for item in items {
                    self.visit(item, depth + 1);
                }
            }
            Value::Object(entries) => {
                self.objects += 1;
                for (_, item) in entries {
                    self.visit(item, depth + 1);
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreeRows {
    pub rows: Vec<ValueRow>,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValueRow {
    pub path: String,
    pub label: String,
    pub kind: String,
    pub preview: String,
    pub depth: usize,
    pub child_count: usize,
}

pub fn decode_rton_bytes(bytes: &[u8]) -> Result<DecodedDocument> {
    let (plain, encrypted_source) = if bytes.starts_with(ENCRYPTED_RTON_PREFIX) {
        (
            decrypt_data(bytes).map_err(|error| CoreError::Rton(error.to_string()))?,
            true,
        )
    } else {
        (bytes.to_vec(), false)
    };
    let value = from_bytes::<Value>(&plain).map_err(|error| CoreError::Rton(error.to_string()))?;
    Ok(DecodedDocument::new(
        value,
        encrypted_source,
        Some(bytes.len()),
    ))
}

pub fn encode_rton_bytes(value: &Value, options: EncodeOptions) -> Result<Vec<u8>> {
    let bytes = match options.encoding {
        BinaryEncoding::Standard => to_bytes(value),
        BinaryEncoding::Compact => to_compact_bytes(value),
    }
    .map_err(|error| CoreError::Rton(error.to_string()))?;

    if options.encrypted {
        encrypt_data(&bytes).map_err(|error| CoreError::Rton(error.to_string()))
    } else {
        Ok(bytes)
    }
}

pub fn parse_text(text: &str, format: TextFormat) -> Result<DecodedDocument> {
    let mut value = match format {
        TextFormat::Json => parse_editor_json(text)?,
        TextFormat::Yaml => serde_yaml::from_str::<Value>(text)?,
        TextFormat::Toml => toml::from_str::<Value>(text)?,
    };
    normalize_editor_value(&mut value);
    Ok(DecodedDocument::new(value, false, None))
}

pub fn value_to_text(value: &Value, format: TextFormat) -> Result<String> {
    match format {
        TextFormat::Json => {
            ensure_json_safe_value(value)?;
            serde_json::to_string_pretty(value).map_err(CoreError::Json)
        }
        TextFormat::Yaml => serde_yaml::to_string(value).map_err(CoreError::Yaml),
        TextFormat::Toml => toml::to_string_pretty(value).map_err(CoreError::TomlSer),
    }
}

pub fn decode_hex_rton(text: &str) -> Result<DecodedDocument> {
    let bytes = hex_to_bytes(text)?;
    decode_rton_bytes(&bytes)
}

pub fn bytes_to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().saturating_mul(3));
    for (index, byte) in bytes.iter().enumerate() {
        if index > 0 {
            if index % 16 == 0 {
                out.push('\n');
            } else {
                out.push(' ');
            }
        }
        out.push_str(&format!("{byte:02X}"));
    }
    out
}

pub fn hex_to_bytes(text: &str) -> Result<Vec<u8>> {
    let filtered: String = text.chars().filter(|ch| ch.is_ascii_hexdigit()).collect();
    Ok(hex::decode(filtered)?)
}

pub fn format_bytes(bytes: usize) -> String {
    const UNITS: &[&str] = &["B", "KiB", "MiB", "GiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} {}", UNITS[unit])
    } else {
        format!("{value:.2} {}", UNITS[unit])
    }
}

pub fn flatten_value_tree(value: &Value, query: &str, limit: usize) -> TreeRows {
    let mut rows = Vec::new();
    let needle = query.trim().to_ascii_lowercase();
    let mut truncated = false;
    flatten_inner(
        value,
        "$",
        "root",
        0,
        &needle,
        limit,
        &mut rows,
        &mut truncated,
    );
    TreeRows { rows, truncated }
}

pub fn value_at_path<'a>(value: &'a Value, path: &str) -> Result<&'a Value> {
    let segments = parse_value_path(path)?;
    let mut current = value;

    for segment in segments {
        current = value_child_at(current, segment, path)?;
    }

    Ok(current)
}

pub fn replace_value_at_path(root: &mut Value, path: &str, replacement: Value) -> Result<()> {
    let segments = parse_value_path(path)?;
    if segments.is_empty() {
        *root = replacement;
        return Ok(());
    }

    let (last, parents) = segments
        .split_last()
        .expect("non-empty segments have a last segment");
    let mut current = root;
    for segment in parents {
        current = value_child_at_mut(current, *segment, path)?;
    }

    match (current, last) {
        (Value::Array(items), PathSegment::Array(index)) => {
            let Some(item) = items.get_mut(*index) else {
                return Err(CoreError::PathNotFound(path.to_string()));
            };
            *item = replacement;
            Ok(())
        }
        (Value::Object(entries), PathSegment::Object(index)) => {
            let Some((_, item)) = entries.get_mut(*index) else {
                return Err(CoreError::PathNotFound(path.to_string()));
            };
            *item = replacement;
            Ok(())
        }
        _ => Err(CoreError::PathNotFound(path.to_string())),
    }
}

pub fn scalar_edit_text(value: &Value) -> Option<String> {
    match value {
        Value::Null => Some("null".to_string()),
        Value::Bool(value) => Some(value.to_string()),
        Value::Int8(value) => Some(value.to_string()),
        Value::UInt8(value) => Some(value.to_string()),
        Value::Int16(value) => Some(value.to_string()),
        Value::UInt16(value) => Some(value.to_string()),
        Value::Int32(value) => Some(value.to_string()),
        Value::UInt32(value) => Some(value.to_string()),
        Value::Int64(value) => Some(value.to_string()),
        Value::UInt64(value) => Some(value.to_string()),
        Value::VarIntI32(value) => Some(value.0.to_string()),
        Value::VarIntU32(value) => Some(value.0.to_string()),
        Value::VarIntI64(value) => Some(value.0.to_string()),
        Value::VarIntU64(value) => Some(value.0.to_string()),
        Value::Float(value) => Some(value.to_string()),
        Value::Double(value) => Some(value.to_string()),
        Value::String(value) => Some(value.clone()),
        Value::Binary(value) => Some(value.to_string()),
        Value::Rtid(value) => Some(value.to_string()),
        Value::Array(_) | Value::Object(_) => None,
    }
}

pub fn parse_scalar_edit(existing: &Value, text: &str) -> Result<Value> {
    let trimmed = text.trim();
    match existing {
        Value::Null => {
            if trimmed.eq_ignore_ascii_case("null") {
                Ok(Value::Null)
            } else {
                Err(invalid_scalar("null", "expected null"))
            }
        }
        Value::Bool(_) if trimmed.eq_ignore_ascii_case("true") => Ok(Value::Bool(true)),
        Value::Bool(_) if trimmed.eq_ignore_ascii_case("false") => Ok(Value::Bool(false)),
        Value::Bool(_) => Err(invalid_scalar("bool", "expected true or false")),
        Value::Int8(_) => parse_typed_scalar::<i8>(trimmed, "i8").map(Value::Int8),
        Value::UInt8(_) => parse_typed_scalar::<u8>(trimmed, "u8").map(Value::UInt8),
        Value::Int16(_) => parse_typed_scalar::<i16>(trimmed, "i16").map(Value::Int16),
        Value::UInt16(_) => parse_typed_scalar::<u16>(trimmed, "u16").map(Value::UInt16),
        Value::Int32(_) => parse_typed_scalar::<i32>(trimmed, "i32").map(Value::Int32),
        Value::UInt32(_) => parse_typed_scalar::<u32>(trimmed, "u32").map(Value::UInt32),
        Value::Int64(_) => parse_typed_scalar::<i64>(trimmed, "i64").map(Value::Int64),
        Value::UInt64(_) => parse_typed_scalar::<u64>(trimmed, "u64").map(Value::UInt64),
        Value::VarIntI32(_) => parse_typed_scalar::<i32>(trimmed, "varint i32")
            .map(|value| Value::VarIntI32(VarInt(value))),
        Value::VarIntU32(_) => parse_typed_scalar::<u32>(trimmed, "varint u32")
            .map(|value| Value::VarIntU32(VarInt(value))),
        Value::VarIntI64(_) => parse_typed_scalar::<i64>(trimmed, "varint i64")
            .map(|value| Value::VarIntI64(VarInt(value))),
        Value::VarIntU64(_) => parse_typed_scalar::<u64>(trimmed, "varint u64")
            .map(|value| Value::VarIntU64(VarInt(value))),
        Value::Float(_) => parse_typed_scalar::<f32>(trimmed, "f32").map(Value::Float),
        Value::Double(_) => parse_typed_scalar::<f64>(trimmed, "f64").map(Value::Double),
        Value::String(_) => Ok(Value::String(text.to_string())),
        Value::Binary(_) => BinaryBlob::from_str(trimmed)
            .map(Value::Binary)
            .map_err(|error| invalid_scalar("binary", error)),
        Value::Rtid(_) => Rtid::from_str(trimmed)
            .map(Value::Rtid)
            .map_err(|error| invalid_scalar("rtid", error)),
        Value::Array(_) => Err(CoreError::NonEditableValue("array")),
        Value::Object(_) => Err(CoreError::NonEditableValue("object")),
    }
}

pub fn edit_value_at_path(root: &mut Value, path: &str, text: &str) -> Result<()> {
    let replacement = {
        let current = value_at_path(root, path)?;
        parse_scalar_edit(current, text)?
    };
    replace_value_at_path(root, path, replacement)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PathSegment {
    Array(usize),
    Object(usize),
}

fn parse_value_path(path: &str) -> Result<Vec<PathSegment>> {
    if path == "$" {
        return Ok(Vec::new());
    }
    if !path.starts_with('$') {
        return Err(CoreError::InvalidPath(path.to_string()));
    }

    let mut segments = Vec::new();
    let mut rest = &path[1..];
    while !rest.is_empty() {
        if let Some(next) = rest.strip_prefix('[') {
            let Some(close) = next.find(']') else {
                return Err(CoreError::InvalidPath(path.to_string()));
            };
            let index = parse_path_index(&next[..close], path)?;
            segments.push(PathSegment::Array(index));
            rest = &next[close + 1..];
            continue;
        }

        if let Some(next) = rest.strip_prefix('.') {
            let Some(hash_index) = find_unescaped_hash(next) else {
                return Err(CoreError::InvalidPath(path.to_string()));
            };
            let after_hash = &next[hash_index + 1..];
            let digit_len = after_hash.bytes().take_while(u8::is_ascii_digit).count();
            if digit_len == 0 {
                return Err(CoreError::InvalidPath(path.to_string()));
            }
            let index = parse_path_index(&after_hash[..digit_len], path)?;
            segments.push(PathSegment::Object(index));
            rest = &after_hash[digit_len..];
            if !rest.is_empty() && !rest.starts_with('.') && !rest.starts_with('[') {
                return Err(CoreError::InvalidPath(path.to_string()));
            }
            continue;
        }

        return Err(CoreError::InvalidPath(path.to_string()));
    }

    Ok(segments)
}

fn find_unescaped_hash(segment: &str) -> Option<usize> {
    let mut escaped = false;
    for (index, ch) in segment.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            '#' => return Some(index),
            _ => {}
        }
    }
    None
}

fn parse_path_index(text: &str, path: &str) -> Result<usize> {
    text.parse::<usize>()
        .map_err(|_| CoreError::InvalidPath(path.to_string()))
}

fn value_child_at<'a>(value: &'a Value, segment: PathSegment, path: &str) -> Result<&'a Value> {
    match (value, segment) {
        (Value::Array(items), PathSegment::Array(index)) => items
            .get(index)
            .ok_or_else(|| CoreError::PathNotFound(path.to_string())),
        (Value::Object(entries), PathSegment::Object(index)) => entries
            .get(index)
            .map(|(_, value)| value)
            .ok_or_else(|| CoreError::PathNotFound(path.to_string())),
        _ => Err(CoreError::PathNotFound(path.to_string())),
    }
}

fn value_child_at_mut<'a>(
    value: &'a mut Value,
    segment: PathSegment,
    path: &str,
) -> Result<&'a mut Value> {
    match (value, segment) {
        (Value::Array(items), PathSegment::Array(index)) => items
            .get_mut(index)
            .ok_or_else(|| CoreError::PathNotFound(path.to_string())),
        (Value::Object(entries), PathSegment::Object(index)) => entries
            .get_mut(index)
            .map(|(_, value)| value)
            .ok_or_else(|| CoreError::PathNotFound(path.to_string())),
        _ => Err(CoreError::PathNotFound(path.to_string())),
    }
}

fn parse_typed_scalar<T>(text: &str, kind: &'static str) -> Result<T>
where
    T: FromStr,
    T::Err: Display,
{
    text.parse::<T>()
        .map_err(|error| invalid_scalar(kind, error))
}

fn invalid_scalar(kind: &'static str, message: impl Display) -> CoreError {
    CoreError::InvalidScalarValue {
        kind,
        message: message.to_string(),
    }
}

fn flatten_inner(
    value: &Value,
    path: &str,
    label: &str,
    depth: usize,
    needle: &str,
    limit: usize,
    rows: &mut Vec<ValueRow>,
    truncated: &mut bool,
) {
    if rows.len() >= limit {
        *truncated = true;
        return;
    }

    let kind = value_kind(value).to_string();
    let preview = value_preview(value);
    let child_count = value_child_count(value);
    let row = ValueRow {
        path: path.to_string(),
        label: label.to_string(),
        kind,
        preview,
        depth,
        child_count,
    };

    if needle.is_empty()
        || row.path.to_ascii_lowercase().contains(needle)
        || row.label.to_ascii_lowercase().contains(needle)
        || row.kind.to_ascii_lowercase().contains(needle)
        || row.preview.to_ascii_lowercase().contains(needle)
    {
        rows.push(row);
    }

    match value {
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                let child_path = format!("{path}[{index}]");
                let child_label = format!("[{index}]");
                flatten_inner(
                    item,
                    &child_path,
                    &child_label,
                    depth + 1,
                    needle,
                    limit,
                    rows,
                    truncated,
                );
                if *truncated {
                    return;
                }
            }
        }
        Value::Object(entries) => {
            for (index, (key, item)) in entries.iter().enumerate() {
                let child_path = format!("{path}.{}#{index}", escape_path_key(key));
                flatten_inner(
                    item,
                    &child_path,
                    key,
                    depth + 1,
                    needle,
                    limit,
                    rows,
                    truncated,
                );
                if *truncated {
                    return;
                }
            }
        }
        _ => {}
    }
}

pub fn value_kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Int8(_) => "i8",
        Value::UInt8(_) => "u8",
        Value::Int16(_) => "i16",
        Value::UInt16(_) => "u16",
        Value::Int32(_) => "i32",
        Value::UInt32(_) => "u32",
        Value::Int64(_) => "i64",
        Value::UInt64(_) => "u64",
        Value::VarIntI32(_) => "varint i32",
        Value::VarIntU32(_) => "varint u32",
        Value::VarIntI64(_) => "varint i64",
        Value::VarIntU64(_) => "varint u64",
        Value::Float(_) => "f32",
        Value::Double(_) => "f64",
        Value::String(_) => "string",
        Value::Binary(_) => "binary",
        Value::Rtid(_) => "rtid",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

pub fn value_preview(value: &Value) -> String {
    let preview = match value {
        Value::Null => "null".to_string(),
        Value::Bool(value) => value.to_string(),
        Value::Int8(value) => value.to_string(),
        Value::UInt8(value) => value.to_string(),
        Value::Int16(value) => value.to_string(),
        Value::UInt16(value) => value.to_string(),
        Value::Int32(value) => value.to_string(),
        Value::UInt32(value) => value.to_string(),
        Value::Int64(value) => value.to_string(),
        Value::UInt64(value) => value.to_string(),
        Value::VarIntI32(value) => value.0.to_string(),
        Value::VarIntU32(value) => value.0.to_string(),
        Value::VarIntI64(value) => value.0.to_string(),
        Value::VarIntU64(value) => value.0.to_string(),
        Value::Float(value) => value.to_string(),
        Value::Double(value) => value.to_string(),
        Value::String(value) => format!("\"{value}\""),
        Value::Binary(value) => value.to_string(),
        Value::Rtid(value) => value.to_string(),
        Value::Array(items) => format!("{} items", items.len()),
        Value::Object(entries) => format!("{} entries", entries.len()),
    };
    truncate(&preview, 140)
}

fn value_child_count(value: &Value) -> usize {
    match value {
        Value::Array(items) => items.len(),
        Value::Object(entries) => entries.len(),
        _ => 0,
    }
}

fn parse_editor_json(json: &str) -> Result<Value> {
    match serde_json::from_str::<Value>(json) {
        Ok(mut value) => {
            normalize_editor_value(&mut value);
            Ok(value)
        }
        Err(primary_error) => {
            let json_value = serde_json::from_str::<serde_json::Value>(json)
                .map_err(|_| CoreError::Json(primary_error))?;
            json_value_to_rton(json_value).map_err(CoreError::Rton)
        }
    }
}

fn json_value_to_rton(value: serde_json::Value) -> std::result::Result<Value, String> {
    match value {
        serde_json::Value::Null => Ok(Value::Rtid(Rtid::Null)),
        serde_json::Value::Bool(value) => Ok(Value::Bool(value)),
        serde_json::Value::Number(number) => {
            if let Some(value) = number.as_i64() {
                Ok(Value::new_int(value))
            } else if let Some(value) = number.as_u64() {
                Ok(Value::new_uint(value))
            } else if let Some(value) = number.as_f64() {
                Ok(Value::Double(value))
            } else {
                Err(format!("Unsupported JSON number: {number}"))
            }
        }
        serde_json::Value::String(text) => {
            if text.starts_with("RTID(") {
                Rtid::from_str(&text)
                    .map(Value::Rtid)
                    .map_err(|error| error.to_string())
            } else if text.starts_with("$BINARY(") {
                BinaryBlob::from_str(&text)
                    .map(Value::Binary)
                    .map_err(|error| error.to_string())
            } else {
                Ok(Value::String(text))
            }
        }
        serde_json::Value::Array(items) => items
            .into_iter()
            .map(json_value_to_rton)
            .collect::<std::result::Result<Vec<_>, _>>()
            .map(Value::Array),
        serde_json::Value::Object(map) => map
            .into_iter()
            .map(|(key, value)| json_value_to_rton(value).map(|value| (key, value)))
            .collect::<std::result::Result<Vec<_>, _>>()
            .map(Value::Object),
    }
}

fn normalize_editor_value(value: &mut Value) {
    match value {
        Value::String(text) if text.starts_with("$BINARY(") => {
            if let Ok(blob) = BinaryBlob::from_str(text) {
                *value = Value::Binary(blob);
            }
        }
        Value::Array(items) => {
            for item in items {
                normalize_editor_value(item);
            }
        }
        Value::Object(entries) => {
            for (_, item) in entries {
                normalize_editor_value(item);
            }
        }
        _ => {}
    }
}

fn ensure_json_safe_value(value: &Value) -> Result<()> {
    match value {
        Value::Float(value) if !value.is_finite() => {
            Err(CoreError::NonFiniteJson(describe_non_finite(*value as f64)))
        }
        Value::Double(value) if !value.is_finite() => {
            Err(CoreError::NonFiniteJson(describe_non_finite(*value)))
        }
        Value::Array(items) => {
            for item in items {
                ensure_json_safe_value(item)?;
            }
            Ok(())
        }
        Value::Object(entries) => {
            for (_, item) in entries {
                ensure_json_safe_value(item)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn describe_non_finite(value: f64) -> &'static str {
    if value.is_nan() {
        "NaN"
    } else if value.is_sign_negative() {
        "-Infinity"
    } else {
        "Infinity"
    }
}

fn escape_path_key(key: &str) -> String {
    key.replace('.', "\\.").replace('#', "\\#")
}

fn truncate(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let mut out: String = value.chars().take(max_chars.saturating_sub(1)).collect();
    out.push_str("...");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
      "name": "Peashooter",
      "enabled": true,
      "cost": 100,
      "resource": "RTID(0)",
      "payload": "$BINARY(\"0A0B0C\", 3)"
    }"#;

    #[test]
    fn parses_editor_json_with_rton_strings() {
        let doc = parse_text(SAMPLE, TextFormat::Json).expect("json parses");
        assert_eq!(doc.stats.objects, 1);
        assert_eq!(doc.stats.binaries, 1);
        assert_eq!(doc.stats.rtids, 1);
    }

    #[test]
    fn round_trips_standard_rton_bytes() {
        let doc = parse_text(SAMPLE, TextFormat::Json).expect("json parses");
        let bytes = encode_rton_bytes(&doc.value, EncodeOptions::default()).expect("rton encodes");
        let decoded = decode_rton_bytes(&bytes).expect("rton decodes");
        assert_eq!(decoded.value, doc.value);
        assert_eq!(decoded.byte_len, Some(bytes.len()));
    }

    #[test]
    fn decodes_hex_rton_text() {
        let doc = parse_text(SAMPLE, TextFormat::Json).expect("json parses");
        let bytes = encode_rton_bytes(&doc.value, EncodeOptions::default()).expect("rton encodes");
        let hex = bytes_to_hex(&bytes);
        let decoded = decode_hex_rton(&hex).expect("hex rton decodes");
        assert_eq!(decoded.value, doc.value);
    }

    #[test]
    fn reads_and_edits_values_by_tree_path() {
        let mut value = Value::Object(vec![
            (
                "a.b#c".to_string(),
                Value::Array(vec![Value::String("first".to_string()), Value::Bool(true)]),
            ),
            ("count".to_string(), Value::UInt16(12)),
            ("resource".to_string(), Value::Rtid(Rtid::Null)),
        ]);

        assert_eq!(
            value_at_path(&value, "$.a\\.b\\#c#0[0]").expect("path resolves"),
            &Value::String("first".to_string())
        );

        edit_value_at_path(&mut value, "$.a\\.b\\#c#0[1]", "false").expect("bool edits");
        assert_eq!(
            value_at_path(&value, "$.a\\.b\\#c#0[1]").expect("path resolves"),
            &Value::Bool(false)
        );

        edit_value_at_path(&mut value, "$.count#1", "65535").expect("u16 edits");
        assert_eq!(
            value_at_path(&value, "$.count#1").expect("path resolves"),
            &Value::UInt16(u16::MAX)
        );

        edit_value_at_path(&mut value, "$.resource#2", "RTID(example@parent)").expect("rtid edits");
        assert_eq!(
            scalar_edit_text(value_at_path(&value, "$.resource#2").expect("path resolves")),
            Some("RTID(example@parent)".to_string())
        );
    }

    #[test]
    fn rejects_invalid_scalar_edits_without_changing_value() {
        let mut value = Value::Object(vec![("count".to_string(), Value::UInt8(7))]);

        let error = edit_value_at_path(&mut value, "$.count#0", "300")
            .expect_err("out-of-range u8 is rejected");
        assert!(error.to_string().contains("Invalid u8 value"));
        assert_eq!(
            value_at_path(&value, "$.count#0").expect("path resolves"),
            &Value::UInt8(7)
        );
    }

    #[test]
    fn round_trips_encrypted_rton_bytes() {
        let doc = parse_text(SAMPLE, TextFormat::Json).expect("json parses");
        let bytes = encode_rton_bytes(
            &doc.value,
            EncodeOptions {
                encoding: BinaryEncoding::Standard,
                encrypted: true,
            },
        )
        .expect("encrypted rton encodes");
        assert!(bytes.starts_with(ENCRYPTED_RTON_PREFIX));
        let decoded = decode_rton_bytes(&bytes).expect("encrypted rton decodes");
        assert!(decoded.encrypted_source);
        assert_eq!(decoded.value, doc.value);
    }
}
