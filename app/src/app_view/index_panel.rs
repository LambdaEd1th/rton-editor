use dioxus::prelude::*;
use dioxus_free_icons::icons::ld_icons::{LdListTree, LdSearch};
use rton_editor_core::{DecodedDocument, TreeRows, ValueSearchResult};
use std::collections::HashSet;
use std::sync::Arc;

use crate::app_layout::FileSummaryPanel;
use crate::components::{ValueSearchResults, ValueTree, lucide_icon};
use crate::i18n::I18n;

#[component]
pub(super) fn IndexPanel(
    i18n: I18n,
    active_file_label: String,
    input_value: String,
    output_value: String,
    active_doc: Option<Arc<DecodedDocument>>,
    active_search: String,
    selected_path: String,
    tree_rows: Arc<TreeRows>,
    expanded_paths: Arc<HashSet<String>>,
    value_search_result: Option<Arc<ValueSearchResult>>,
    on_search_change: EventHandler<String>,
    on_select_path: EventHandler<String>,
    on_toggle_path: EventHandler<String>,
    suppress_resize_observer: bool,
) -> Element {
    rsx! {
        aside { class: "rton-side-panel rton-side-panel-right",
            FileSummaryPanel {
                i18n,
                active_file_label,
                input_value,
                output_value,
                doc: active_doc.clone()
            }

            div { class: "rton-index-panel",
                div { class: "rton-index-header",
                    div { class: "rton-index-title",
                        div { class: "rton-index-title-line",
                            span { class: "panel-header-icon", {lucide_icon(LdListTree)} }
                            h2 { {i18n.t("panel-inspector")} }
                        }
                        p { "RtonValue" }
                    }
                    div { class: "rton-index-search",
                        span { class: "search-icon", {lucide_icon(LdSearch)} }
                        input {
                            r#type: "search",
                            placeholder: i18n.t("inspector-search-placeholder"),
                            value: "{active_search}",
                            disabled: active_doc.is_none(),
                            oninput: move |event| on_search_change.call(event.value()),
                            onkeydown: move |event| {
                                if event.key().to_string() == "Escape" {
                                    event.prevent_default();
                                    on_search_change.call(String::new());
                                }
                            }
                        }
                        if !active_search.is_empty() {
                            button {
                                r#type: "button",
                                class: "rton-index-search-clear",
                                aria_label: i18n.t("inspector-clear-search"),
                                title: i18n.t("inspector-clear-search"),
                                onclick: move |_| on_search_change.call(String::new()),
                                "×"
                            }
                        }
                    }
                }
                if let Some(search_result) = value_search_result {
                    ValueSearchResults {
                        result: search_result,
                        i18n,
                        on_select: on_select_path,
                        suppress_resize_observer
                    }
                } else if active_doc.is_some() {
                    ValueTree {
                        rows: tree_rows,
                        expanded_paths,
                        selected_path,
                        i18n,
                        on_select: on_select_path,
                        on_toggle: on_toggle_path,
                        suppress_resize_observer
                    }
                } else {
                    div { class: "empty-state",
                        strong { {i18n.t("inspector-empty-title")} }
                        p { {i18n.t("inspector-empty-body")} }
                    }
                }
            }
        }
    }
}
