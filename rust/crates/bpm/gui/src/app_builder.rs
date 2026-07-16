use std::sync::Arc;

use atomic_refcell::AtomicRefCell;
use eframe::egui::Context;

use crate::{BPMDetectionApp, app::BPMDetectionGUI};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GuiLifecycleOwner {
    ApplicationRuntime,
    ParentRuntime,
}

fn configure_quit_shortcuts(context: &Context, lifecycle_owner: GuiLifecycleOwner) {
    if lifecycle_owner == GuiLifecycleOwner::ParentRuntime {
        context.options_mut(|options| options.quit_shortcuts.clear());
    }
}

pub struct AppBuilderShell {
    context_receiver: Arc<AtomicRefCell<Option<Context>>>,
    bpm_detection_gui: BPMDetectionGUI,
    lifecycle_owner: GuiLifecycleOwner,
}

impl AppBuilderShell {
    pub(crate) fn new(
        context_receiver: Arc<AtomicRefCell<Option<Context>>>,
        bpm_detection_gui: BPMDetectionGUI,
        lifecycle_owner: GuiLifecycleOwner,
    ) -> Self {
        Self { context_receiver, bpm_detection_gui, lifecycle_owner }
    }

    pub fn with_config<Config>(self, base_config: Config) -> AppBuilder<Config> {
        AppBuilder::new(self.context_receiver, self.bpm_detection_gui, base_config, self.lifecycle_owner)
    }
}

pub struct AppBuilder<Config> {
    context_receiver: Arc<AtomicRefCell<Option<Context>>>,
    bpm_detection_gui: BPMDetectionGUI,
    base_config: Config,
    lifecycle_owner: GuiLifecycleOwner,
}

impl<Config> AppBuilder<Config> {
    pub fn build(self, context: Context) -> BPMDetectionApp<Config> {
        configure_quit_shortcuts(&context, self.lifecycle_owner);
        self.context_receiver.borrow_mut().replace(context);
        BPMDetectionApp { base_config: self.base_config, bpm_detection_gui: self.bpm_detection_gui }
    }

    pub fn new(
        context_receiver: Arc<AtomicRefCell<Option<Context>>>,
        bpm_detection_gui: BPMDetectionGUI,
        base_config: Config,
        lifecycle_owner: GuiLifecycleOwner,
    ) -> Self {
        Self { context_receiver, bpm_detection_gui, base_config, lifecycle_owner }
    }
}

#[cfg(test)]
#[path = "../tests/unit/app_builder.rs"]
mod tests;
