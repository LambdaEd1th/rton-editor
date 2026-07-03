#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RtonTagInfo {
    pub(crate) name: &'static str,
    pub(crate) category: &'static str,
    pub(crate) payload_kind: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RtonPayloadInfo {
    pub(crate) label: String,
    pub(crate) value: String,
    pub(crate) bytes: Option<String>,
    pub(crate) range: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RtonVarintInfo {
    pub(crate) value: String,
    pub(crate) zigzag: String,
    pub(crate) length: usize,
    pub(crate) bytes: String,
    pub(crate) next_offset: usize,
    pub(crate) raw_value: u128,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RtonStringMode {
    Direct,
    Definition,
    Reference,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RtonStringInfo {
    pub(crate) mode: RtonStringMode,
    pub(crate) encoding: &'static str,
    pub(crate) table: Option<&'static str>,
    pub(crate) index: Option<String>,
    pub(crate) length: Option<String>,
    pub(crate) byte_length: Option<String>,
    pub(crate) text: Option<String>,
    pub(crate) resolved_text: Option<String>,
    pub(crate) scan_limited: bool,
    pub(crate) scan_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RtonStringTables {
    pub(crate) ascii: Vec<String>,
    pub(crate) utf8: Vec<String>,
    pub(crate) limited: bool,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RtonAsciiRun {
    pub(crate) text: String,
    pub(crate) start: usize,
    pub(crate) end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ParsedStringPayload {
    pub(super) text: String,
    pub(super) length: Option<u128>,
    pub(super) byte_length: u128,
    pub(super) end_offset: usize,
}
