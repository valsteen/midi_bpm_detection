use std::{
    mem,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use atomic_float::AtomicF32;
use atomic_refcell::AtomicRefCell;
use bpm_detection_config::max_histogram_data_buffer_size;
use bpm_detection_core::bpm_detection_receiver::BPMDetectionReceiver;
use derivative::Derivative;
use eframe::egui::{Context, ViewportCommand, WindowLevel};
use errors::{LogErrorWithExt, LogOptionWithExt, minitrace};
use instant::Instant;

use crate::callback_slot::ArcCallbackSlot;

#[derive(Clone, Derivative)]
#[derivative(Debug)]
pub struct GuiRemote {
    pub(crate) context: Arc<AtomicRefCell<Option<Context>>>,
    #[derivative(Debug = "ignore")]
    pub(crate) keys_sender: ArcCallbackSlot<dyn FnMut(&'static str) + Send>,
    #[derivative(Debug = "ignore")]
    pub(crate) on_gui_exit_callback: ArcCallbackSlot<dyn Fn() + Send>,
    pub(crate) producer_histogram_scratch: Arc<AtomicRefCell<Vec<f32>>>,
    pub(crate) gui_histogram_snapshot: Arc<AtomicRefCell<HistogramSnapshot>>,
    pub(crate) estimated_bpm: Arc<AtomicF32>,
    pub(crate) daw_bpm: Arc<AtomicF32>,
    pub(crate) should_save: Arc<AtomicBool>,
}

#[derive(Derivative)]
#[derivative(Debug)]
pub(crate) struct HistogramSnapshot {
    pub(crate) data_points: Vec<f32>,
    pub(crate) updated_at: Instant,
}

impl Default for HistogramSnapshot {
    fn default() -> Self {
        Self { data_points: Vec::with_capacity(max_histogram_data_buffer_size()), updated_at: Instant::now() }
    }
}

impl BPMDetectionReceiver for GuiRemote {
    fn receive_bpm_histogram_data(&mut self, histogram_data_points: &[f32], detected_bpm: f32) {
        let mut producer_histogram_scratch = self.producer_histogram_scratch.borrow_mut();
        producer_histogram_scratch.resize(histogram_data_points.len(), 0.0);
        producer_histogram_scratch.copy_from_slice(histogram_data_points);

        // Publish only a complete snapshot when the GUI snapshot is immediately available. If it is busy, deliberately
        // drop this visualization update rather than block or retry.
        self.gui_histogram_snapshot
            .try_borrow_mut()
            .map(|mut gui_histogram_snapshot| {
                let HistogramSnapshot { data_points, updated_at } = &mut *gui_histogram_snapshot;
                mem::swap(data_points, &mut *producer_histogram_scratch);
                *updated_at = Instant::now();
            })
            .log_error_msg("GUI histogram snapshot busy; dropping best-effort visualization update")
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
        if let Ok(context) = self.context.try_borrow().log_error_msg("could not get context to close window")
            && let Some(context) = context.as_ref().log_error_msg("no context present")
        {
            context.send_viewport_cmd(ViewportCommand::Close);
        }
    }

    #[minitrace::trace]
    pub fn always_on_top(&self) {
        if let Ok(context) = self.context.try_borrow().log_error_msg("could not get context to put window on top")
            && let Some(context) = context.as_ref().log_error_msg("no context present")
        {
            context.send_viewport_cmd(ViewportCommand::WindowLevel(WindowLevel::AlwaysOnTop));
        }
    }

    #[minitrace::trace]
    pub fn always_on_top_cancel(&self) {
        if let Ok(context) = self.context.try_borrow().log_error_msg("could not get context to cancel window on top")
            && let Some(context) = context.as_ref().log_error_msg("no context present")
        {
            context.send_viewport_cmd(ViewportCommand::WindowLevel(WindowLevel::Normal));
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
