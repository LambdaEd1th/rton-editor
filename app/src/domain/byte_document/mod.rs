mod document;
mod mmap;
mod pieces;
mod read;
mod reader;
mod source;

pub(crate) use document::ByteDocument;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use mmap::MappedByteSource;
#[cfg(test)]
pub(crate) use pieces::{BytePiece, BytePieceSource, PieceBytes};
#[cfg(not(test))]
pub(crate) use pieces::{BytePiece, BytePieceSource, PieceBytes};
pub(crate) use read::ByteRead;
pub(crate) use reader::ByteDocumentReader;
pub(crate) use source::ByteSource;
