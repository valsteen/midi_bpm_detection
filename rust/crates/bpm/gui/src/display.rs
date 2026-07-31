use std::{
    mem,
    sync::{Arc, Weak, atomic::Ordering},
};

use atomic_float::AtomicF32;
use atomic_refcell::AtomicRefCell;
use bpm_detection_config::max_histogram_data_buffer_size;
use bpm_detection_core::bpm_detection_receiver::BPMDetectionReceiver;
use eframe::egui::Context;
use errors::LogErrorWithExt;
use instant::Instant;

pub(crate) struct DisplayState {
    pub(crate) context: AtomicRefCell<Option<Context>>,
    pub(crate) histogram: AtomicRefCell<HistogramSnapshot>,
    pub(crate) estimated_bpm: AtomicF32,
    pub(crate) daw_bpm: AtomicF32,
}

impl DisplayState {
    pub(crate) fn new() -> Self {
        Self {
            context: AtomicRefCell::new(None),
            histogram: AtomicRefCell::new(HistogramSnapshot::default()),
            estimated_bpm: AtomicF32::new(f32::NAN),
            daw_bpm: AtomicF32::new(f32::NAN),
        }
    }
}

pub(crate) struct HistogramSnapshot {
    pub(crate) data_points: Vec<f32>,
    pub(crate) updated_at: Instant,
}

impl Default for HistogramSnapshot {
    fn default() -> Self {
        Self { data_points: Vec::with_capacity(max_histogram_data_buffer_size()), updated_at: Instant::now() }
    }
}

#[derive(Clone)]
pub struct BpmDisplayPublisher {
    state: Weak<DisplayState>,
    producer_histogram_scratch: Arc<AtomicRefCell<Vec<f32>>>,
}

impl BpmDisplayPublisher {
    pub(crate) fn new(state: Weak<DisplayState>) -> Self {
        Self {
            state,
            producer_histogram_scratch: Arc::new(AtomicRefCell::new(Vec::with_capacity(
                max_histogram_data_buffer_size(),
            ))),
        }
    }

    fn request_repaint(&self) {
        let Some(state) = self.state.upgrade() else {
            return;
        };
        let Ok(context) = state.context.try_borrow() else {
            return;
        };
        if let Some(context) = context.as_ref() {
            context.request_repaint();
        }
    }
}

impl BPMDetectionReceiver for BpmDisplayPublisher {
    fn receive_bpm_histogram_data(&mut self, histogram_data_points: &[f32], detected_bpm: f32) {
        let Some(state) = self.state.upgrade() else {
            return;
        };

        state.estimated_bpm.store(detected_bpm, Ordering::Relaxed);
        let Ok(mut producer_histogram_scratch) = self
            .producer_histogram_scratch
            .try_borrow_mut()
            .log_error_msg("GUI producer histogram scratch busy; dropping best-effort visualization update")
        else {
            self.request_repaint();
            return;
        };
        producer_histogram_scratch.resize(histogram_data_points.len(), 0.0);
        producer_histogram_scratch.copy_from_slice(histogram_data_points);

        state
            .histogram
            .try_borrow_mut()
            .map(|mut histogram| {
                mem::swap(&mut histogram.data_points, &mut producer_histogram_scratch);
                histogram.updated_at = Instant::now();
            })
            .log_error_msg("GUI histogram snapshot busy; dropping best-effort visualization update")
            .ok();

        drop(producer_histogram_scratch);
        self.request_repaint();
    }

    fn receive_daw_bpm(&self, bpm: f32) {
        if let Some(state) = self.state.upgrade() {
            state.daw_bpm.store(bpm, Ordering::Relaxed);
        }
    }
}

#[derive(Clone)]
pub struct GuiContextHandle {
    state: Weak<DisplayState>,
}

impl GuiContextHandle {
    pub(crate) fn new(state: Weak<DisplayState>) -> Self {
        Self { state }
    }

    pub fn request_repaint(&self) {
        let Some(state) = self.state.upgrade() else {
            return;
        };
        let Ok(context) = state.context.try_borrow() else {
            return;
        };
        if let Some(context) = context.as_ref() {
            context.request_repaint();
        }
    }

    #[must_use]
    pub fn egui_wants_keyboard_input(&self) -> bool {
        let Some(state) = self.state.upgrade() else {
            return false;
        };
        let Ok(context) = state.context.try_borrow() else {
            return false;
        };
        context.as_ref().is_some_and(Context::egui_wants_keyboard_input)
    }
}
