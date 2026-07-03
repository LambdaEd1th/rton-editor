use wasm_bindgen::prelude::*;

use rton_editor_core::{
    WorkerModeSwitchRequest, WorkerParseRequest, WorkerRtonSizeRequest, perform_worker_mode_switch,
    perform_worker_parse, perform_worker_rton_size,
};

#[wasm_bindgen]
pub fn rton_worker_mode_switch(request: JsValue) -> Result<JsValue, JsValue> {
    console_error_panic_hook::set_once();

    let request = serde_wasm_bindgen::from_value::<WorkerModeSwitchRequest>(request)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    let response = perform_worker_mode_switch(request)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    serde_wasm_bindgen::to_value(&response).map_err(|error| JsValue::from_str(&error.to_string()))
}

#[wasm_bindgen]
pub fn rton_worker_parse(request: JsValue) -> Result<JsValue, JsValue> {
    console_error_panic_hook::set_once();

    let request = serde_wasm_bindgen::from_value::<WorkerParseRequest>(request)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    let response =
        perform_worker_parse(request).map_err(|error| JsValue::from_str(&error.to_string()))?;
    serde_wasm_bindgen::to_value(&response).map_err(|error| JsValue::from_str(&error.to_string()))
}

#[wasm_bindgen]
pub fn rton_worker_rton_size(request: JsValue) -> Result<JsValue, JsValue> {
    console_error_panic_hook::set_once();

    let request = serde_wasm_bindgen::from_value::<WorkerRtonSizeRequest>(request)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    let response =
        perform_worker_rton_size(request).map_err(|error| JsValue::from_str(&error.to_string()))?;
    serde_wasm_bindgen::to_value(&response).map_err(|error| JsValue::from_str(&error.to_string()))
}
