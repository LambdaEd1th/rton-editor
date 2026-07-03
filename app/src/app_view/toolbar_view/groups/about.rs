use dioxus::prelude::*;
use dioxus_free_icons::icons::ld_icons::{LdGithub, LdInfo, LdX};

use crate::components::{button_class, lucide_icon};
use crate::i18n::I18n;

const AUTHOR_NAME: &str = "LambdaEd1th";
const AUTHOR_URL: &str = "https://space.bilibili.com/8217621";
const GITHUB_URL: &str = "https://github.com/LambdaEd1th/rton-editor-legacy";

#[component]
pub(super) fn AboutToolbarGroup(i18n: I18n) -> Element {
    let mut open = use_signal(|| false);
    let title = i18n.t("about-title");
    let close_label = i18n.t("about-close");
    let github_label = i18n.t("about-github");

    rsx! {
        div { class: "rton-toolbar-group",
            button {
                class: button_class("secondary"),
                title: i18n.t("toolbar-about"),
                onclick: move |_| open.set(true),
                span { class: "button-icon", {lucide_icon(LdInfo)} }
                {i18n.t("toolbar-about")}
            }
            if *open.read() {
                div {
                    class: "rton-about-backdrop",
                    onclick: move |_| open.set(false),
                    div {
                        class: "rton-about-dialog",
                        role: "dialog",
                        aria_modal: "true",
                        aria_label: "{title}",
                        onclick: move |event| event.stop_propagation(),
                        header { class: "rton-about-header",
                            div { class: "rton-about-title-row",
                                span { class: "rton-about-icon", {lucide_icon(LdInfo)} }
                                h2 { "{title}" }
                            }
                            button {
                                r#type: "button",
                                class: "rton-about-close",
                                title: "{close_label}",
                                aria_label: "{close_label}",
                                onclick: move |_| open.set(false),
                                {lucide_icon(LdX)}
                            }
                        }
                        dl { class: "rton-about-meta-list",
                            AboutMetaItem {
                                label: i18n.t("about-version"),
                                value: env!("CARGO_PKG_VERSION").to_string()
                            }
                            AboutMetaItem {
                                label: i18n.t("about-license"),
                                value: env!("CARGO_PKG_LICENSE").to_string()
                            }
                            div { class: "rton-about-meta-item",
                                dt { {i18n.t("about-author")} }
                                dd {
                                    a {
                                        class: "rton-about-link",
                                        href: AUTHOR_URL,
                                        target: "_blank",
                                        rel: "noopener noreferrer",
                                        "{AUTHOR_NAME}"
                                    }
                                }
                            }
                        }
                        div { class: "rton-about-actions",
                            a {
                                class: "rton-about-github-link",
                                href: GITHUB_URL,
                                target: "_blank",
                                rel: "noopener noreferrer",
                                title: "{github_label}",
                                aria_label: "{github_label}",
                                {lucide_icon(LdGithub)}
                                span { "{github_label}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn AboutMetaItem(label: String, value: String) -> Element {
    rsx! {
        div { class: "rton-about-meta-item",
            dt { "{label}" }
            dd { "{value}" }
        }
    }
}
