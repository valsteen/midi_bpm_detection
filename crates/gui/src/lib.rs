#![allow(forbidden_lint_groups)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::module_name_repetitions)]

pub use gui_remote::GuiRemote;
use std::sync::{Arc, atomic::AtomicBool};

pub use app::BPMDetectionGUI;
use atomic_float::AtomicF32;
use atomic_refcell::AtomicRefCell;

pub use eframe;
use eframe::{egui, egui::Context};

#[cfg(not(target_arch = "wasm32"))]
use errors::MakeReportExt;
#[cfg(not(target_arch = "wasm32"))]
use log::info;

use sync::Mutex;

use errors::Result;
use midi::bpm::max_histogram_data_buffer_size;

pub use crate::application_parameters::BPMDetectionParameters;
use crate::gui_remote::HistogramDataPoints;

pub mod add_slider;
mod app;
mod application_parameters;
mod config;
mod config_ui;
mod gui_remote;

pub use config::GUIConfig;

pub fn create_gui<P: BPMDetectionParameters>(bpm_detection_parameters: P) -> (GuiRemote, GUIBuilder<P>) {
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
        live_parameters: bpm_detection_parameters,
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
    (gui_remote, GUIBuilder { context_receiver, bpm_detection_gui })
}

pub struct GUIBuilder<P: BPMDetectionParameters + 'static> {
    context_receiver: Arc<AtomicRefCell<Option<Context>>>,
    bpm_detection_gui: BPMDetectionGUI<P>,
}

impl<P> GUIBuilder<P>
where
    P: BPMDetectionParameters + 'static,
{
    pub fn build(self, context: Context) -> BPMDetectionGUI<P> {
        self.context_receiver.borrow_mut().replace(context);
        self.bpm_detection_gui
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn start_gui<P>(gui_builder: GUIBuilder<P>) -> Result<()>
where
    P: BPMDetectionParameters + 'static,
{
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
                gui_builder.context_receiver.borrow_mut().replace(cc.egui_ctx.clone());
                Ok(Box::new(gui_builder.bpm_detection_gui))
            }
        }),
    )
    .report_msg("Could not display eframe")?;
    info!("gui exit");
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub fn start_gui<P>(gui_builder: GUIBuilder<P>) -> Result<()>
where
    P: BPMDetectionParameters + 'static,
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
                    gui_builder.context_receiver.borrow_mut().replace(cc.egui_ctx.clone());
                    Ok(Box::new(gui_builder.bpm_detection_gui))
                }),
            )
            .await
            .expect("failed to start eframe");
    });
    Ok(())
}

pub static GIT_COMMIT_HASH: &str = env!("_GIT_INFO");
include!(concat!(env!("OUT_DIR"), "/build_time.rs"));
