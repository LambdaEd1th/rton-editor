use dioxus::prelude::*;

use crate::app_actions::initial_toolbar_rows;
use crate::app_constants::{LEFT_PANEL_DEFAULT_WIDTH, RIGHT_PANEL_DEFAULT_WIDTH};
use crate::components::{FileSelection, HexJumpTarget, PanelResizeDrag, TextJumpTarget};
use crate::domain::{DropMarker, EditorTabState, Status, ThemePreference, Tone, ToolbarGroupId};
use crate::file_import::LoadedFileState;
use crate::i18n::{I18n, Locale};
use crate::platform;

pub(super) struct AppSignals {
    pub(super) tabs: Signal<Vec<EditorTabState>>,
    pub(super) loaded_files: Signal<Vec<LoadedFileState>>,
    pub(super) active_tab_id: Signal<usize>,
    pub(super) next_tab_id: Signal<usize>,
    pub(super) next_loaded_file_id: Signal<usize>,
    pub(super) compact_output: Signal<bool>,
    pub(super) encrypt_output: Signal<bool>,
    pub(super) dragging_files: Signal<bool>,
    pub(super) theme_preference: Signal<ThemePreference>,
    pub(super) locale: Signal<Locale>,
    pub(super) i18n_revision: Signal<u64>,
    #[cfg(target_arch = "wasm32")]
    pub(super) i18n_loaded: Signal<bool>,
    pub(super) line_wrapping: Signal<bool>,
    pub(super) editor_search_panel_visible: Signal<bool>,
    pub(super) editor_search_text: Signal<String>,
    pub(super) editor_replace_text: Signal<String>,
    pub(super) editor_search_case_sensitive: Signal<bool>,
    pub(super) editor_search_match_index: Signal<usize>,
    pub(super) editor_search_focus_token: Signal<u64>,
    pub(super) file_search_query: Signal<String>,
    pub(super) file_selection: Signal<FileSelection>,
    pub(super) next_jump_id: Signal<u64>,
    pub(super) text_jump_target: Signal<Option<TextJumpTarget>>,
    pub(super) hex_jump_target: Signal<Option<HexJumpTarget>>,
    pub(super) left_panel_width: Signal<i32>,
    pub(super) right_panel_width: Signal<i32>,
    pub(super) panel_resize_drag: Signal<Option<PanelResizeDrag>>,
    pub(super) dragged_tab_id: Signal<Option<usize>>,
    pub(super) tab_drop_marker: Signal<Option<DropMarker<usize>>>,
    pub(super) toolbar_rows: Signal<Vec<Vec<ToolbarGroupId>>>,
    pub(super) dragged_toolbar_group_id: Signal<Option<ToolbarGroupId>>,
    pub(super) toolbar_drop_marker: Signal<Option<DropMarker<ToolbarGroupId>>>,
    pub(super) status: Signal<Status>,
}

pub(super) fn use_app_signals(initial_locale_snapshot: Locale) -> AppSignals {
    AppSignals {
        tabs: use_signal(Vec::<EditorTabState>::new),
        loaded_files: use_signal(Vec::<LoadedFileState>::new),
        active_tab_id: use_signal(|| 0_usize),
        next_tab_id: use_signal(|| 1_usize),
        next_loaded_file_id: use_signal(|| 1_usize),
        compact_output: use_signal(|| false),
        encrypt_output: use_signal(|| false),
        dragging_files: use_signal(|| false),
        theme_preference: use_signal(|| ThemePreference::System),
        locale: use_signal(move || initial_locale_snapshot),
        i18n_revision: use_signal(|| 0_u64),
        #[cfg(target_arch = "wasm32")]
        i18n_loaded: use_signal(|| false),
        line_wrapping: use_signal(platform::read_line_wrapping_preference),
        editor_search_panel_visible: use_signal(|| false),
        editor_search_text: use_signal(String::new),
        editor_replace_text: use_signal(String::new),
        editor_search_case_sensitive: use_signal(|| false),
        editor_search_match_index: use_signal(|| 0_usize),
        editor_search_focus_token: use_signal(|| 0_u64),
        file_search_query: use_signal(String::new),
        file_selection: use_signal(FileSelection::default),
        next_jump_id: use_signal(|| 1_u64),
        text_jump_target: use_signal(|| None::<TextJumpTarget>),
        hex_jump_target: use_signal(|| None::<HexJumpTarget>),
        left_panel_width: use_signal(|| LEFT_PANEL_DEFAULT_WIDTH),
        right_panel_width: use_signal(|| RIGHT_PANEL_DEFAULT_WIDTH),
        panel_resize_drag: use_signal(|| None::<PanelResizeDrag>),
        dragged_tab_id: use_signal(|| None::<usize>),
        tab_drop_marker: use_signal(|| None::<DropMarker<usize>>),
        toolbar_rows: use_signal(initial_toolbar_rows),
        dragged_toolbar_group_id: use_signal(|| None::<ToolbarGroupId>),
        toolbar_drop_marker: use_signal(|| None::<DropMarker<ToolbarGroupId>>),
        status: use_signal(move || {
            Status::new(
                I18n::new(initial_locale_snapshot).t("status-ready"),
                Tone::Ok,
            )
        }),
    }
}
