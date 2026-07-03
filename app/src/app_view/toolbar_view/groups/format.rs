use dioxus::prelude::*;

use crate::components::mode_button_class;
use crate::domain::EditorMode;

#[component]
pub(super) fn FormatToolbarGroup(
    active_mode_snapshot: Option<EditorMode>,
    switch_mode: EventHandler<EditorMode>,
) -> Element {
    rsx! {
        div { class: "rton-toolbar-group",
            for mode in [EditorMode::RtonHex, EditorMode::Json, EditorMode::Yaml, EditorMode::Toml] {
                button {
                    class: mode_button_class(active_mode_snapshot == Some(mode)),
                    disabled: active_mode_snapshot.is_none(),
                    onclick: move |_| switch_mode.call(mode),
                    "{mode.label()}"
                }
            }
        }
    }
}
