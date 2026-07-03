use dioxus::prelude::*;
use dioxus_html::InteractionLocation;

use crate::app_actions::{finish_tab_drag_state, finish_toolbar_drag_state};
use crate::components::{PanelResizeDrag, PanelSide, clamp_panel_width};
use crate::domain::{
    DropMarker, EditorTabState, ToolbarDropTarget, ToolbarGroupId, apply_toolbar_drop_target,
};

pub(crate) fn update_panel_resize_width(
    event: MouseEvent,
    panel_resize_drag: Signal<Option<PanelResizeDrag>>,
    mut left_panel_width: Signal<i32>,
    mut right_panel_width: Signal<i32>,
) {
    let Some(drag) = *panel_resize_drag.read() else {
        return;
    };

    event.prevent_default();
    let x = event.client_coordinates().x;
    let delta = match drag.side {
        PanelSide::Left => x - drag.start_x,
        PanelSide::Right => drag.start_x - x,
    };
    let next_width = clamp_panel_width(drag.start_width as f64 + delta);
    match drag.side {
        PanelSide::Left => left_panel_width.set(next_width),
        PanelSide::Right => right_panel_width.set(next_width),
    }
}

pub(crate) fn finish_workspace_drag_state(
    mut panel_resize_drag: Signal<Option<PanelResizeDrag>>,
    tabs: Signal<Vec<EditorTabState>>,
    dragged_tab_id: Signal<Option<usize>>,
    tab_drop_marker: Signal<Option<DropMarker<usize>>>,
    toolbar_rows: Signal<Vec<Vec<ToolbarGroupId>>>,
    dragged_toolbar_group_id: Signal<Option<ToolbarGroupId>>,
    toolbar_drop_marker: Signal<Option<DropMarker<ToolbarGroupId>>>,
) {
    panel_resize_drag.set(None);
    finish_tab_drag_state(tabs, dragged_tab_id, tab_drop_marker);
    finish_toolbar_drag_state(toolbar_rows, dragged_toolbar_group_id, toolbar_drop_marker);
}

pub(crate) fn start_tab_drag_if_needed(
    id: usize,
    tabs: Signal<Vec<EditorTabState>>,
    mut dragged_tab_id: Signal<Option<usize>>,
    mut tab_drop_marker: Signal<Option<DropMarker<usize>>>,
) {
    if tabs.read().len() > 1 {
        dragged_tab_id.set(Some(id));
        tab_drop_marker.set(None);
    }
}

pub(crate) fn update_tab_drop_marker_for_drag(
    marker: DropMarker<usize>,
    dragged_tab_id: Signal<Option<usize>>,
    mut tab_drop_marker: Signal<Option<DropMarker<usize>>>,
) {
    if matches!(*dragged_tab_id.read(), Some(id) if id != marker.id) {
        tab_drop_marker.set(Some(marker));
    }
}

pub(crate) fn start_toolbar_group_drag_state(
    id: ToolbarGroupId,
    mut dragged_toolbar_group_id: Signal<Option<ToolbarGroupId>>,
    mut toolbar_drop_marker: Signal<Option<DropMarker<ToolbarGroupId>>>,
) {
    dragged_toolbar_group_id.set(Some(id));
    toolbar_drop_marker.set(None);
}

pub(crate) fn update_toolbar_drop_marker_for_drag(
    target: ToolbarDropTarget,
    mut toolbar_rows: Signal<Vec<Vec<ToolbarGroupId>>>,
    dragged_toolbar_group_id: Signal<Option<ToolbarGroupId>>,
    mut toolbar_drop_marker: Signal<Option<DropMarker<ToolbarGroupId>>>,
) {
    let Some(dragged_id) = *dragged_toolbar_group_id.read() else {
        return;
    };
    let marker = {
        let mut rows = toolbar_rows.write();
        apply_toolbar_drop_target(&mut rows, dragged_id, target)
    };
    toolbar_drop_marker.set(marker);
}
