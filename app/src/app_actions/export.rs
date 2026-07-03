use dioxus::prelude::*;
use rton_editor_core::{
    BinaryEncoding, EncodeOptions, TextFormat, encode_rton_bytes, format_bytes, value_to_text,
};

use crate::domain::{
    EditorTabState, Status, Tone, document_for_owned_tab, export_rton_name, export_text_name,
};
use crate::i18n::I18n;
use crate::platform::{self, run_cpu_task};

use super::document::cache_parsed_document;
use super::tabs::active_tab;

pub(crate) fn export_active_rton(
    tabs: Signal<Vec<EditorTabState>>,
    active_tab_id: Signal<usize>,
    compact_output: Signal<bool>,
    encrypt_output: Signal<bool>,
    mut status: Signal<Status>,
    i18n: I18n,
) {
    let Some(active_tab) = active_tab(tabs, active_tab_id) else {
        status.set(Status::new(i18n.t("status-no-document-export"), Tone::Warn));
        return;
    };
    let tab_id = active_tab.id;
    let file_name = active_tab.file_name.clone();
    let selected_path = active_tab.selected_path.clone();
    let options = EncodeOptions {
        encoding: if *compact_output.read() {
            BinaryEncoding::Compact
        } else {
            BinaryEncoding::Standard
        },
        encrypted: *encrypt_output.read(),
    };

    spawn(async move {
        let doc = match run_cpu_task(move || {
            document_for_owned_tab(active_tab).map_err(|error| error.to_string())
        })
        .await
        {
            Ok(doc) => doc,
            Err(error) => {
                status.set(Status::new(error, Tone::Error));
                return;
            }
        };
        match run_cpu_task({
            let doc = doc.clone();
            move || encode_rton_bytes(&doc.value, options).map_err(|error| error.to_string())
        })
        .await
        {
            Ok(bytes) => {
                let default_name = export_rton_name(&file_name);
                match platform::save_bytes(&default_name, &bytes) {
                    Ok(true) => {
                        cache_parsed_document(tabs, tab_id, selected_path, doc, false);
                        status.set(Status::new(
                            i18n.t_args(
                                "status-exported-file-bytes",
                                &[("name", default_name), ("bytes", format_bytes(bytes.len()))],
                            ),
                            Tone::Ok,
                        ));
                    }
                    Ok(false) => {
                        status.set(Status::new(i18n.t("status-export-cancelled"), Tone::Info))
                    }
                    Err(error) => status.set(Status::new(error, Tone::Error)),
                }
            }
            Err(error) => status.set(Status::new(error, Tone::Error)),
        }
    });
}

pub(crate) fn export_active_text(
    format: TextFormat,
    tabs: Signal<Vec<EditorTabState>>,
    active_tab_id: Signal<usize>,
    mut status: Signal<Status>,
    i18n: I18n,
) {
    let Some(active_tab) = active_tab(tabs, active_tab_id) else {
        status.set(Status::new(i18n.t("status-no-document-export"), Tone::Warn));
        return;
    };
    let tab_id = active_tab.id;
    let file_name = active_tab.file_name.clone();
    let selected_path = active_tab.selected_path.clone();

    spawn(async move {
        let doc = match run_cpu_task(move || {
            document_for_owned_tab(active_tab).map_err(|error| error.to_string())
        })
        .await
        {
            Ok(doc) => doc,
            Err(error) => {
                status.set(Status::new(error, Tone::Error));
                return;
            }
        };
        match run_cpu_task({
            let doc = doc.clone();
            move || value_to_text(&doc.value, format).map_err(|error| error.to_string())
        })
        .await
        {
            Ok(text) => {
                let default_name = export_text_name(&file_name, format);
                match platform::save_text(&default_name, &text) {
                    Ok(true) => {
                        cache_parsed_document(tabs, tab_id, selected_path, doc, false);
                        status.set(Status::new(
                            i18n.t_args("status-exported-file", &[("name", default_name)]),
                            Tone::Ok,
                        ));
                    }
                    Ok(false) => {
                        status.set(Status::new(i18n.t("status-export-cancelled"), Tone::Info))
                    }
                    Err(error) => status.set(Status::new(error, Tone::Error)),
                }
            }
            Err(error) => status.set(Status::new(error, Tone::Error)),
        }
    });
}
