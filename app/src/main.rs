mod app_actions;
mod app_constants;
mod app_i18n;
mod app_layout;
mod app_sample;
mod app_view;
mod batch_export_runner;
mod components;
mod domain;
mod file_import;
mod i18n;
mod platform;
#[cfg(test)]
mod tests;

#[cfg(target_arch = "wasm32")]
fn main() {
    dioxus::launch(app_view::App);
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    launch_desktop_app();
}

#[cfg(not(target_arch = "wasm32"))]
fn launch_desktop_app() {
    use std::cell::RefCell;
    use std::rc::Rc;

    use dioxus::desktop::{Config, LogicalSize, WindowBuilder, WindowEvent};

    use crate::app_constants::{DESKTOP_WINDOW_MIN_HEIGHT, DESKTOP_WINDOW_MIN_WIDTH};

    let size = platform::read_window_size_preference();
    let window_handle = Rc::new(RefCell::new(
        None::<std::sync::Arc<dioxus::desktop::tao::window::Window>>,
    ));
    let on_window_handle = window_handle.clone();
    let resize_window_handle = window_handle;
    let window = WindowBuilder::new()
        .with_title("RTON Editor")
        .with_inner_size(LogicalSize::new(size.width as f64, size.height as f64))
        .with_min_inner_size(LogicalSize::new(
            DESKTOP_WINDOW_MIN_WIDTH as f64,
            DESKTOP_WINDOW_MIN_HEIGHT as f64,
        ));
    let desktop_config = Config::new()
        .with_window(window)
        .with_on_window(move |window, _| {
            *on_window_handle.borrow_mut() = Some(window);
        })
        .with_custom_event_handler(move |event, _| {
            let dioxus::desktop::tao::event::Event::WindowEvent {
                window_id,
                event: WindowEvent::Resized(_),
                ..
            } = event
            else {
                return;
            };
            let Some(window) = resize_window_handle.borrow().as_ref().cloned() else {
                return;
            };
            if window.id() != *window_id {
                return;
            }
            let logical_size = window.inner_size().to_logical::<f64>(window.scale_factor());
            let width = logical_size.width.round().max(0.0) as u32;
            let height = logical_size.height.round().max(0.0) as u32;
            let _ = platform::save_window_size_preference(width, height);
        });

    dioxus::LaunchBuilder::desktop()
        .with_cfg(desktop_config)
        .launch(app_view::App);
}
