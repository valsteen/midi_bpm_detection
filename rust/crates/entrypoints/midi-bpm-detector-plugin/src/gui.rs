use std::sync::{Arc, atomic::Ordering};

use bpm_detection_config::Settings;
use crossbeam::atomic::AtomicCell;
use gui::{BPMDetectionGUI, EditableSettings, GuiChanges, GuiLifecycleOwner, create_gui, eframe::egui};
use nice_plug::prelude::ParamSetter;
use nice_plug_egui::EguiState;
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
        merge_host_changes(&mut state.draft, &state.previous_host, &current_host);
        state.previous_host = current_host;

        let before = state.draft.clone();
        let shortcut_changed = toggle_send_tempo_from_shortcut(ui, &mut state.draft);
        state.gui.prepare();
        let mut changes = egui::CentralPanel::default().show(ui, |ui| state.gui.show(ui, &mut state.draft)).inner;
        changes.send_tempo |= shortcut_changed;
        commit_gui_edits(&self.params, setter, &before, &state.draft, changes);
    }
}

fn toggle_send_tempo_from_shortcut(ui: &egui::Ui, draft: &mut EditableSettings) -> bool {
    if !ui.input(|input| input.key_pressed(egui::Key::T)) {
        return false;
    }
    let Some(send_tempo) = draft.send_tempo.as_mut() else {
        return false;
    };
    *send_tempo = !*send_tempo;
    true
}

fn merge_host_changes(draft: &mut EditableSettings, previous: &EditableSettings, current: &EditableSettings) {
    let draft_gui = &mut draft.bpm.gui_config;
    let previous_gui = &previous.bpm.gui_config;
    let current_gui = &current.bpm.gui_config;
    if current_gui.interpolation_duration != previous_gui.interpolation_duration {
        draft_gui.interpolation_duration = current_gui.interpolation_duration;
    }
    if !f32::eq(&current_gui.interpolation_curve, &previous_gui.interpolation_curve) {
        draft_gui.interpolation_curve = current_gui.interpolation_curve;
    }

    let draft_static = &mut draft.bpm.static_bpm_detection_config;
    let previous_static = &previous.bpm.static_bpm_detection_config;
    let current_static = &current.bpm.static_bpm_detection_config;
    if !f32::eq(&current_static.bpm_center, &previous_static.bpm_center) {
        draft_static.bpm_center = current_static.bpm_center;
    }
    if current_static.bpm_range != previous_static.bpm_range {
        draft_static.bpm_range = current_static.bpm_range;
    }
    if current_static.sample_rate != previous_static.sample_rate {
        draft_static.sample_rate = current_static.sample_rate;
    }
    if !f64::eq(&current_static.normal_distribution.std_dev, &previous_static.normal_distribution.std_dev) {
        draft_static.normal_distribution.std_dev = current_static.normal_distribution.std_dev;
    }
    if !f32::eq(&current_static.normal_distribution.resolution, &previous_static.normal_distribution.resolution) {
        draft_static.normal_distribution.resolution = current_static.normal_distribution.resolution;
    }
    if !f32::eq(&current_static.normal_distribution.cutoff, &previous_static.normal_distribution.cutoff) {
        draft_static.normal_distribution.cutoff = current_static.normal_distribution.cutoff;
    }
    if !f32::eq(&current_static.normal_distribution.factor, &previous_static.normal_distribution.factor) {
        draft_static.normal_distribution.factor = current_static.normal_distribution.factor;
    }

    let draft_dynamic = &mut draft.bpm.dynamic_bpm_detection_config;
    let previous_dynamic = &previous.bpm.dynamic_bpm_detection_config;
    let current_dynamic = &current.bpm.dynamic_bpm_detection_config;
    if current_dynamic.beats_lookback != previous_dynamic.beats_lookback {
        draft_dynamic.beats_lookback = current_dynamic.beats_lookback;
    }
    if current_dynamic.normal_distribution_weight != previous_dynamic.normal_distribution_weight {
        draft_dynamic.normal_distribution_weight = current_dynamic.normal_distribution_weight;
    }
    if current_dynamic.time_distance_weight != previous_dynamic.time_distance_weight {
        draft_dynamic.time_distance_weight = current_dynamic.time_distance_weight;
    }
    if current_dynamic.velocity_current_note_weight != previous_dynamic.velocity_current_note_weight {
        draft_dynamic.velocity_current_note_weight = current_dynamic.velocity_current_note_weight;
    }
    if current_dynamic.velocity_note_from_weight != previous_dynamic.velocity_note_from_weight {
        draft_dynamic.velocity_note_from_weight = current_dynamic.velocity_note_from_weight;
    }
    if current_dynamic.in_beat_range_weight != previous_dynamic.in_beat_range_weight {
        draft_dynamic.in_beat_range_weight = current_dynamic.in_beat_range_weight;
    }
    if current_dynamic.multiplier_weight != previous_dynamic.multiplier_weight {
        draft_dynamic.multiplier_weight = current_dynamic.multiplier_weight;
    }
    if current_dynamic.subdivision_weight != previous_dynamic.subdivision_weight {
        draft_dynamic.subdivision_weight = current_dynamic.subdivision_weight;
    }
    if current_dynamic.octave_distance_weight != previous_dynamic.octave_distance_weight {
        draft_dynamic.octave_distance_weight = current_dynamic.octave_distance_weight;
    }
    if current_dynamic.pitch_distance_weight != previous_dynamic.pitch_distance_weight {
        draft_dynamic.pitch_distance_weight = current_dynamic.pitch_distance_weight;
    }
    if current_dynamic.high_tempo_bias_weight != previous_dynamic.high_tempo_bias_weight {
        draft_dynamic.high_tempo_bias_weight = current_dynamic.high_tempo_bias_weight;
    }

    if current.send_tempo != previous.send_tempo {
        draft.send_tempo = current.send_tempo;
    }
}

