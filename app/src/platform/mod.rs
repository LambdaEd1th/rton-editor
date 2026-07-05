mod dialogs;
mod i18n_sources;
mod preferences;
mod save;
mod task;
mod worker;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeOpenFile {
    pub path: std::path::PathBuf,
    pub display_name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct I18nSource {
    pub code: String,
    pub source: String,
}

#[cfg(not(target_arch = "wasm32"))]
pub use dialogs::files_in_folder;
pub use dialogs::{open_files, open_folder};
#[cfg(not(target_arch = "wasm32"))]
pub use i18n_sources::read_i18n_sources;
#[cfg(target_arch = "wasm32")]
pub use i18n_sources::read_i18n_sources_async;
pub use preferences::{
    read_line_wrapping_preference, read_locale_preference, read_theme_preference,
    read_toolbar_layout_preference, save_line_wrapping_preference, save_locale_preference,
    save_theme_preference, save_toolbar_layout_preference, system_locale,
};
#[cfg(not(target_arch = "wasm32"))]
pub use preferences::{read_window_size_preference, save_window_size_preference};
pub use save::{save_bytes, save_text};
pub use task::{run_cpu_task, sleep_ms};
#[cfg(target_arch = "wasm32")]
pub use worker::{
    run_mode_switch_worker, run_open_text_file_worker, run_open_text_worker, run_parse_worker,
    run_rton_size_worker,
};
