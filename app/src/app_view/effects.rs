use dioxus::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use rton_editor_core::encode_rton_bytes;
use rton_editor_core::{BinaryEncoding, EncodeOptions};
#[cfg(target_arch = "wasm32")]
use rton_editor_core::{TextFormat, WorkerDocumentSource, WorkerRtonSizeRequest};

#[cfg(target_arch = "wasm32")]
use crate::app_i18n::{bump_i18n_revision, initial_locale, load_i18n_sources_async};
#[cfg(not(target_arch = "wasm32"))]
use crate::domain::document_for_owned_tab;
use crate::domain::{EditorMode, EditorTabState};
#[cfg(target_arch = "wasm32")]
use crate::domain::{Status, Tone};
#[cfg(target_arch = "wasm32")]
use crate::i18n::{I18n, Locale};
#[cfg(not(target_arch = "wasm32"))]
use crate::platform::run_cpu_task;
#[cfg(target_arch = "wasm32")]
use crate::platform::run_rton_size_worker;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RtonOutputSize {
    pub(super) tab_id: usize,
    pub(super) compact: bool,
    pub(super) encrypted: bool,
    pub(super) byte_count: usize,
}

pub(super) fn use_rton_output_size_effect(
    active_tab: Option<EditorTabState>,
    compact: bool,
    encrypted: bool,
    mut output_size: Signal<Option<RtonOutputSize>>,
    mut output_size_generation: Signal<u64>,
) {
    use_effect(use_reactive(
        &(active_tab, compact, encrypted),
        move |(active_tab, compact, encrypted)| {
            let generation = output_size_generation.peek().saturating_add(1);
            output_size_generation.set(generation);

            let Some(tab) = active_tab else {
                output_size.set(None);
                return;
            };
            if tab.mode != EditorMode::RtonHex || !encrypted {
                output_size.set(None);
                return;
            }

            let tab_id = tab.id;
            output_size.set(None);
            spawn(async move {
                let encode_options = EncodeOptions {
                    encoding: if compact {
                        BinaryEncoding::Compact
                    } else {
                        BinaryEncoding::Standard
                    },
                    encrypted,
                };
                let Ok(byte_count) = rton_output_size_for_tab(tab, encode_options).await else {
                    return;
                };
                if *output_size_generation.peek() == generation {
                    output_size.set(Some(RtonOutputSize {
                        tab_id,
                        compact,
                        encrypted,
                        byte_count,
                    }));
                }
            });
        },
    ));
}

#[cfg(not(target_arch = "wasm32"))]
async fn rton_output_size_for_tab(
    tab: EditorTabState,
    encode_options: EncodeOptions,
) -> Result<usize, String> {
    let doc = run_cpu_task(move || document_for_owned_tab(tab).map_err(|error| error.to_string()))
        .await?;
    run_cpu_task(move || {
        encode_rton_bytes(&doc.value, encode_options)
            .map(|bytes| bytes.len())
            .map_err(|error| error.to_string())
    })
    .await
}

#[cfg(target_arch = "wasm32")]
async fn rton_output_size_for_tab(
    tab: EditorTabState,
    encode_options: EncodeOptions,
) -> Result<usize, String> {
    let response = run_rton_size_worker(WorkerRtonSizeRequest {
        source: worker_document_source(&tab)?,
        encode_options,
    })
    .await?;
    Ok(response.byte_len)
}

#[cfg(target_arch = "wasm32")]
fn worker_document_source(tab: &EditorTabState) -> Result<WorkerDocumentSource, String> {
    match tab.mode {
        EditorMode::RtonHex => {
            let bytes = tab
                .byte_doc
                .as_ref()
                .map(|byte_doc| byte_doc.as_cow().into_owned())
                .ok_or_else(|| "Missing RTON bytes".to_string())?;
            Ok(WorkerDocumentSource::RtonBytes(bytes))
        }
        EditorMode::Json | EditorMode::Yaml | EditorMode::Toml => {
            let format = match tab.mode {
                EditorMode::Json => TextFormat::Json,
                EditorMode::Yaml => TextFormat::Yaml,
                EditorMode::Toml => TextFormat::Toml,
                EditorMode::RtonHex => unreachable!("rton handled above"),
            };
            let text = tab
                .text_buffer
                .as_ref()
                .map(|buffer| buffer.text.to_string())
                .unwrap_or_else(|| tab.editor_text.to_string());
            Ok(WorkerDocumentSource::Text { text, format })
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub(super) fn use_web_i18n_loader(
    mut locale: Signal<Locale>,
    mut status: Signal<Status>,
    mut i18n_loaded: Signal<bool>,
    i18n_revision: Signal<u64>,
    initial_locale_snapshot: Locale,
) {
    use_future(move || async move {
        let loaded = load_i18n_sources_async().await;
        i18n_loaded.set(true);
        if loaded == 0 {
            return;
        }

        bump_i18n_revision(i18n_revision);
        if *locale.peek() == initial_locale_snapshot {
            let next_locale = initial_locale();
            locale.set(next_locale);
            status.set(Status::new(
                I18n::new(next_locale).t("status-ready"),
                Tone::Ok,
            ));
        }
    });
}