fn commit_gui_edits(
    params: &MidiBpmDetectorParams,
    setter: &ParamSetter<'_>,
    before: &EditableSettings,
    after: &EditableSettings,
    changes: GuiChanges,
) {
    let mut mirrored = before.bpm.clone();

    if changes.gui {
        let before = &before.bpm.gui_config;
        let after = &after.bpm.gui_config;
        let mirrored = &mut mirrored.gui_config;
        if after.interpolation_duration != before.interpolation_duration {
            params.gui_params.mirror_interpolation_duration(mirrored, after.interpolation_duration, setter);
        }
        if !f32::eq(&after.interpolation_curve, &before.interpolation_curve) {
            params.gui_params.mirror_interpolation_curve(mirrored, after.interpolation_curve, setter);
        }
    }

    if changes.static_detection {
        commit_static_gui_edits(params, setter, before, after, &mut mirrored);
    }
    if changes.dynamic_detection {
        commit_dynamic_gui_edits(params, setter, before, after, &mut mirrored);
    }
    if changes.send_tempo && after.send_tempo != before.send_tempo {
        let enabled = after.send_tempo.expect("plugin supports send tempo");
        setter.begin_set_parameter(&params.send_tempo);
        setter.set_parameter(&params.send_tempo, enabled);
        setter.end_set_parameter(&params.send_tempo);
    }
}

