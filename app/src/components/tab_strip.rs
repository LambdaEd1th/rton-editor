use dioxus::prelude::*;

use crate::app_constants::TAB_DROP_MIDPOINT_PX;
use crate::domain::{DropMarker, DropPlacement, EditorMode, leaf_display_name};
use crate::i18n::I18n;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TabHeader {
    pub(crate) id: usize,
    pub(crate) file_name: String,
    pub(crate) mode: EditorMode,
    pub(crate) dirty: bool,
    pub(crate) closeable: bool,
}

fn file_tab_class(active: bool, dragging: bool, drop_placement: Option<DropPlacement>) -> String {
    let mut class_name = if active {
        "rton-file-tab active".to_string()
    } else {
        "rton-file-tab".to_string()
    };
    if dragging {
        class_name.push_str(" dragging");
    }
    if let Some(drop_placement) = drop_placement {
        class_name.push(' ');
        class_name.push_str(drop_placement.class());
    }
    class_name
}

fn scroll_active_tab_into_view(active_tab_id: usize) {
    document::eval(&format!(
        r#"
        (() => {{
            const container = document.querySelector('.rton-file-tabs');
            const tab = document.querySelector('.rton-file-tab[data-rton-tab-id="{active_tab_id}"]');
            if (!container || !tab) return;

            const tabLeft = tab.offsetLeft;
            const tabRight = tabLeft + tab.offsetWidth;
            const visibleLeft = container.scrollLeft;
            const visibleRight = visibleLeft + container.clientWidth;
            const padding = 8;

            if (tabLeft < visibleLeft + padding) {{
                container.scrollTo({{
                    left: Math.max(0, tabLeft - padding),
                    behavior: 'smooth',
                }});
            }} else if (tabRight > visibleRight - padding) {{
                container.scrollTo({{
                    left: tabRight - container.clientWidth + padding,
                    behavior: 'smooth',
                }});
            }}
        }})();
        "#
    ));
}

#[component]
pub(crate) fn TabStrip(
    tabs: Vec<TabHeader>,
    active_tab_id: usize,
    dragged_tab_id: Option<usize>,
    drop_marker: Option<DropMarker<usize>>,
    i18n: I18n,
    on_activate: EventHandler<usize>,
    on_close: EventHandler<usize>,
    on_drag_start: EventHandler<usize>,
    on_drop_marker: EventHandler<DropMarker<usize>>,
    on_drag_end: EventHandler<()>,
) -> Element {
    let tab_count = tabs.len();
    use_effect(use_reactive(&active_tab_id, move |active_tab_id| {
        scroll_active_tab_into_view(active_tab_id);
    }));

    rsx! {
        nav { class: "rton-tab-strip",
            div {
                class: "rton-file-tabs",
                role: "tablist",
                aria_label: i18n.t("tabs-open-files"),
                for tab in tabs {
                    div {
                        "data-rton-tab-id": "{tab.id}",
                        class: file_tab_class(
                            tab.id == active_tab_id,
                            dragged_tab_id == Some(tab.id),
                            drop_marker.filter(|marker| marker.id == tab.id).map(|marker| marker.placement),
                        ),
                        onmousedown: move |_| {
                            if tab_count > 1 {
                                on_drag_start.call(tab.id);
                            }
                        },
                        onmousemove: move |event| {
                            event.prevent_default();
                            let placement = if event.element_coordinates().x < TAB_DROP_MIDPOINT_PX {
                                DropPlacement::Before
                            } else {
                                DropPlacement::After
                            };
                            on_drop_marker.call(DropMarker { id: tab.id, placement });
                        },
                        onmouseup: move |_| on_drag_end.call(()),
                        button {
                            class: "rton-file-tab-label",
                            role: "tab",
                            aria_selected: tab.id == active_tab_id,
                            onclick: move |_| on_activate.call(tab.id),
                            title: i18n.t_args("tabs-switch-to", &[("name", tab.file_name.clone())]),
                            span { class: "rton-file-tab-name", "{leaf_display_name(&tab.file_name)}" }
                            if tab.dirty {
                                span { class: "rton-file-tab-dirty" }
                            }
                        }
                        button {
                            class: "rton-file-tab-close",
                            disabled: !tab.closeable,
                            title: i18n.t("title-close-tab"),
                            aria_label: i18n.t("title-close-tab"),
                            onmousedown: move |event| event.stop_propagation(),
                            onclick: move |event| {
                                event.stop_propagation();
                                on_close.call(tab.id);
                            },
                            "×"
                        }
                    }
                }
            }
        }
    }
}
