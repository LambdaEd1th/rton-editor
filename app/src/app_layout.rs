use dioxus::prelude::*;
use dioxus_free_icons::icons::ld_icons::{LdActivity, LdFileArchive, LdFolderOpen};
use rton_editor_core::DecodedDocument;
use std::sync::Arc;

use crate::app_constants::LOADABLE_FILE_HINT;
use crate::components::{MetaItem, PanelHeader, StatsGrid, button_class, lucide_icon};
use crate::domain::{EditorMode, Status};
use crate::i18n::I18n;

#[component]
pub(crate) fn EmptyDropStage(i18n: I18n) -> Element {
    rsx! {
        div { class: "rton-empty-drop-stage",
            div { class: "empty-editor-icon", {lucide_icon(LdFolderOpen)} }
            div { class: "empty-editor-title", {i18n.t("drop-title")} }
            div {
                class: "empty-editor-subtitle",
                {i18n.t_args("drop-subtitle", &[("hint", LOADABLE_FILE_HINT.to_string())])}
            }
        }
    }
}

#[component]
pub(crate) fn FileSummaryPanel(
    i18n: I18n,
    active_file_label: String,
    input_value: String,
    output_value: String,
    active_mode: Option<EditorMode>,
    doc: Option<Arc<DecodedDocument>>,
    on_switch_mode: EventHandler<EditorMode>,
) -> Element {
    rsx! {
        div { class: "rton-inspector-summary",
            PanelHeader {
                icon: lucide_icon(LdFileArchive),
                title: i18n.t("panel-file-properties"),
                subtitle: i18n.t("panel-current-file"),
                div { class: "batch-export-grid",
                    for mode in [EditorMode::RtonHex, EditorMode::Json, EditorMode::Yaml, EditorMode::Toml] {
                        button {
                            class: if active_mode == Some(mode) { button_class("primary") } else { button_class("secondary") },
                            disabled: active_mode.is_none(),
                            aria_pressed: active_mode == Some(mode),
                            onclick: move |_| on_switch_mode.call(mode),
                            "{mode.label()}"
                        }
                    }
                }
            }
            section { class: "file-summary-meta-section",
                dl { class: "meta-list",
                    MetaItem {
                        label: i18n.t("panel-name"),
                        value: active_file_label
                    }
                    MetaItem {
                        label: i18n.t("panel-input"),
                        value: input_value
                    }
                    MetaItem {
                        label: i18n.t("panel-output"),
                        value: output_value
                    }
                }
            }
            if let Some(doc) = doc.as_ref() {
                section { class: "file-summary-stats-section",
                    div { class: "stats-title",
                        span { class: "panel-header-icon muted", {lucide_icon(LdActivity)} }
                        h2 { {i18n.t("panel-stats")} }
                    }
                    StatsGrid { stats: doc.stats.clone(), i18n }
                }
            }
        }
    }
}

#[component]
pub(crate) fn StatusBar(
    i18n: I18n,
    active_file_label: String,
    output_value: String,
    status: Status,
) -> Element {
    rsx! {
        footer { class: "status-bar {status.tone.class()}",
            span { class: "status-file",
                "{active_file_label}"
            }
            span { class: "status-message", "{status.message}" }
            span { class: "status-output-label", {i18n.t("panel-output")} }
            span { class: "status-output-value",
                "{output_value}"
            }
        }
    }
}