fn commit_static_gui_edits(
    params: &MidiBpmDetectorParams,
    setter: &ParamSetter<'_>,
    before: &EditableSettings,
    after: &EditableSettings,
    mirrored: &mut Settings,
) {
    let before = &before.bpm.static_bpm_detection_config;
    let after = &after.bpm.static_bpm_detection_config;
    let mirrored = &mut mirrored.static_bpm_detection_config;

    if !f32::eq(&after.bpm_center, &before.bpm_center) {
        params.static_params.mirror_bpm_center(mirrored, after.bpm_center, setter);
    }
    if after.bpm_range != before.bpm_range {
        params.static_params.mirror_bpm_range(mirrored, after.bpm_range, setter);
    }
    if after.sample_rate != before.sample_rate {
        params.static_params.mirror_sample_rate(mirrored, after.sample_rate, setter);
    }

    let before_normal = &before.normal_distribution;
    let after_normal = &after.normal_distribution;
    let mirrored_normal = &mut mirrored.normal_distribution;
    if !f64::eq(&after_normal.std_dev, &before_normal.std_dev) {
        params.static_params.normal_distribution.mirror_std_dev(mirrored_normal, after_normal.std_dev, setter);
    }
    if !f32::eq(&after_normal.resolution, &before_normal.resolution) {
        params.static_params.normal_distribution.mirror_resolution(mirrored_normal, after_normal.resolution, setter);
    }
    if !f32::eq(&after_normal.cutoff, &before_normal.cutoff) {
        params.static_params.normal_distribution.mirror_cutoff(mirrored_normal, after_normal.cutoff, setter);
    }
    if !f32::eq(&after_normal.factor, &before_normal.factor) {
        params.static_params.normal_distribution.mirror_factor(mirrored_normal, after_normal.factor, setter);
    }
}

fn commit_dynamic_gui_edits(
    params: &MidiBpmDetectorParams,
    setter: &ParamSetter<'_>,
    before: &EditableSettings,
    after: &EditableSettings,
    mirrored: &mut Settings,
) {
    let before = &before.bpm.dynamic_bpm_detection_config;
    let after = &after.bpm.dynamic_bpm_detection_config;
    let mirrored = &mut mirrored.dynamic_bpm_detection_config;

    if after.beats_lookback != before.beats_lookback {
        params.dynamic_params.mirror_beats_lookback(mirrored, after.beats_lookback, setter);
    }
    if after.normal_distribution_weight != before.normal_distribution_weight {
        params.dynamic_params.mirror_normal_distribution_weight(mirrored, after.normal_distribution_weight, setter);
    }
    if after.time_distance_weight != before.time_distance_weight {
        params.dynamic_params.mirror_time_distance_weight(mirrored, after.time_distance_weight, setter);
    }
    if after.velocity_current_note_weight != before.velocity_current_note_weight {
        params.dynamic_params.mirror_velocity_current_note_weight(mirrored, after.velocity_current_note_weight, setter);
    }
    if after.velocity_note_from_weight != before.velocity_note_from_weight {
        params.dynamic_params.mirror_velocity_note_from_weight(mirrored, after.velocity_note_from_weight, setter);
    }
    if after.in_beat_range_weight != before.in_beat_range_weight {
        params.dynamic_params.mirror_in_beat_range_weight(mirrored, after.in_beat_range_weight, setter);
    }
    if after.multiplier_weight != before.multiplier_weight {
        params.dynamic_params.mirror_multiplier_weight(mirrored, after.multiplier_weight, setter);
    }
    if after.subdivision_weight != before.subdivision_weight {
        params.dynamic_params.mirror_subdivision_weight(mirrored, after.subdivision_weight, setter);
    }
    if after.octave_distance_weight != before.octave_distance_weight {
        params.dynamic_params.mirror_octave_distance_weight(mirrored, after.octave_distance_weight, setter);
    }
    if after.pitch_distance_weight != before.pitch_distance_weight {
        params.dynamic_params.mirror_pitch_distance_weight(mirrored, after.pitch_distance_weight, setter);
    }
    if after.high_tempo_bias_weight != before.high_tempo_bias_weight {
        params.dynamic_params.mirror_high_tempo_bias_weight(mirrored, after.high_tempo_bias_weight, setter);
    }
}

#[cfg(test)]
#[path = "../tests/unit/gui.rs"]
mod tests;
