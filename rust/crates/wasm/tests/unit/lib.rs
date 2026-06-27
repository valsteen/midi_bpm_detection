#![allow(clippy::missing_panics_doc)]
use errors::error_backtrace;
use parameter::OnOff;
use serde::{Deserialize, Serialize};
#[allow(clippy::module_name_repetitions)]
use wasm_bindgen_test::*;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub test: OnOff<f32>,
}

impl Default for Config {
    fn default() -> Self {
        match toml::de::Deserializer::parse(CONFIG).and_then(Config::deserialize) {
            Ok(config) => config,
            Err(err) => {
                error_backtrace!("{err}");
                panic!("invalid built-in configuration");
            }
        }
    }
}

const CONFIG: &str = "[test]
enabled = false
value = 1";

#[wasm_bindgen_test]
fn test_config() {
    let config = Config::default();
    assert_eq!(config.test, OnOff::Off(1.0));
}
