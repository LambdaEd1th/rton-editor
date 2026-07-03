use dioxus::prelude::*;

use rton_editor_core::TextFormat;

use crate::app_sample::SAMPLE_JSON;
use crate::components::FileSelection;
use crate::domain::{EditorTabState, OpenTabError, Status, Tone, create_text_tab};
use crate::file_import::{
    LoadedFileState, create_tab_from_loaded_file, loaded_file_draft_from_native,
    stage_loaded_file_drafts,
};
use crate::i18n::I18n;
use crate::platform;

use super::document::parse_tab_by_id;

pub(crate) fn load_sample_tab(
    mut next_tab_id: Signal<usize>,
    mut tabs: Signal<Vec<EditorTabState>>,
    mut active_tab_id: Signal<usize>,
    mut status: Signal<Status>,
    i18n: I18n,
) {
    let id = *next_tab_id.read();
    next_tab_id.set(id + 1);
    match create_text_tab(
        id,
        format!("sample-{id}.json"),
        SAMPLE_JSON.to_string(),
        TextFormat::Json,
    ) {
        Ok(tab) => {
            let name = tab.file_name.clone();
            tabs.write().push(tab);
            active_tab_id.set(id);
            status.set(Status::new(
                i18n.t_args("status-sample-opened", &[("name", name)]),
                Tone::Ok,
            ));
            parse_tab_by_id(id, tabs, status, i18n);
        }
        Err(error) => status.set(Status::new(error.to_string(), Tone::Error)),
    }
}

pub(crate) fn activate_tab_by_id(
    id: usize,
    tabs: Signal<Vec<EditorTabState>>,
    mut active_tab_id: Signal<usize>,
    mut status: Signal<Status>,
    i18n: I18n,
) {
    if tabs.read().iter().any(|tab| tab.id == id) {
        active_tab_id.set(id);
        status.set(Status::new(i18n.t("status-tab-activated"), Tone::Info));
    }
}

pub(crate) fn open_loaded_file_by_id(
    file_id: usize,
    loaded_files: Signal<Vec<LoadedFileState>>,
    mut next_tab_id: Signal<usize>,
    mut tabs: Signal<Vec<EditorTabState>>,
    mut active_tab_id: Signal<usize>,
    mut status: Signal<Status>,
    i18n: I18n,
) {
    spawn(async move {
        let Some(entry) = loaded_files
            .read()
            .iter()
            .find(|file| file.id == file_id)
            .cloned()
        else {
            return;
        };

        if let Some(tab_id) = entry.tab_id
            && tabs.read().iter().any(|tab| tab.id == tab_id)
        {
            active_tab_id.set(tab_id);
            status.set(Status::new(i18n.t("status-tab-activated"), Tone::Info));
            return;
        }

        let id = *next_tab_id.read();
        next_tab_id.set(id + 1);
        let name = entry.display_name.clone();
        match create_tab_from_loaded_file(id, &entry).await {
            Ok(tab) => {
                tabs.write().push(tab);
                crate::file_import::set_loaded_file_tab_id(loaded_files, file_id, Some(id));
                active_tab_id.set(id);
                status.set(Status::new(
                    i18n.t_args("status-opened-file", &[("name", name)]),
                    Tone::Ok,
                ));
                parse_tab_by_id(id, tabs, status, i18n);
            }
            Err(OpenTabError::Read(error)) => status.set(Status::new(
                i18n.t_args(
                    "status-file-read-error",
                    &[("name", name), ("error", error)],
                ),
                Tone::Error,
            )),
            Err(error) => status.set(Status::new(
                i18n.t_args(
                    "status-file-error",
                    &[("name", name), ("error", error.message())],
                ),
                Tone::Error,
            )),
        }
    });
}

pub(crate) fn open_native_files_dialog(
    loaded_files: Signal<Vec<LoadedFileState>>,
    next_loaded_file_id: Signal<usize>,
    file_selection: Signal<FileSelection>,
    mut status: Signal<Status>,
    i18n: I18n,
) {
    match platform::open_files() {
        Ok(Some(files)) => stage_native_open_files(
            files,
            loaded_files,
            next_loaded_file_id,
            file_selection,
            status,
            i18n,
        ),
        Ok(None) => {}
        Err(error) => status.set(Status::new(error, Tone::Error)),
    }
}

pub(crate) fn open_native_folder_dialog(
    loaded_files: Signal<Vec<LoadedFileState>>,
    next_loaded_file_id: Signal<usize>,
    file_selection: Signal<FileSelection>,
    mut status: Signal<Status>,
    i18n: I18n,
) {
    match platform::open_folder() {
        Ok(Some(files)) => stage_native_open_files(
            files,
            loaded_files,
            next_loaded_file_id,
            file_selection,
            status,
            i18n,
        ),
        Ok(None) => {}
        Err(error) => status.set(Status::new(error, Tone::Error)),
    }
}

fn stage_native_open_files(
    files: Vec<platform::NativeOpenFile>,
    loaded_files: Signal<Vec<LoadedFileState>>,
    next_loaded_file_id: Signal<usize>,
    file_selection: Signal<FileSelection>,
    mut status: Signal<Status>,
    i18n: I18n,
) {
    if files.is_empty() {
        status.set(Status::new(i18n.t("status-no-loadable-files"), Tone::Warn));
        return;
    }

    let drafts = files
        .into_iter()
        .map(loaded_file_draft_from_native)
        .collect();
    let indexed =
        stage_loaded_file_drafts(loaded_files, next_loaded_file_id, file_selection, drafts);
    status.set(Status::new(
        i18n.t_args("status-indexed-files", &[("count", indexed.to_string())]),
        Tone::Ok,
    ));
}
