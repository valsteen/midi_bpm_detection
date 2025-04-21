use std::{
    mem,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use atomic_float::AtomicF32;
use atomic_refcell::AtomicRefCell;
use bpm_detection_core::{bpm_detection_receiver::BPMDetectionReceiver, parameters::max_histogram_data_buffer_size};
use derivative::Derivative;
use eframe::egui::{Context, ViewportCommand, WindowLevel};
use errors::{LogErrorWithExt, LogOptionWithExt, minitrace};
use instant::Instant;
use sync::Mutex;

#[derive(Clone, Derivative)]
#[derivative(Debug)]
pub struct GuiRemote {
    pub(crate) context: Arc<AtomicRefCell<Option<Context>>>,
    #[derivative(Debug = "ignore")]
    pub(crate) keys_sender: Arc<Mutex<Option<Box<dyn FnMut(&'static str) + Send>>>>,
    #[derivative(Debug = "ignore")]
    pub(crate) on_gui_exit_callback: Arc<Mutex<Option<Box<dyn Fn() + Send>>>>,
    pub(crate) swap_histogram_data_points: Arc<AtomicRefCell<Vec<f32>>>,
    pub(crate) histogram_data_points: Arc<AtomicRefCell<HistogramDataPoints>>,
    pub(crate) estimated_bpm: Arc<AtomicF32>,
    pub(crate) daw_bpm: Arc<AtomicF32>,
    pub(crate) should_save: Arc<AtomicBool>,
}

#[allow(forbidden_lint_groups)]
#[allow(clippy::struct_field_names)]
#[derive(Derivative)]
#[derivative(Debug)]
pub(crate) struct HistogramDataPoints {
    pub(crate) inbound_histogram_data_points: Vec<f32>,
    pub(crate) inbound_histogram_data_update: Instant,
}

impl Default for HistogramDataPoints {
    fn default() -> Self {
        Self {
            inbound_histogram_data_points: Vec::with_capacity(max_histogram_data_buffer_size()),
            inbound_histogram_data_update: Instant::now(),
        }
    }
}

impl BPMDetectionReceiver for GuiRemote {
    fn receive_bpm_histogram_data(&mut self, histogram_data_points: &[f32], detected_bpm: f32) {
        let mut swap_histogram_data_points = self.swap_histogram_data_points.borrow_mut();
        swap_histogram_data_points.resize(histogram_data_points.len(), 0.0);
        swap_histogram_data_points.copy_from_slice(histogram_data_points);

        self.histogram_data_points
            .try_borrow_mut()
            .map(|mut histogram_data_points| {
                let HistogramDataPoints { inbound_histogram_data_points, inbound_histogram_data_update } =
                    &mut *histogram_data_points;
                mem::swap(inbound_histogram_data_points, &mut *swap_histogram_data_points);
                *inbound_histogram_data_update = Instant::now();
            })
            .log_error_msg("race condition while taking histogram_data_points, skipping update")
            .ok();

        self.estimated_bpm.store(detected_bpm, Ordering::Relaxed);
        self.request_repaint();
    }

    fn receive_daw_bpm(&self, bpm: f32) {
        self.daw_bpm.store(bpm, Ordering::Relaxed);
    }
}

impl GuiRemote {
    pub fn save_config(&self) {
        self.should_save.store(true, Ordering::Relaxed);
    }

    pub fn set_on_gui_exit_callback<F: Fn() + Send + 'static>(&self, callback: F) {
        self.on_gui_exit_callback.lock().replace(Box::new(callback));
    }

    pub fn receive_keystrokes(&self, sender: Box<dyn FnMut(&'static str) + Send + Sync>) {
        self.keys_sender.lock().replace(sender);
    }

    #[minitrace::trace]
    pub fn close(&self) {
        if let Ok(context) = self.context.try_borrow().log_error_msg("could not get context to close window") {
            if let Some(context) = context.as_ref().log_error_msg("no context present") {
                context.send_viewport_cmd(ViewportCommand::Close);
            }
        }
    }

    #[minitrace::trace]
    pub fn always_on_top(&self) {
        if let Ok(context) = self.context.try_borrow().log_error_msg("could not get context to put window on top") {
            if let Some(context) = context.as_ref().log_error_msg("no context present") {
                context.send_viewport_cmd(ViewportCommand::WindowLevel(WindowLevel::AlwaysOnTop));
            }
        }
    }

    #[minitrace::trace]
    pub fn always_on_top_cancel(&self) {
        if let Ok(context) = self.context.try_borrow().log_error_msg("could not get context to cancel window on top") {
            if let Some(context) = context.as_ref().log_error_msg("no context present") {
                context.send_viewport_cmd(ViewportCommand::WindowLevel(WindowLevel::Normal));
            }
        }
    }

    #[must_use]
    pub fn get_context(&self) -> Option<Context> {
        self.context.try_borrow().ok()?.clone()
    }

    pub fn request_repaint(&self) {
        let Ok(context) = self.context.try_borrow() else {
            return;
        };

        if let Some(context) = context.as_ref() {
            context.request_repaint();
        }
    }
}
