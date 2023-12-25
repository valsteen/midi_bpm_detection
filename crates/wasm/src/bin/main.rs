#![allow(forbidden_lint_groups)]
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};
#[cfg(target_arch = "wasm32")]
use {
    errors::{initialize_panic_handler, Result},
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
