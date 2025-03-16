#![allow(forbidden_lint_groups)]
use wasm_bindgen::{JsValue, prelude::wasm_bindgen};
#[cfg(target_arch = "wasm32")]
use {
    errors::{Result, initialize_panic_handler},
    gui::eframe,
    wasm::wasm::run,
};

#[wasm_bindgen]
extern "C" {
    fn start(gui_remote: JsValue);
}

#[cfg(target_arch = "wasm32")]
fn main() -> Result<()> {
    eframe::WebLogger::init(errors::LevelFilter::Debug)?;

    initialize_panic_handler(|| ())?;
    start(JsValue::from(run()?));
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {}
