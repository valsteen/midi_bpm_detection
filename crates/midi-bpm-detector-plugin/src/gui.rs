use crate::{
    MidiBpmDetector, MidiBpmDetectorParams,
    config::{Config, LiveConfig},
};
use crossbeam::atomic::AtomicCell;
use gui::{BPMDetectionGUI, BPMDetectionParameters, GuiRemote, create_gui};
use nih_plug::prelude::{AsyncExecutor, ParamSetter};
use nih_plug_egui::{
    EguiState,
    egui::{Context, mutex::RwLock},
};
use std::sync::{Arc, atomic::Ordering};

use sync::ArcAtomicBool;

pub struct GuiEditor {
    pub editor_state: Arc<EguiState>,
    pub bpm_detection_gui: Option<BPMDetectionGUI<LiveConfig>>,
    pub gui_remote_receiver: Arc<AtomicCell<Option<GuiRemote>>>,
    pub force_evaluate_bpm_detection: ArcAtomicBool,
    pub config: Arc<RwLock<Config>>,
    pub gui_must_update_config: ArcAtomicBool,
    pub params: Arc<MidiBpmDetectorParams>,
}

impl GuiEditor {
    pub fn build(&mut self, egui_ctx: &Context, async_executor: AsyncExecutor<MidiBpmDetector>) {
        let config = self.config.read().clone();
        let live_config = LiveConfig::new(
            config.clone(),
            self.config.clone(),
            async_executor,
            self.force_evaluate_bpm_detection.clone(),
            self.params.clone(),
        );
        let send_tempo_changed = live_config.send_tempo_changed.clone();
        let (gui_remote, gui_builder) = create_gui(live_config);
        gui_remote.receive_keystrokes({
            let send_tempo = config.send_tempo.clone();
            Box::new(move |key| {
                if key.to_lowercase() == "t" {
                    send_tempo.fetch_xor(true, Ordering::Acquire);
                    send_tempo_changed.store(true, Ordering::Release);
                }
            })
        });
        let gui = gui_builder.build(egui_ctx.clone());
        self.bpm_detection_gui = Some(gui);
        self.gui_remote_receiver.store(Some(gui_remote));
        self.force_evaluate_bpm_detection.store(true, Ordering::Relaxed);
    }

    pub fn update(&mut self, setter: &ParamSetter, egui_ctx: &Context) {
        let should_drop = match (self.editor_state.is_open(), &mut self.bpm_detection_gui) {
            (true, Some(bpm_detection_gui)) => {
                if bpm_detection_gui.live_parameters.send_tempo_changed.fetch_xor(true, Ordering::Relaxed) {
                    let send_tempo = bpm_detection_gui.live_parameters.get_send_tempo();
                    setter.begin_set_parameter(&self.params.send_tempo);
                    setter.set_parameter(&self.params.send_tempo, send_tempo);
                    setter.end_set_parameter(&self.params.send_tempo);
                }

                bpm_detection_gui.live_parameters.apply_delayed_updates();

                if self.gui_must_update_config.take(Ordering::Relaxed) {
                    bpm_detection_gui.live_parameters.config = self.config.read().clone();
                }

                // error may happen if corresponding remote was dropped
                if bpm_detection_gui.update(egui_ctx).is_ok() {
                    bpm_detection_gui.live_parameters.apply_changes_to_daw_parameters(setter);
                    false
                } else {
                    true
                }
            }
            #[allow(clippy::match_same_arms)]
            (true, None) => {
                // editor is open but the gui is not yet there
                false
            }
            (false, None) => {
                // editor is closed, the gui is gone, don't do anything
                false
            }
            (false, Some(_)) => {
                // window is closed, free up resources
                true
            }
        };

        if should_drop {
            self.bpm_detection_gui = None;
        }
    }
}
