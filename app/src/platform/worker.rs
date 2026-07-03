#[cfg(target_arch = "wasm32")]
use js_sys::{Function, Promise, Reflect};
#[cfg(target_arch = "wasm32")]
use rton_editor_core::{
    WorkerModeSwitchRequest, WorkerModeSwitchResponse, WorkerParseRequest, WorkerParseResponse,
    WorkerRtonSizeRequest, WorkerRtonSizeResponse,
};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::JsFuture;
#[cfg(target_arch = "wasm32")]
use web_sys::{ErrorEvent, MessageEvent, Worker, WorkerOptions, WorkerType};

#[cfg(target_arch = "wasm32")]
const RTON_WORKER_URL: &str = "assets/worker/rton-worker.js";

#[cfg(target_arch = "wasm32")]
pub async fn run_mode_switch_worker(
    request: WorkerModeSwitchRequest,
) -> Result<WorkerModeSwitchResponse, String> {
    let request = serde_wasm_bindgen::to_value(&request).map_err(|error| error.to_string())?;
    let options = WorkerOptions::new();
    options.set_type(WorkerType::Module);
    let worker = Worker::new_with_options(RTON_WORKER_URL, &options).map_err(js_error_string)?;
    let message = worker_message("mode-switch", &request)?;
    let response = await_worker_value(&worker, message)
        .await
        .and_then(parse_mode_switch_envelope);
    worker.terminate();
    response
}

#[cfg(target_arch = "wasm32")]
pub async fn run_parse_worker(request: WorkerParseRequest) -> Result<WorkerParseResponse, String> {
    let request = serde_wasm_bindgen::to_value(&request).map_err(|error| error.to_string())?;
    let options = WorkerOptions::new();
    options.set_type(WorkerType::Module);
    let worker = Worker::new_with_options(RTON_WORKER_URL, &options).map_err(js_error_string)?;
    let message = worker_message("parse", &request)?;
    let response = await_worker_value(&worker, message)
        .await
        .and_then(parse_parse_envelope);
    worker.terminate();
    response
}

#[cfg(target_arch = "wasm32")]
pub async fn run_rton_size_worker(
    request: WorkerRtonSizeRequest,
) -> Result<WorkerRtonSizeResponse, String> {
    let request = serde_wasm_bindgen::to_value(&request).map_err(|error| error.to_string())?;
    let options = WorkerOptions::new();
    options.set_type(WorkerType::Module);
    let worker = Worker::new_with_options(RTON_WORKER_URL, &options).map_err(js_error_string)?;
    let message = worker_message("rton-size", &request)?;
    let response = await_worker_value(&worker, message)
        .await
        .and_then(parse_rton_size_envelope);
    worker.terminate();
    response
}

#[cfg(target_arch = "wasm32")]
fn worker_message(kind: &str, request: &JsValue) -> Result<JsValue, String> {
    let message = js_sys::Object::new();
    Reflect::set(
        &message,
        &JsValue::from_str("kind"),
        &JsValue::from_str(kind),
    )
    .map_err(js_error_string)?;
    Reflect::set(&message, &JsValue::from_str("request"), request).map_err(js_error_string)?;
    Ok(message.into())
}

#[cfg(target_arch = "wasm32")]
async fn await_worker_value(worker: &Worker, message: JsValue) -> Result<JsValue, String> {
    let worker_for_promise = worker.clone();
    let promise = Promise::new(&mut move |resolve: Function, reject: Function| {
        let resolve_message = resolve.clone();
        let reject_message = reject.clone();
        let onmessage = Closure::<dyn FnMut(MessageEvent)>::once(move |event: MessageEvent| {
            let _ = resolve_message.call1(&JsValue::UNDEFINED, &event.data());
        });
        let onerror = Closure::<dyn FnMut(ErrorEvent)>::once(move |event: ErrorEvent| {
            let message = if event.message().is_empty() {
                "Worker error".to_string()
            } else {
                event.message()
            };
            let _ = reject_message.call1(&JsValue::UNDEFINED, &JsValue::from_str(&message));
        });

        worker_for_promise.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
        worker_for_promise.set_onerror(Some(onerror.as_ref().unchecked_ref()));
        onmessage.forget();
        onerror.forget();

        if let Err(error) = worker_for_promise.post_message(&message) {
            let _ = reject.call1(&JsValue::UNDEFINED, &error);
        }
    });

    JsFuture::from(promise).await.map_err(js_error_string)
}

#[cfg(target_arch = "wasm32")]
fn parse_mode_switch_envelope(value: JsValue) -> Result<WorkerModeSwitchResponse, String> {
    let response = worker_response_value(value)?;
    serde_wasm_bindgen::from_value(response).map_err(|error| error.to_string())
}

#[cfg(target_arch = "wasm32")]
fn parse_parse_envelope(value: JsValue) -> Result<WorkerParseResponse, String> {
    let response = worker_response_value(value)?;
    serde_wasm_bindgen::from_value(response).map_err(|error| error.to_string())
}

#[cfg(target_arch = "wasm32")]
fn parse_rton_size_envelope(value: JsValue) -> Result<WorkerRtonSizeResponse, String> {
    let response = worker_response_value(value)?;
    serde_wasm_bindgen::from_value(response).map_err(|error| error.to_string())
}

#[cfg(target_arch = "wasm32")]
fn worker_response_value(value: JsValue) -> Result<JsValue, String> {
    let ok = Reflect::get(&value, &JsValue::from_str("ok"))
        .ok()
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    if !ok {
        let error = Reflect::get(&value, &JsValue::from_str("error"))
            .ok()
            .and_then(|value| value.as_string())
            .unwrap_or_else(|| "Worker failed".to_string());
        return Err(error);
    }

    Reflect::get(&value, &JsValue::from_str("response")).map_err(js_error_string)
}

#[cfg(target_arch = "wasm32")]
fn js_error_string(value: JsValue) -> String {
    value.as_string().unwrap_or_else(|| format!("{value:?}"))
}
