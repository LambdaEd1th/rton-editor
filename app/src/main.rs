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
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    use dioxus::desktop::tao::event::Event;
    use dioxus::desktop::tao::window::Window;
    use dioxus::desktop::{Config, LogicalSize, WindowBuilder, WindowEvent};

    use crate::app_constants::{DESKTOP_WINDOW_MIN_HEIGHT, DESKTOP_WINDOW_MIN_WIDTH};

    let size = platform::read_window_size_preference();
    let window_handle = Rc::new(RefCell::new(None::<std::sync::Arc<Window>>));
    let window_scale_factor = Rc::new(Cell::new(1.0_f64));
    let on_window_handle = window_handle.clone();
    let on_window_scale_factor = window_scale_factor.clone();
    let event_window_handle = window_handle;
    let event_window_scale_factor = window_scale_factor;
    let window = WindowBuilder::new()
        .with_title("RTON Editor")
        .with_inner_size(LogicalSize::new(size.width as f64, size.height as f64))
        .with_min_inner_size(LogicalSize::new(
            DESKTOP_WINDOW_MIN_WIDTH as f64,
            DESKTOP_WINDOW_MIN_HEIGHT as f64,
        ));
    let desktop_config = Config::new()
        .with_window(window)
        .with_menu(None)
        .with_on_window(move |window, _| {
            on_window_scale_factor.set(window.scale_factor());
            window.set_inner_size(LogicalSize::new(size.width as f64, size.height as f64));
            *on_window_handle.borrow_mut() = Some(window);
        })
        .with_custom_event_handler(move |event, _| {
            let window = event_window_handle.borrow().as_ref().cloned();
            if let Some(window) = window.as_ref() {
                event_window_scale_factor.set(window.scale_factor());
            }
            match event {
                Event::WindowEvent {
                    window_id,
                    event: WindowEvent::Resized(size),
                    ..
                } if window
                    .as_ref()
                    .is_none_or(|window| window.id() == *window_id) =>
                {
                    save_desktop_window_size_from_physical(
                        size.width,
                        size.height,
                        event_window_scale_factor.get(),
                    );
                }
                Event::WindowEvent {
                    window_id,
                    event:
                        WindowEvent::ScaleFactorChanged {
                            scale_factor,
                            new_inner_size,
                        },
                    ..
                } if window
                    .as_ref()
                    .is_none_or(|window| window.id() == *window_id) =>
                {
                    event_window_scale_factor.set(*scale_factor);
                    save_desktop_window_size_from_physical(
                        new_inner_size.width,
                        new_inner_size.height,
                        *scale_factor,
                    );
                }
                Event::WindowEvent {
                    window_id,
                    event: WindowEvent::CloseRequested | WindowEvent::Destroyed,
                    ..
                } if window
                    .as_ref()
                    .is_some_and(|window| window.id() == *window_id) =>
                {
                    if let Some(window) = window {
                        save_desktop_window_size(&window);
                    }
                }
                Event::LoopDestroyed => {
                    if let Some(window) = window {
                        save_desktop_window_size(&window);
                    }
                }
                _ => {}
            }
        });

    dioxus::LaunchBuilder::desktop()
        .with_cfg(desktop_config)
        .launch(app_view::App);
}

#[cfg(not(target_arch = "wasm32"))]
fn save_desktop_window_size(window: &dioxus::desktop::tao::window::Window) {
    let Some((width, height)) = desktop_window_logical_size(window) else {
        return;
    };
    if let Err(error) = platform::save_window_size_preference(width, height) {
        eprintln!("failed to save window size preference: {error}");
    }
}

#[cfg(all(not(target_arch = "wasm32"), target_os = "macos"))]
fn desktop_window_logical_size(
    window: &dioxus::desktop::tao::window::Window,
) -> Option<(u32, u32)> {
    if !window.scale_factor().is_finite() || window.scale_factor() <= 0.0 {
        return None;
    }

    // tao's inner_size is unreliable on macOS. Dioxus works around it by using
    // outer_size and subtracting the decorated title bar height.
    let size = window.outer_size().to_logical::<u32>(window.scale_factor());
    let title_bar_adjustment = if window.is_decorated() { 28 } else { 0 };
    Some((size.width, size.height.saturating_sub(title_bar_adjustment)))
}

#[cfg(all(not(target_arch = "wasm32"), not(target_os = "macos")))]
fn desktop_window_logical_size(
    window: &dioxus::desktop::tao::window::Window,
) -> Option<(u32, u32)> {
    if !window.scale_factor().is_finite() || window.scale_factor() <= 0.0 {
        return None;
    }

    let size = window.inner_size().to_logical::<u32>(window.scale_factor());
    Some((size.width, size.height))
}

#[cfg(not(target_arch = "wasm32"))]
fn save_desktop_window_size_from_physical(width: u32, height: u32, scale_factor: f64) {
    if width == 0 || height == 0 || !scale_factor.is_finite() || scale_factor <= 0.0 {
        return;
    }
    let logical_size =
        dioxus::desktop::tao::dpi::PhysicalSize::new(width, height).to_logical::<f64>(scale_factor);
    let width = logical_size.width.round().max(0.0) as u32;
    let height = logical_size.height.round().max(0.0) as u32;
    if let Err(error) = platform::save_window_size_preference(width, height) {
        eprintln!("failed to save window size preference: {error}");
    }
}
