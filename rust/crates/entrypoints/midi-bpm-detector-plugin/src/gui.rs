use std::sync::{Arc, atomic::Ordering};

use bpm_detection_config::{DynamicBPMDetectionConfig, GUIConfig, Settings, StaticBPMDetectionConfig};
use crossbeam::atomic::AtomicCell;
use gui::{BPMDetectionGUI, EditableSettings, GuiChanges, GuiLifecycleOwner, create_gui, eframe::egui};
use nice_plug::prelude::ParamSetter;
use nice_plug_egui::EguiState;
use parameter::MergeChangedFields;
use parameter_nice_plug::MirrorChangedConfig;
use sync::ArcAtomicBool;

use crate::{MidiBpmDetectorParams, task_executor::GuiOutputHandoff};

pub struct GuiEditor {
    pub editor_state: Arc<EguiState>,
    pub gui_state: Option<PluginGuiState>,
    pub gui_output_handoff: Arc<AtomicCell<Option<GuiOutputHandoff>>>,
    pub force_evaluate_bpm_detection: ArcAtomicBool,
    pub params: Arc<MidiBpmDetectorParams>,
}

pub struct PluginGuiState {
    gui: BPMDetectionGUI,
    draft: EditableSettings,
    previous_host: EditableSettings,
}

impl GuiEditor {
    pub fn build(&mut self, egui_ctx: &egui::Context) {
        let (publisher, context, mut gui) = create_gui();
        gui.attach_context(egui_ctx, GuiLifecycleOwner::ParentRuntime);
        let host = self.params.read_editable_settings();
        self.gui_state = Some(PluginGuiState { gui, draft: host.clone(), previous_host: host });
        self.gui_output_handoff.store(Some((publisher, context)));
        self.force_evaluate_bpm_detection.store(true, Ordering::Relaxed);
    }

    pub fn update(&mut self, setter: &ParamSetter<'_>, ui: &mut egui::Ui) {
        if !self.editor_state.is_open() {
            self.gui_state = None;
            return;
        }

        let Some(state) = self.gui_state.as_mut() else {
            return;
        };

        let current_host = self.params.read_editable_settings();
        let merged = merge_host_changes(&state.draft, &state.previous_host, &current_host);
        state.draft = merged;
        state.previous_host = current_host;

        let before = state.draft.clone();
        let shortcut_changed = toggle_send_tempo_from_shortcut(ui, &mut state.draft);
        state.gui.prepare();
        let mut changes = egui::CentralPanel::default().show(ui, |ui| state.gui.show(ui, &mut state.draft)).inner;
        changes.send_tempo |= shortcut_changed;
        let committed = commit_gui_edits(&self.params, setter, &before, &state.draft, changes);
        state.draft = committed;
    }
}

/// Applies the plugin-only keyboard shortcut to the GUI-owned draft.
///
/// The returned flag joins the shared GUI receipt so the updated draft value is
/// sent through the same host-parameter commit path as the visible toggle.
fn toggle_send_tempo_from_shortcut(ui: &egui::Ui, draft: &mut EditableSettings) -> bool {
    if !ui.input(|input| input.key_pressed(egui::Key::T)) {
        return false;
    }
    let Some(send_tempo) = draft.send_tempo else {
        return false;
    };
    draft.send_tempo = Some(!send_tempo);
    true
}

/// Reconciles the persistent GUI draft with two consecutive host snapshots.
///
/// `draft` is the state retained by the editor and may contain GUI edits that
/// have been sent to the host but are not visible in host readback yet.
/// `previous` and `current` come from consecutive calls to
/// [`MidiBpmDetectorParams::read_editable_settings`]. A field whose current host
/// value differs from its previous host value is newly observed host data and
/// replaces that draft field. Otherwise the draft field is preserved.
///
/// The returned settings become the editor draft before the current frame is
/// rendered. The caller advances `previous_host` separately to the exact
/// `current` snapshot; requesting a host setter never advances it directly.
fn merge_host_changes(
    draft: &EditableSettings,
    previous: &EditableSettings,
    current: &EditableSettings,
) -> EditableSettings {
    let gui_config =
        GUIConfig::merge_changed_fields(&draft.bpm.gui_config, &previous.bpm.gui_config, &current.bpm.gui_config);
    let static_bpm_detection_config = StaticBPMDetectionConfig::merge_changed_fields(
        &draft.bpm.static_bpm_detection_config,
        &previous.bpm.static_bpm_detection_config,
        &current.bpm.static_bpm_detection_config,
    );
    let dynamic_bpm_detection_config = DynamicBPMDetectionConfig::merge_changed_fields(
        &draft.bpm.dynamic_bpm_detection_config,
        &previous.bpm.dynamic_bpm_detection_config,
        &current.bpm.dynamic_bpm_detection_config,
    );
    let send_tempo = if current.send_tempo == previous.send_tempo { draft.send_tempo } else { current.send_tempo };

    EditableSettings {
        bpm: Settings { gui_config, dynamic_bpm_detection_config, static_bpm_detection_config },
        send_tempo,
    }
}

/// Sends this frame's GUI edits to the plugin host and returns the next GUI draft.
///
/// `before` is the reconciled draft captured immediately before rendering.
/// `after` is that draft after the shared GUI and plugin shortcut have edited it.
/// `changes` is the fixed-size receipt produced while rendering; it limits
/// detailed comparisons and host setter calls to groups touched in this frame.
///
/// Changed fields are submitted through their typed nice-plug setters. The
/// returned complete settings preserve all `after` values and are assigned back
/// to `PluginGuiState::draft`. They do not update `previous_host`; only a later
/// parameter readback is allowed to acknowledge a host value.
fn commit_gui_edits(
    params: &MidiBpmDetectorParams,
    setter: &ParamSetter<'_>,
    before: &EditableSettings,
    after: &EditableSettings,
    changes: GuiChanges,
) -> EditableSettings {
    let gui_config = if changes.gui {
        params.gui_params.mirror_changed_config(&before.bpm.gui_config, &after.bpm.gui_config, setter)
    } else {
        after.bpm.gui_config.clone()
    };
    let static_bpm_detection_config = if changes.static_detection {
        params.static_params.mirror_changed_config(
            &before.bpm.static_bpm_detection_config,
            &after.bpm.static_bpm_detection_config,
            setter,
        )
    } else {
        after.bpm.static_bpm_detection_config.clone()
    };
    let dynamic_bpm_detection_config = if changes.dynamic_detection {
        params.dynamic_params.mirror_changed_config(
            &before.bpm.dynamic_bpm_detection_config,
            &after.bpm.dynamic_bpm_detection_config,
            setter,
        )
    } else {
        after.bpm.dynamic_bpm_detection_config.clone()
    };
    if changes.send_tempo && after.send_tempo != before.send_tempo {
        let enabled = after.send_tempo.expect("plugin supports send tempo");
        setter.begin_set_parameter(&params.send_tempo);
        setter.set_parameter(&params.send_tempo, enabled);
        setter.end_set_parameter(&params.send_tempo);
    }

    EditableSettings {
        bpm: Settings { gui_config, dynamic_bpm_detection_config, static_bpm_detection_config },
        send_tempo: after.send_tempo,
    }
}

#[cfg(test)]
#[path = "../tests/unit/gui.rs"]
mod tests;
