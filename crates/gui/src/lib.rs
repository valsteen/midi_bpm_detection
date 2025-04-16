#![allow(forbidden_lint_groups)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::module_name_repetitions)]

use std::sync::{Arc, atomic::AtomicBool};

pub use app::{BPMDetectionApp, BPMDetectionGUI};
use atomic_float::AtomicF32;
use atomic_refcell::AtomicRefCell;
use bpm_detection_core::bpm::max_histogram_data_buffer_size;
pub use eframe;
use eframe::egui;
#[cfg(not(target_arch = "wasm32"))]
use errors::MakeReportExt;
use errors::Result;
pub use gui_remote::GuiRemote;
#[cfg(not(target_arch = "wasm32"))]
use log::info;
use sync::Mutex;

use crate::gui_remote::HistogramDataPoints;
pub use crate::{application_parameters::BPMDetectionConfig, config::GUIConfigAccessor};

pub mod add_slider;
mod app;
mod app_builder;
mod application_parameters;
mod config;
mod config_ui;
mod gui_remote;

pub use config::{DefaultGUIParameters, GUIConfig, GUIParameters};

use crate::app_builder::AppBuilder;

pub fn create_gui<BaseConfig>(base_config: BaseConfig) -> (GuiRemote, AppBuilder<BaseConfig>) {
    let estimated_bpm = Arc::new(AtomicF32::new(f32::NAN));
    let daw_bpm = Arc::new(AtomicF32::new(f32::NAN));
    let should_save = Arc::new(AtomicBool::default());

    let context_receiver = Arc::new(AtomicRefCell::new(None));
    let keys_sender = Arc::new(Mutex::new(None));
    let weak_keys_sender = Arc::downgrade(&keys_sender);
    let gui_exit_callback = Arc::new(Mutex::new(None));

    #[cfg(not(target_arch = "wasm32"))]
    let weak_on_gui_exit_callback = Arc::downgrade(&gui_exit_callback);

    let histogram_data_points = Arc::new(AtomicRefCell::new(HistogramDataPoints::default()));

    let bpm_detection_gui = BPMDetectionGUI {
        keys_sender: weak_keys_sender,
        #[cfg(not(target_arch = "wasm32"))]
        on_gui_exit_callback: weak_on_gui_exit_callback,
        histogram_data_points: Arc::downgrade(&histogram_data_points),
        interpolated_data_points: Vec::with_capacity(max_histogram_data_buffer_size()),
        estimated_bpm: Arc::downgrade(&estimated_bpm),
        daw_bpm: Arc::downgrade(&daw_bpm),
        should_save: Arc::downgrade(&should_save),
    };

    let gui_remote = GuiRemote {
        context: context_receiver.clone(),
        keys_sender,
        on_gui_exit_callback: gui_exit_callback,
        swap_histogram_data_points: Arc::new(AtomicRefCell::new(Vec::with_capacity(max_histogram_data_buffer_size()))),
        histogram_data_points,
        estimated_bpm,
        daw_bpm,
        should_save,
    };
    (gui_remote, AppBuilder::new(context_receiver, bpm_detection_gui, base_config))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn start_gui<Config: BPMDetectionConfig>(app_builder: AppBuilder<Config>) -> Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([640.0, 480.0]),
        persist_window: true,

        ..Default::default()
    };

    eframe::run_native(
        "Estimated BPM",
        options,
        Box::new({
            move |cc| {
                // This gives us image support:
                egui_extras::install_image_loaders(&cc.egui_ctx);
                let bpm_detection_app = app_builder.build(cc.egui_ctx.clone());
                Ok(Box::new(bpm_detection_app))
            }
        }),
    )
    .report_msg("Could not display eframe")?;
    info!("gui exit");
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub fn start_gui<P>(gui_builder: AppBuilder<P>) -> Result<()>
where
    P: BPMDetectionConfig + 'static,
{
    use eframe::wasm_bindgen::JsCast;

    let document = web_sys::window().expect("No window").document().expect("No document");

    let canvas = document
        .get_element_by_id("the_canvas_id")
        .expect("Failed to find the_canvas_id")
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .expect("the_canvas_id was not a HtmlCanvasElement");

    wasm_bindgen_futures::spawn_local(async {
        eframe::WebRunner::new()
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(move |cc| {
                    cc.egui_ctx.set_theme(egui::ThemePreference::Dark);
                    let bpm_detection_app = gui_builder.build(cc.egui_ctx.clone());
                    Ok(Box::new(bpm_detection_app))
                }),
            )
            .await
            .expect("failed to start eframe");
    });
    Ok(())
}

pub static GIT_COMMIT_HASH: &str = env!("_GIT_INFO");
include!(concat!(env!("OUT_DIR"), "/build_time.rs"));
