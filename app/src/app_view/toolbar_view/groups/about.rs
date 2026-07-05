use dioxus::prelude::*;
use dioxus_free_icons::icons::ld_icons::{LdGithub, LdInfo, LdX};

use crate::components::{button_class, lucide_icon};
use crate::i18n::I18n;

const AUTHOR_NAME: &str = "LambdaEd1th";
const AUTHOR_URL: &str = "https://space.bilibili.com/8217621";
const GITHUB_URL: &str = "https://github.com/LambdaEd1th/rton-editor";

#[derive(Clone, Copy, PartialEq)]
struct AboutDialogOffset {
    x: f64,
    y: f64,
}

#[derive(Clone, Copy, PartialEq)]
struct AboutDialogDrag {
    start_x: f64,
    start_y: f64,
    start_offset: AboutDialogOffset,
}

fn about_dialog_style(offset: AboutDialogOffset) -> String {
    if offset.x == 0.0 && offset.y == 0.0 {
        String::new()
    } else {
        format!("transform: translate({}px, {}px)", offset.x, offset.y)
    }
}

#[component]
pub(super) fn AboutToolbarGroup(i18n: I18n) -> Element {
    let mut open = use_signal(|| false);
    let mut dialog_offset = use_signal(|| AboutDialogOffset { x: 0.0, y: 0.0 });
    let mut dialog_drag = use_signal(|| None::<AboutDialogDrag>);
    let mut suppress_backdrop_click = use_signal(|| false);
    let title = i18n.t("about-title");
    let close_label = i18n.t("about-close");
    let github_label = i18n.t("about-github");
    let dialog_style = about_dialog_style(*dialog_offset.read());
    let dialog_class = if dialog_drag.read().is_some() {
        "rton-about-dialog dragging"
    } else {
        "rton-about-dialog"
    };

    rsx! {
        div { class: "rton-toolbar-group",
            button {
                class: button_class("secondary"),
                title: i18n.t("toolbar-about"),
                onclick: move |_| {
                    dialog_offset.set(AboutDialogOffset { x: 0.0, y: 0.0 });
                    dialog_drag.set(None);
                    suppress_backdrop_click.set(false);
                    open.set(true);
                },
                span { class: "button-icon", {lucide_icon(LdInfo)} }
                {i18n.t("toolbar-about")}
            }
            if *open.read() {
                div {
                    class: "rton-about-backdrop",
                    onclick: move |_| {
                        if *suppress_backdrop_click.read() {
                            suppress_backdrop_click.set(false);
                        } else {
                            dialog_offset.set(AboutDialogOffset { x: 0.0, y: 0.0 });
                            dialog_drag.set(None);
                            open.set(false);
                        }
                    },
                    onmousemove: move |event| {
                        let Some(drag) = *dialog_drag.read() else {
                            return;
                        };
                        event.prevent_default();
                        let coordinates = event.client_coordinates();
                        dialog_offset.set(AboutDialogOffset {
                            x: drag.start_offset.x + coordinates.x - drag.start_x,
                            y: drag.start_offset.y + coordinates.y - drag.start_y,
                        });
                        suppress_backdrop_click.set(true);
                    },
                    onmouseup: move |_| dialog_drag.set(None),
                    onmouseleave: move |_| dialog_drag.set(None),
                    div {
                        class: "{dialog_class}",
                        style: "{dialog_style}",
                        role: "dialog",
                        aria_modal: "true",
                        aria_label: "{title}",
                        onclick: move |event| {
                            suppress_backdrop_click.set(false);
                            event.stop_propagation();
                        },
                        header {
                            class: "rton-about-header",
                            onmousedown: move |event| {
                                event.prevent_default();
                                event.stop_propagation();
                                let client_coordinates = event.client_coordinates();
                                dialog_drag.set(Some(AboutDialogDrag {
                                    start_x: client_coordinates.x,
                                    start_y: client_coordinates.y,
                                    start_offset: *dialog_offset.peek(),
                                }));
                                suppress_backdrop_click.set(true);
                            },
                            div { class: "rton-about-title-row",
                                span { class: "rton-about-icon", {lucide_icon(LdInfo)} }
                                h2 { "{title}" }
                            }
                            button {
                                r#type: "button",
                                class: "rton-about-close",
                                title: "{close_label}",
                                aria_label: "{close_label}",
                                onmousedown: move |event| event.stop_propagation(),
                                onclick: move |_| {
                                    dialog_offset.set(AboutDialogOffset { x: 0.0, y: 0.0 });
                                    dialog_drag.set(None);
                                    open.set(false);
                                },
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
