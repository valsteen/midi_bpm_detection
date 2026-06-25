use std::sync::Arc;

use atomic_refcell::AtomicRefCell;
use eframe::egui::Context;

use crate::{BPMDetectionApp, app::BPMDetectionGUI};

pub struct AppBuilderShell {
    context_receiver: Arc<AtomicRefCell<Option<Context>>>,
    bpm_detection_gui: BPMDetectionGUI,
}

impl AppBuilderShell {
    pub(crate) fn new(
        context_receiver: Arc<AtomicRefCell<Option<Context>>>,
        bpm_detection_gui: BPMDetectionGUI,
    ) -> Self {
        Self { context_receiver, bpm_detection_gui }
    }

    pub fn with_config<Config>(self, base_config: Config) -> AppBuilder<Config> {
        AppBuilder::new(self.context_receiver, self.bpm_detection_gui, base_config)
    }
}

pub struct AppBuilder<Config> {
    context_receiver: Arc<AtomicRefCell<Option<Context>>>,
    bpm_detection_gui: BPMDetectionGUI,
    base_config: Config,
}

impl<Config> AppBuilder<Config> {
    pub fn build(self, context: Context) -> BPMDetectionApp<Config> {
        self.context_receiver.borrow_mut().replace(context);
        BPMDetectionApp { base_config: self.base_config, bpm_detection_gui: self.bpm_detection_gui }
    }

    pub fn new(
        context_receiver: Arc<AtomicRefCell<Option<Context>>>,
        bpm_detection_gui: BPMDetectionGUI,
        base_config: Config,
    ) -> Self {
        Self { context_receiver, bpm_detection_gui, base_config }
    }
}
