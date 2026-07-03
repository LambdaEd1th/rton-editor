use dioxus::prelude::*;
use rton_editor_core::{DecodedDocument, EncodeOptions};
use std::collections::HashSet;
use std::sync::Arc;

use crate::components::FileListItem;
use crate::domain::{
    BatchExportMode, EditorTabState, OpenTabError, Status, Tone, ZipArchiveBuilder,
    batch_archive_name, batch_output_path, document_for_owned_tab, encode_batch_export_document,
    unique_zip_path,
};
use crate::file_import::{LoadedFileState, create_tab_from_loaded_file};
use crate::i18n::I18n;
use crate::platform::{self, run_cpu_task};

#[allow(clippy::too_many_arguments)]
pub(crate) async fn batch_export_selected_files(
    selected_items: Vec<FileListItem>,
    tabs: Vec<EditorTabState>,
    loaded_files: Vec<LoadedFileState>,
    mode: BatchExportMode,
    options: EncodeOptions,
    mut status: Signal<Status>,
    i18n: I18n,
) {
    if selected_items.is_empty() {
        status.set(Status::new(i18n.t("status-select-files-first"), Tone::Warn));
        return;
    }

    status.set(Status::new(
        i18n.t_args(
            "status-batch-converting",
            &[
                ("count", selected_items.len().to_string()),
                ("format", mode.label().to_string()),
            ],
        ),
        Tone::Warn,
    ));

    let total = selected_items.len();
    let mut used_paths = HashSet::new();
    let mut archive = ZipArchiveBuilder::new();
    let mut exported_count = 0usize;
    let mut errors = Vec::new();

    for (index, item) in selected_items.iter().enumerate() {
        match resolve_batch_export_document(item, &tabs, &loaded_files, i18n).await {
            Ok(doc) => {
                match run_cpu_task(move || encode_batch_export_document(&doc, mode, options)).await
                {
                    Ok(bytes) => {
                        let path =
                            unique_zip_path(&batch_output_path(&item.path, mode), &mut used_paths);
                        match archive.push_entry(&path, &bytes) {
                            Ok(()) => exported_count += 1,
                            Err(error) => errors.push(format!("{}: {error}", item.path)),
                        }
                    }
                    Err(error) => errors.push(format!("{}: {error}", item.path)),
                }
            }
            Err(error) => errors.push(format!("{}: {error}", item.path)),
        }

        if index % 24 == 23 {
            status.set(Status::new(
                i18n.t_args(
                    "status-batch-progress",
                    &[
                        ("format", mode.label().to_string()),
                        ("completed", (index + 1).to_string()),
                        ("total", total.to_string()),
                    ],
                ),
                Tone::Warn,
            ));
        }
    }

    if exported_count == 0 {
        status.set(Status::new(
            errors
                .first()
                .cloned()
                .unwrap_or_else(|| i18n.t("status-no-batch-success")),
            Tone::Error,
        ));
        return;
    }

    let zip_bytes = match archive.finish() {
        Ok(bytes) => bytes,
        Err(error) => {
            status.set(Status::new(error, Tone::Error));
            return;
        }
    };
    let archive_name = batch_archive_name(mode);
    match platform::save_bytes(&archive_name, &zip_bytes) {
        Ok(true) => {
            let suffix = if errors.is_empty() {
                String::new()
            } else {
                i18n.t_args(
                    "status-batch-failure-suffix",
                    &[("count", errors.len().to_string())],
                )
            };
            status.set(Status::new(
                i18n.t_args(
                    "status-batch-exported",
                    &[
                        ("count", exported_count.to_string()),
                        ("format", mode.label().to_string()),
                        ("suffix", suffix),
                    ],
                ),
                if errors.is_empty() {
                    Tone::Ok
                } else {
                    Tone::Warn
                },
            ));
        }
        Ok(false) => status.set(Status::new(i18n.t("status-export-cancelled"), Tone::Info)),
        Err(error) => status.set(Status::new(error, Tone::Error)),
    }
}

async fn resolve_batch_export_document(
    item: &FileListItem,
    tabs: &[EditorTabState],
    loaded_files: &[LoadedFileState],
    i18n: I18n,
) -> Result<Arc<DecodedDocument>, String> {
    if let Some(tab_id) = item.tab_id
        && let Some(tab) = tabs.iter().find(|tab| tab.id == tab_id).cloned()
    {
        return document_for_batch_tab(tab).await;
    }

    if let Some(file_id) = item.file_id {
        let file = loaded_files
            .iter()
            .find(|file| file.id == file_id)
            .ok_or_else(|| i18n.t("status-file-list-item-stale"))?;
        let tab = create_tab_from_loaded_file(0, file)
            .await
            .map_err(OpenTabError::message)?;
        return run_cpu_task(move || {
            document_for_owned_tab(tab).map_err(|error| error.to_string())
        })
        .await;
    }

    Err(i18n.t("status-no-export-value"))
}

async fn document_for_batch_tab(tab: EditorTabState) -> Result<Arc<DecodedDocument>, String> {
    run_cpu_task(move || document_for_owned_tab(tab).map_err(|error| error.to_string())).await
}
