use dioxus_html::FileData;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::domain::{ByteDocument, EditorTabState, OpenTabError, create_tab_from_byte_document};
use crate::platform;

use super::state::{LoadedFileDraft, LoadedFileSource, LoadedFileState};

pub(crate) fn loaded_file_draft_from_native(file: platform::NativeOpenFile) -> LoadedFileDraft {
    loaded_file_draft_from_native_path(file.display_name, file.path)
}

fn loaded_file_draft_from_native_path(display_name: String, path: PathBuf) -> LoadedFileDraft {
    let size = std::fs::metadata(&path)
        .ok()
        .and_then(|metadata| usize::try_from(metadata.len()).ok());
    LoadedFileDraft {
        display_name,
        source: LoadedFileSource::NativePath(path),
        size,
    }
}

pub(crate) async fn loaded_file_draft_from_file_data(
    display_name: String,
    file: FileData,
) -> Result<LoadedFileDraft, String> {
    if let Some(path) = file_data_real_path(&file) {
        return Ok(loaded_file_draft_from_native_path(display_name, path));
    }

    #[cfg(target_arch = "wasm32")]
    if let Some(web_file) = super::web_drop::file_data_web_file(&file) {
        return Ok(LoadedFileDraft {
            display_name,
            size: super::web_drop::web_file_size(&web_file),
            source: LoadedFileSource::WebFile(web_file),
        });
    }

    let bytes = file.read_bytes().await.map_err(|error| error.to_string())?;
    let size = bytes.len();
    Ok(LoadedFileDraft {
        display_name,
        source: LoadedFileSource::Bytes(Arc::<[u8]>::from(bytes.as_ref())),
        size: Some(size),
    })
}

pub(crate) async fn create_tab_from_loaded_file(
    id: usize,
    file: &LoadedFileState,
) -> Result<EditorTabState, OpenTabError> {
    match &file.source {
        LoadedFileSource::NativePath(path) => {
            create_tab_from_file_path(id, file.display_name.clone(), path)
        }
        LoadedFileSource::Bytes(bytes) => create_tab_from_byte_document(
            id,
            file.display_name.clone(),
            ByteDocument::from_arc(bytes.clone()),
        )
        .map_err(|error| OpenTabError::Decode(error.to_string())),
        #[cfg(target_arch = "wasm32")]
        LoadedFileSource::WebFile(web_file) => {
            let bytes = super::web_drop::read_web_file_bytes(web_file)
                .await
                .map_err(OpenTabError::Read)?;
            create_tab_from_byte_document(
                id,
                file.display_name.clone(),
                ByteDocument::from_vec(bytes),
            )
            .map_err(|error| OpenTabError::Decode(error.to_string()))
        }
    }
}

fn create_tab_from_file_path(
    id: usize,
    name: String,
    path: &Path,
) -> Result<EditorTabState, OpenTabError> {
    let byte_doc = ByteDocument::from_file_path(path).map_err(OpenTabError::Read)?;
    create_tab_from_byte_document(id, name, byte_doc)
        .map_err(|error| OpenTabError::Decode(error.to_string()))
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn file_data_display_name(file: &FileData) -> String {
    use crate::domain::normalize_display_path;

    let path = normalize_display_path(&file.path().to_string_lossy());
    if path.is_empty() { file.name() } else { path }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn file_data_display_name(file: &FileData) -> String {
    file.name()
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn dropped_directory_files(
    file: &FileData,
) -> Result<Option<Vec<platform::NativeOpenFile>>, String> {
    let path = file.path();
    if path.is_dir() {
        platform::files_in_folder(&path).map(Some)
    } else {
        Ok(None)
    }
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn dropped_directory_files(
    _file: &FileData,
) -> Result<Option<Vec<platform::NativeOpenFile>>, String> {
    Ok(None)
}

#[cfg(not(target_arch = "wasm32"))]
fn file_data_real_path(file: &FileData) -> Option<PathBuf> {
    let path = file.path();
    if path.as_os_str().is_empty() || !path.is_file() {
        return None;
    }

    Some(path)
}

#[cfg(target_arch = "wasm32")]
fn file_data_real_path(_file: &FileData) -> Option<PathBuf> {
    None
}
