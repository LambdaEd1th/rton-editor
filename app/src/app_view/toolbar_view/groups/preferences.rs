use dioxus::prelude::*;

use crate::domain::{Status, ThemePreference, Tone};
use crate::i18n::{I18n, LanguageOption, Locale};
use crate::platform;

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
    rsx! {
        div { class: "rton-toolbar-group",
            label { class: "rton-theme-label",
                span { {i18n.t("toolbar-theme")} }
                select {
                    class: "rton-theme-select",
                    value: theme_preference_snapshot.code(),
                    onchange: move |event| theme_preference.set(ThemePreference::from_code(&event.value())),
                    option { value: "system", selected: theme_preference_snapshot == ThemePreference::System, {i18n.t("theme-system")} }
                    option { value: "light", selected: theme_preference_snapshot == ThemePreference::Light, {i18n.t("theme-light")} }
                    option { value: "dark", selected: theme_preference_snapshot == ThemePreference::Dark, {i18n.t("theme-dark")} }
                }
            }
            label { class: "rton-theme-label",
                span { {i18n.t("toolbar-language")} }
                select {
                    class: "rton-theme-select",
                    value: "{locale_snapshot.code()}",
                    onchange: move |event| {
                        let next_locale = Locale::from_code(&event.value());
                        let _ = platform::save_locale_preference(next_locale.code());
                        locale.set(next_locale);
                        status.set(Status::new(
                            I18n::new(next_locale).t("status-ready"),
                            Tone::Ok,
                        ));
                    },
                    for language in language_options.clone() {
                        option {
                            value: "{language.locale.code()}",
                            selected: locale_snapshot == language.locale,
                            {language.label}
                        }
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
