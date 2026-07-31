#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::module_name_repetitions)]

use std::sync::Arc;

pub use app::{BPMDetectionGUI, GuiLifecycleOwner};
use bpm_detection_config::max_histogram_data_buffer_size;
pub use display::{BpmDisplayPublisher, GuiContextHandle};
pub use editable_settings::{EditableSettings, GuiChanges};
pub use eframe;
use eframe::egui;

pub mod add_slider;
mod app;
mod config_ui;
mod display;
mod editable_settings;

#[must_use]
pub fn create_gui() -> (BpmDisplayPublisher, GuiContextHandle, BPMDetectionGUI) {
    let state = Arc::new(display::DisplayState::new());
    let publisher = BpmDisplayPublisher::new(Arc::downgrade(&state));
    let context = GuiContextHandle::new(Arc::downgrade(&state));
    let gui = BPMDetectionGUI::new(state);
    debug_assert_eq!(gui.interpolated_data_points.capacity(), max_histogram_data_buffer_size());
    (publisher, context, gui)
}

pub static GIT_COMMIT_HASH: &str = env!("_GIT_INFO");
include!(concat!(env!("OUT_DIR"), "/build_time.rs"));

#[cfg(test)]
#[path = "../tests/unit/display.rs"]
mod display_tests;
