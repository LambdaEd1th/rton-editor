use dioxus::prelude::*;

use crate::domain::{Status, ThemePreference, Tone};
use crate::i18n::{I18n, LanguageOption, Locale};
use crate::platform;

#[derive(Clone, PartialEq, Eq)]
struct ToolbarSelectOption {
    value: String,
    label: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct ToolbarSelectMenuPosition {
    left: i32,
    top: i32,
    min_width: i32,
}

#[component]
pub(super) fn PreferencesToolbarGroup(
    i18n: I18n,
    active: bool,
    theme_preference_snapshot: ThemePreference,
    locale_snapshot: Locale,
    language_options: Vec<LanguageOption>,
    line_wrapping_snapshot: bool,
    editor_search_panel_visible_snapshot: bool,
    mut theme_preference: Signal<ThemePreference>,
    mut locale: Signal<Locale>,
    mut line_wrapping: Signal<bool>,
    mut editor_search_panel_visible: Signal<bool>,
    mut status: Signal<Status>,
) -> Element {
    let theme_options = vec![
        ToolbarSelectOption {
            value: ThemePreference::System.code().to_string(),
            label: i18n.t("theme-system"),
        },
        ToolbarSelectOption {
            value: ThemePreference::Light.code().to_string(),
            label: i18n.t("theme-light"),
        },
        ToolbarSelectOption {
            value: ThemePreference::Dark.code().to_string(),
            label: i18n.t("theme-dark"),
        },
    ];
    let language_select_options = language_options
        .into_iter()
        .map(|language| ToolbarSelectOption {
            value: language.locale.code().to_string(),
            label: language.label,
        })
        .collect::<Vec<_>>();

    rsx! {
        div { class: "rton-toolbar-group",
            label { class: "rton-theme-label",
                span { {i18n.t("toolbar-theme")} }
                ToolbarInlineSelect {
                    value: theme_preference_snapshot.code().to_string(),
                    options: theme_options,
                    aria_label: i18n.t("toolbar-theme"),
                    on_change: move |value: String| {
                        let next_theme = ThemePreference::from_code(&value);
                        let _ = platform::save_theme_preference(next_theme);
                        theme_preference.set(next_theme);
                    }
                }
            }
            label { class: "rton-theme-label",
                span { {i18n.t("toolbar-language")} }
                ToolbarInlineSelect {
                    value: locale_snapshot.code().to_string(),
                    options: language_select_options,
                    aria_label: i18n.t("toolbar-language"),
                    on_change: move |value: String| {
                        let next_locale = Locale::from_code(&value);
                        let _ = platform::save_locale_preference(next_locale.code());
                        locale.set(next_locale);
                        status.set(Status::new(
                            I18n::new(next_locale).t("status-ready"),
                            Tone::Ok,
                        ));
                    }
                }
            }
            label { class: "rton-switch",
                input {
                    class: "rton-switch-input",
                    r#type: "checkbox",
                    checked: line_wrapping_snapshot,
                    onchange: move |event| {
                        let checked = event.checked();
                        let _ = platform::save_line_wrapping_preference(checked);
                        line_wrapping.set(checked);
                    }
                }
                span { class: "rton-switch-label", {i18n.t("toolbar-wrap")} }
                span { class: "rton-switch-track" }
            }
            label { class: "rton-switch",
                input {
                    class: "rton-switch-input",
                    r#type: "checkbox",
                    checked: editor_search_panel_visible_snapshot,
                    disabled: !active,
                    onchange: move |event| editor_search_panel_visible.set(event.checked())
                }
                span { class: "rton-switch-label", {i18n.t("toolbar-search")} }
                span { class: "rton-switch-track" }
            }
        }
    }
}

#[component]
fn ToolbarInlineSelect(
    value: String,
    options: Vec<ToolbarSelectOption>,
    aria_label: String,
    on_change: EventHandler<String>,
) -> Element {
    let mut control_mounted = use_signal(|| None::<MountedEvent>);
    let mut menu_position = use_signal(|| None::<ToolbarSelectMenuPosition>);
    let menu_position_snapshot = *menu_position.read();
    let open = menu_position_snapshot.is_some();
    let selected_label = options
        .iter()
        .find(|option| option.value == value)
        .map(|option| option.label.clone())
        .unwrap_or_else(|| value.clone());
    let control_class = if open {
        "rton-inline-select-control rton-inline-select-toolbar rton-theme-select is-open"
    } else {
        "rton-inline-select-control rton-inline-select-toolbar rton-theme-select"
    };
    let shell_class = if open {
        "rton-inline-select-shell is-open"
    } else {
        "rton-inline-select-shell"
    };

    rsx! {
        span {
            class: shell_class,
            button {
                r#type: "button",
                class: control_class,
                aria_label: "{aria_label}",
                aria_haspopup: "listbox",
                aria_expanded: open,
                onmounted: move |event| control_mounted.set(Some(event)),
                onmousedown: move |event| event.stop_propagation(),
                onclick: move |event| {
                    event.prevent_default();
                    event.stop_propagation();
                    if menu_position.peek().is_some() {
                        menu_position.set(None);
                    } else {
                        let mounted = control_mounted.peek().clone();
                        spawn(async move {
                            menu_position.set(Some(toolbar_select_menu_position(mounted).await));
                        });
                    }
                },
                onkeydown: move |event| {
                    event.stop_propagation();
                    let key = event.key().to_string();
                    match key.as_str() {
                        "Enter" | " " | "Space" | "ArrowDown" => {
                            event.prevent_default();
                            let mounted = control_mounted.peek().clone();
                            spawn(async move {
                                menu_position.set(Some(toolbar_select_menu_position(mounted).await));
                            });
                        }
                        "Escape" => {
                            event.prevent_default();
                            menu_position.set(None);
                        }
                        _ => {}
                    }
                },
                span {
                    class: "rton-inline-select-value",
                    title: "{selected_label}",
                    "{selected_label}"
                }
                span { class: "rton-inline-select-caret", aria_hidden: "true", "▾" }
            }
            if let Some(position) = menu_position_snapshot {
                div {
                    class: "rton-inline-select-backdrop",
                    onmousedown: move |event| {
                        event.stop_propagation();
                        menu_position.set(None);
                    }
                }
                div {
                    class: "rton-inline-select-menu rton-inline-select-menu-toolbar",
                    role: "listbox",
                    aria_label: "{aria_label}",
                    style: "left: {position.left}px; top: {position.top}px; min-width: {position.min_width}px",
                    onmousedown: move |event| event.stop_propagation(),
                    for option in options {
                        {
                            let active = option.value == value;
                            let option_value = option.value.clone();
                            rsx! {
                                button {
                                    key: "{option.value}",
                                    r#type: "button",
                                    class: if active { "active" } else { "" },
                                    role: "option",
                                    aria_selected: active,
                                    title: "{option.label}",
                                    onclick: move |event| {
                                        event.stop_propagation();
                                        menu_position.set(None);
                                        on_change.call(option_value.clone());
                                    },
                                    "{option.label}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

async fn toolbar_select_menu_position(mounted: Option<MountedEvent>) -> ToolbarSelectMenuPosition {
    let Some(event) = mounted else {
        return ToolbarSelectMenuPosition {
            left: 8,
            top: 8,
            min_width: 138,
        };
    };
    let Ok(rect) = event.get_client_rect().await else {
        return ToolbarSelectMenuPosition {
            left: 8,
            top: 8,
            min_width: 138,
        };
    };
    ToolbarSelectMenuPosition {
        left: rect.origin.x.round().max(8.0) as i32,
        top: (rect.origin.y + rect.height() + 6.0).round().max(8.0) as i32,
        min_width: rect.width().round().max(138.0) as i32,
    }
}
