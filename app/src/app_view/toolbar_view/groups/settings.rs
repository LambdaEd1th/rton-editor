use dioxus::prelude::*;
use dioxus_free_icons::icons::ld_icons::{
    LdCheck, LdChevronDown, LdGithub, LdInfo, LdLanguages, LdMonitor, LdMoon, LdSettings, LdSun,
    LdX,
};

use crate::components::lucide_icon;
use crate::domain::{Status, ThemePreference, Tone};
use crate::i18n::{I18n, LanguageOption, Locale};
use crate::platform;

const AUTHOR_NAME: &str = "LambdaEd1th";
const AUTHOR_URL: &str = "https://space.bilibili.com/8217621";
const GITHUB_URL: &str = "https://github.com/LambdaEd1th/rton-editor";

const THEME_OPTIONS: [ThemePreference; 3] = [
    ThemePreference::System,
    ThemePreference::Light,
    ThemePreference::Dark,
];

fn theme_label_key(theme: ThemePreference) -> &'static str {
    match theme {
        ThemePreference::System => "theme-system",
        ThemePreference::Light => "theme-light",
        ThemePreference::Dark => "theme-dark",
    }
}

fn theme_icon(theme: ThemePreference) -> Element {
    match theme {
        ThemePreference::System => lucide_icon(LdMonitor),
        ThemePreference::Light => lucide_icon(LdSun),
        ThemePreference::Dark => lucide_icon(LdMoon),
    }
}

#[component]
pub(crate) fn SettingsDialog(
    i18n: I18n,
    closing: bool,
    theme_preference_snapshot: ThemePreference,
    locale_snapshot: Locale,
    language_options: Vec<LanguageOption>,
    mut theme_preference: Signal<ThemePreference>,
    mut locale: Signal<Locale>,
    mut status: Signal<Status>,
    on_close: EventHandler<()>,
) -> Element {
    let mut language_open = use_signal(|| false);
    let language_open_snapshot = *language_open.read();
    let title = i18n.t("settings-title");
    let close_label = i18n.t("about-close");
    let current_language_label = language_options
        .iter()
        .find(|option| option.locale == locale_snapshot)
        .map(|option| option.label.clone())
        .unwrap_or_else(|| locale_snapshot.code().to_string());

    rsx! {
        div {
            class: if closing { "rton-settings-backdrop closing" } else { "rton-settings-backdrop" },
            tabindex: "-1",
            onmounted: move |event| {
                spawn(async move {
                    let _ = event.set_focus(true).await;
                });
            },
            onkeydown: move |event| {
                if event.key() == Key::Escape {
                    event.prevent_default();
                    on_close.call(());
                }
            },
            onclick: move |_| on_close.call(()),
            div {
                class: "rton-settings-dialog",
                role: "dialog",
                aria_modal: "true",
                aria_label: "{title}",
                onclick: move |event| event.stop_propagation(),
                header { class: "rton-settings-header",
                    div { class: "rton-settings-title-row",
                        span { class: "rton-settings-title-icon", {lucide_icon(LdSettings)} }
                        h2 { "{title}" }
                    }
                    button {
                        r#type: "button",
                        class: "rton-settings-close",
                        title: "{close_label}",
                        aria_label: "{close_label}",
                        onclick: move |_| on_close.call(()),
                        {lucide_icon(LdX)}
                    }
                }

                div { class: "rton-settings-content",
                    section { class: "rton-settings-section",
                        div { class: "rton-settings-section-heading",
                            span { class: "rton-settings-section-icon", {lucide_icon(LdMonitor)} }
                            span { {i18n.t("settings-appearance")} }
                        }
                        div { class: "rton-settings-theme-segments",
                            for theme in THEME_OPTIONS {
                                button {
                                    key: "{theme.code()}",
                                    r#type: "button",
                                    class: if theme == theme_preference_snapshot { "active" } else { "" },
                                    aria_pressed: theme == theme_preference_snapshot,
                                    onclick: move |_| {
                                        let _ = platform::save_theme_preference(theme);
                                        theme_preference.set(theme);
                                    },
                                    span { class: "rton-settings-theme-icon", {theme_icon(theme)} }
                                    span { {i18n.t(theme_label_key(theme))} }
                                }
                            }
                        }
                    }

                    section { class: "rton-settings-section",
                        div { class: "rton-settings-section-heading",
                            span { class: "rton-settings-section-icon", {lucide_icon(LdLanguages)} }
                            span { {i18n.t("settings-language")} }
                        }
                        div { class: if language_open_snapshot { "rton-settings-language open" } else { "rton-settings-language" },
                            button {
                                r#type: "button",
                                class: "rton-settings-language-control",
                                aria_label: i18n.t("settings-language"),
                                aria_haspopup: "listbox",
                                aria_expanded: language_open_snapshot,
                                onclick: move |_| language_open.set(!language_open_snapshot),
                                span { "{current_language_label}" }
                                span { class: "rton-settings-language-caret", {lucide_icon(LdChevronDown)} }
                            }
                            div {
                                class: "rton-settings-language-menu",
                                role: "listbox",
                                aria_label: i18n.t("settings-language"),
                                aria_hidden: !language_open_snapshot,
                                for option in language_options {
                                    {
                                        let option_locale = option.locale;
                                        let active = option_locale == locale_snapshot;
                                        rsx! {
                                            button {
                                                key: "{option_locale.code()}",
                                                r#type: "button",
                                                class: if active { "active" } else { "" },
                                                role: "option",
                                                tabindex: if language_open_snapshot { "0" } else { "-1" },
                                                aria_selected: active,
                                                onclick: move |_| {
                                                    language_open.set(false);
                                                    let _ = platform::save_locale_preference(option_locale.code());
                                                    locale.set(option_locale);
                                                    status.set(Status::new(
                                                        I18n::new(option_locale).t("status-ready"),
                                                        Tone::Ok,
                                                    ));
                                                },
                                                span { "{option.label}" }
                                                if active {
                                                    span { class: "rton-settings-language-check", {lucide_icon(LdCheck)} }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    section { class: "rton-settings-section rton-settings-about-section",
                        div { class: "rton-settings-section-heading",
                            span { class: "rton-settings-section-icon", {lucide_icon(LdInfo)} }
                            span { {i18n.t("settings-about")} }
                        }
                        dl { class: "rton-settings-about-list",
                            SettingsAboutItem {
                                label: i18n.t("about-version"),
                                value: env!("CARGO_PKG_VERSION").to_string()
                            }
                            SettingsAboutItem {
                                label: i18n.t("about-license"),
                                value: env!("CARGO_PKG_LICENSE").to_string()
                            }
                            div { class: "rton-settings-about-item",
                                dt { {i18n.t("about-author")} }
                                dd {
                                    a {
                                        href: AUTHOR_URL,
                                        target: "_blank",
                                        rel: "noopener noreferrer",
                                        "{AUTHOR_NAME}"
                                    }
                                }
                            }
                        }
                        a {
                            class: "rton-settings-github-link",
                            href: GITHUB_URL,
                            target: "_blank",
                            rel: "noopener noreferrer",
                            title: i18n.t("about-github"),
                            {lucide_icon(LdGithub)}
                            span { {i18n.t("about-github")} }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn SettingsAboutItem(label: String, value: String) -> Element {
    rsx! {
        div { class: "rton-settings-about-item",
            dt { "{label}" }
            dd { "{value}" }
        }
    }
}
