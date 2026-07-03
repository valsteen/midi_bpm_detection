use std::sync::{Arc, atomic::Ordering};

use crossbeam::atomic::AtomicCell;
use gui::{BPMDetectionApp, BPMDetectionConfig, GuiRemote, create_gui};
use nih_plug::prelude::{AsyncExecutor, ParamSetter};
use nih_plug_egui::{EguiState, egui::Context};
use sync::{ArcAtomicBool, RwLock};

use crate::{
    MidiBpmDetector, MidiBpmDetectorParams,
    bpm_detector_configuration::{BaseConfig, LiveConfig},
    plugin_config::PluginConfig,
};

pub struct GuiEditor {
    pub editor_state: Arc<EguiState>,
    pub bpm_detection_app: Option<BPMDetectionApp<BaseConfig>>,
    pub gui_remote_handoff: Arc<AtomicCell<Option<GuiRemote>>>,
    pub force_evaluate_bpm_detection: ArcAtomicBool,
    pub gui_task_config: Arc<RwLock<PluginConfig>>,
    pub gui_must_update_config: ArcAtomicBool,
    pub params: Arc<MidiBpmDetectorParams>,
}

impl GuiEditor {
    pub fn build(&mut self, egui_ctx: &Context, async_executor: AsyncExecutor<MidiBpmDetector>) {
        let config = self.gui_task_config.read().clone();
        let live_config = BaseConfig::new(
            config.clone(),
            self.gui_task_config.clone(),
            async_executor,
            self.force_evaluate_bpm_detection.clone(),
            self.params.clone(),
        );
        let (gui_remote, gui_builder) = create_gui(live_config);
        gui_remote.receive_keystrokes({
            let send_tempo = config.send_tempo.clone();
            Box::new(move |key| {
                if key.to_lowercase() == "t" {
                    send_tempo.toggle_from_shortcut();
                }
            })
        });
        let bpm_detection_app = gui_builder.build(egui_ctx.clone());
        self.bpm_detection_app = Some(bpm_detection_app);
        self.gui_remote_handoff.store(Some(gui_remote));
        self.force_evaluate_bpm_detection.store(true, Ordering::Relaxed);
    }

    pub fn update(&mut self, param_setter: &ParamSetter, egui_ctx: &Context) {
        if !self.editor_state.is_open() {
            if self.bpm_detection_app.is_some() {
                // window is closed, free up resources
                self.bpm_detection_app = None;
            } else {
                // editor is closed, the gui is gone, don't do anything
            }
            return;
        }

        let Some(BPMDetectionApp { base_config, bpm_detection_gui }) = self.bpm_detection_app.as_mut() else {
            // editor is open but the gui is not yet there
            return;
        };

        let mut live_config = LiveConfig { base_config, param_setter };
        if live_config.base_config.config.send_tempo.take_host_param_update_request() {
            let send_tempo = live_config.get_send_tempo();
            param_setter.begin_set_parameter(&self.params.send_tempo);
            param_setter.set_parameter(&self.params.send_tempo, send_tempo);
            param_setter.end_set_parameter(&self.params.send_tempo);
        }
        live_config.base_config.apply_delayed_updates();

        if self.gui_must_update_config.take(Ordering::Relaxed) {
            live_config.base_config.config = self.gui_task_config.read().clone();
        }

        // error may happen if corresponding remote was dropped
        if bpm_detection_gui.update_context(egui_ctx, &mut live_config).is_err() {
            self.bpm_detection_app = None;
            return;
        }

        if live_config.base_config.has_config_changes_via_ui {
            let mut config = self.gui_task_config.write();
            *config = live_config.base_config.config.clone();
            live_config.base_config.has_config_changes_via_ui = false;
        }
    }
}
