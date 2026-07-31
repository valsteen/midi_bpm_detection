use std::sync::{Arc, atomic::Ordering};

use bpm_detection_config::{
    DynamicBPMDetectionConfig, GUIConfig, NormalDistributionConfig, Settings, StaticBPMDetectionConfig,
};
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
    let gui_config = merge_gui_config(&draft.bpm.gui_config, &previous.bpm.gui_config, &current.bpm.gui_config);
    let static_bpm_detection_config = merge_static_config(
        &draft.bpm.static_bpm_detection_config,
        &previous.bpm.static_bpm_detection_config,
        &current.bpm.static_bpm_detection_config,
    );
    let dynamic_bpm_detection_config = merge_dynamic_config(
        &draft.bpm.dynamic_bpm_detection_config,
        &previous.bpm.dynamic_bpm_detection_config,
        &current.bpm.dynamic_bpm_detection_config,
    );
    let send_tempo = merge_host_value(draft.send_tempo, previous.send_tempo, current.send_tempo);

    EditableSettings {
        bpm: Settings { gui_config, dynamic_bpm_detection_config, static_bpm_detection_config },
        send_tempo,
    }
}

/// Reconciles every GUI-display parameter and returns a complete group.
///
/// The inputs are the corresponding groups from the retained draft and the two
/// host snapshots. The result is installed in the merged `EditableSettings`.
fn merge_gui_config(draft: &GUIConfig, previous: &GUIConfig, current: &GUIConfig) -> GUIConfig {
    let interpolation_duration =
        merge_host_value(draft.interpolation_duration, previous.interpolation_duration, current.interpolation_duration);
    let interpolation_curve =
        merge_host_value(draft.interpolation_curve, previous.interpolation_curve, current.interpolation_curve);

    GUIConfig { interpolation_duration, interpolation_curve }
}

/// Reconciles every static-detection parameter and returns a complete group.
///
/// The inputs are the corresponding groups from the retained draft and the two
/// host snapshots. The returned group includes the separately reconciled nested
/// normal-distribution configuration.
fn merge_static_config(
    draft: &StaticBPMDetectionConfig,
    previous: &StaticBPMDetectionConfig,
    current: &StaticBPMDetectionConfig,
) -> StaticBPMDetectionConfig {
    let bpm_center = merge_host_value(draft.bpm_center, previous.bpm_center, current.bpm_center);
    let bpm_range = merge_host_value(draft.bpm_range, previous.bpm_range, current.bpm_range);
    let sample_rate = merge_host_value(draft.sample_rate, previous.sample_rate, current.sample_rate);
    let normal_distribution = merge_normal_distribution_config(
        &draft.normal_distribution,
        &previous.normal_distribution,
        &current.normal_distribution,
    );

    StaticBPMDetectionConfig { bpm_center, bpm_range, sample_rate, normal_distribution }
}

/// Reconciles every nested normal-distribution parameter and returns a complete group.
///
/// The inputs come from the enclosing static groups. The result becomes the
/// `normal_distribution` field of the merged static configuration.
fn merge_normal_distribution_config(
    draft: &NormalDistributionConfig,
    previous: &NormalDistributionConfig,
    current: &NormalDistributionConfig,
) -> NormalDistributionConfig {
    let std_dev = merge_host_value(draft.std_dev, previous.std_dev, current.std_dev);
    let resolution = merge_host_value(draft.resolution, previous.resolution, current.resolution);
    let cutoff = merge_host_value(draft.cutoff, previous.cutoff, current.cutoff);
    let factor = merge_host_value(draft.factor, previous.factor, current.factor);

    NormalDistributionConfig { std_dev, resolution, cutoff, factor }
}

/// Reconciles every dynamic-detection parameter and returns a complete group.
///
/// The inputs are the corresponding groups from the retained draft and the two
/// host snapshots. The result is installed in the merged `EditableSettings`.
fn merge_dynamic_config(
    draft: &DynamicBPMDetectionConfig,
    previous: &DynamicBPMDetectionConfig,
    current: &DynamicBPMDetectionConfig,
) -> DynamicBPMDetectionConfig {
    let beats_lookback = merge_host_value(draft.beats_lookback, previous.beats_lookback, current.beats_lookback);
    let normal_distribution_weight = merge_host_value(
        draft.normal_distribution_weight,
        previous.normal_distribution_weight,
        current.normal_distribution_weight,
    );
    let time_distance_weight =
        merge_host_value(draft.time_distance_weight, previous.time_distance_weight, current.time_distance_weight);
    let velocity_current_note_weight = merge_host_value(
        draft.velocity_current_note_weight,
        previous.velocity_current_note_weight,
        current.velocity_current_note_weight,
    );
    let velocity_note_from_weight = merge_host_value(
        draft.velocity_note_from_weight,
        previous.velocity_note_from_weight,
        current.velocity_note_from_weight,
    );
    let in_beat_range_weight =
        merge_host_value(draft.in_beat_range_weight, previous.in_beat_range_weight, current.in_beat_range_weight);
    let multiplier_weight =
        merge_host_value(draft.multiplier_weight, previous.multiplier_weight, current.multiplier_weight);
    let subdivision_weight =
        merge_host_value(draft.subdivision_weight, previous.subdivision_weight, current.subdivision_weight);
    let octave_distance_weight =
        merge_host_value(draft.octave_distance_weight, previous.octave_distance_weight, current.octave_distance_weight);
    let pitch_distance_weight =
        merge_host_value(draft.pitch_distance_weight, previous.pitch_distance_weight, current.pitch_distance_weight);
    let high_tempo_bias_weight =
        merge_host_value(draft.high_tempo_bias_weight, previous.high_tempo_bias_weight, current.high_tempo_bias_weight);

    DynamicBPMDetectionConfig {
        beats_lookback,
        normal_distribution_weight,
        time_distance_weight,
        velocity_current_note_weight,
        velocity_note_from_weight,
        in_beat_range_weight,
        multiplier_weight,
        subdivision_weight,
        octave_distance_weight,
        pitch_distance_weight,
        high_tempo_bias_weight,
    }
}

/// Keeps a pending GUI value unless host readback changed since the previous snapshot.
fn merge_host_value<Value: Copy + PartialEq>(draft: Value, previous: Value, current: Value) -> Value {
    if current == previous { draft } else { current }
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
        commit_gui_config(params, setter, &before.bpm.gui_config, &after.bpm.gui_config)
    } else {
        after.bpm.gui_config.clone()
    };
    let static_bpm_detection_config = if changes.static_detection {
        commit_static_gui_edits(
            params,
            setter,
            &before.bpm.static_bpm_detection_config,
            &after.bpm.static_bpm_detection_config,
        )
    } else {
        after.bpm.static_bpm_detection_config.clone()
    };
    let dynamic_bpm_detection_config = if changes.dynamic_detection {
        commit_dynamic_gui_edits(
            params,
            setter,
            &before.bpm.dynamic_bpm_detection_config,
            &after.bpm.dynamic_bpm_detection_config,
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

/// Commits changed GUI-display fields and returns the complete post-render group.
///
/// The local mirror starts from `before` because generated host adapters need
/// the prior typed value while issuing setter operations. It is not returned;
/// the exhaustive result is built from the post-render field values.
fn commit_gui_config(
    params: &MidiBpmDetectorParams,
    setter: &ParamSetter<'_>,
    before: &GUIConfig,
    after: &GUIConfig,
) -> GUIConfig {
    let mut mirrored = before.clone();
    let interpolation_duration =
        commit_gui_value(before.interpolation_duration, after.interpolation_duration, |interpolation_duration| {
            params.gui_params.mirror_interpolation_duration(&mut mirrored, interpolation_duration, setter);
        });
    let interpolation_curve =
        commit_gui_value(before.interpolation_curve, after.interpolation_curve, |interpolation_curve| {
            params.gui_params.mirror_interpolation_curve(&mut mirrored, interpolation_curve, setter);
        });

    GUIConfig { interpolation_duration, interpolation_curve }
}

/// Commits changed static-detection fields and returns the complete post-render group.
///
/// `before` and `after` are the static groups captured before and after rendering.
/// A local clone of `before` supplies prior typed values to host adapters. The
/// result becomes the static group in the next GUI draft.
fn commit_static_gui_edits(
    params: &MidiBpmDetectorParams,
    setter: &ParamSetter<'_>,
    before: &StaticBPMDetectionConfig,
    after: &StaticBPMDetectionConfig,
) -> StaticBPMDetectionConfig {
    let mut mirrored = before.clone();
    let bpm_center = commit_gui_value(before.bpm_center, after.bpm_center, |bpm_center| {
        params.static_params.mirror_bpm_center(&mut mirrored, bpm_center, setter);
    });
    let bpm_range = commit_gui_value(before.bpm_range, after.bpm_range, |bpm_range| {
        params.static_params.mirror_bpm_range(&mut mirrored, bpm_range, setter);
    });
    let sample_rate = commit_gui_value(before.sample_rate, after.sample_rate, |sample_rate| {
        params.static_params.mirror_sample_rate(&mut mirrored, sample_rate, setter);
    });
    let normal_distribution =
        commit_normal_distribution_gui_edits(params, setter, &before.normal_distribution, &after.normal_distribution);

    StaticBPMDetectionConfig { bpm_center, bpm_range, sample_rate, normal_distribution }
}

/// Commits changed nested normal-distribution fields and returns the complete post-render group.
///
/// The inputs come from the enclosing pre-render and post-render static groups.
/// A local clone supplies prior values to host adapters, and the result is placed
/// into the returned static configuration.
fn commit_normal_distribution_gui_edits(
    params: &MidiBpmDetectorParams,
    setter: &ParamSetter<'_>,
    before: &NormalDistributionConfig,
    after: &NormalDistributionConfig,
) -> NormalDistributionConfig {
    let mut mirrored = before.clone();
    let std_dev = commit_gui_value(before.std_dev, after.std_dev, |std_dev| {
        params.static_params.normal_distribution.mirror_std_dev(&mut mirrored, std_dev, setter);
    });
    let resolution = commit_gui_value(before.resolution, after.resolution, |resolution| {
        params.static_params.normal_distribution.mirror_resolution(&mut mirrored, resolution, setter);
    });
    let cutoff = commit_gui_value(before.cutoff, after.cutoff, |cutoff| {
        params.static_params.normal_distribution.mirror_cutoff(&mut mirrored, cutoff, setter);
    });
    let factor = commit_gui_value(before.factor, after.factor, |factor| {
        params.static_params.normal_distribution.mirror_factor(&mut mirrored, factor, setter);
    });

    NormalDistributionConfig { std_dev, resolution, cutoff, factor }
}

/// Commits changed dynamic-detection fields and returns the complete post-render group.
///
/// `before` and `after` are the dynamic groups captured before and after rendering.
/// A local clone of `before` supplies prior typed values to host adapters. The
/// result becomes the dynamic group in the next GUI draft.
fn commit_dynamic_gui_edits(
    params: &MidiBpmDetectorParams,
    setter: &ParamSetter<'_>,
    before: &DynamicBPMDetectionConfig,
    after: &DynamicBPMDetectionConfig,
) -> DynamicBPMDetectionConfig {
    let mut mirrored = before.clone();
    let beats_lookback = commit_gui_value(before.beats_lookback, after.beats_lookback, |beats_lookback| {
        params.dynamic_params.mirror_beats_lookback(&mut mirrored, beats_lookback, setter);
    });
    let normal_distribution_weight = commit_gui_value(
        before.normal_distribution_weight,
        after.normal_distribution_weight,
        |normal_distribution_weight| {
            params.dynamic_params.mirror_normal_distribution_weight(&mut mirrored, normal_distribution_weight, setter);
        },
    );
    let time_distance_weight =
        commit_gui_value(before.time_distance_weight, after.time_distance_weight, |time_distance_weight| {
            params.dynamic_params.mirror_time_distance_weight(&mut mirrored, time_distance_weight, setter);
        });
    let velocity_current_note_weight = commit_gui_value(
        before.velocity_current_note_weight,
        after.velocity_current_note_weight,
        |velocity_current_note_weight| {
            params.dynamic_params.mirror_velocity_current_note_weight(
                &mut mirrored,
                velocity_current_note_weight,
                setter,
            );
        },
    );
    let velocity_note_from_weight = commit_gui_value(
        before.velocity_note_from_weight,
        after.velocity_note_from_weight,
        |velocity_note_from_weight| {
            params.dynamic_params.mirror_velocity_note_from_weight(&mut mirrored, velocity_note_from_weight, setter);
        },
    );
    let in_beat_range_weight =
        commit_gui_value(before.in_beat_range_weight, after.in_beat_range_weight, |in_beat_range_weight| {
            params.dynamic_params.mirror_in_beat_range_weight(&mut mirrored, in_beat_range_weight, setter);
        });
    let multiplier_weight = commit_gui_value(before.multiplier_weight, after.multiplier_weight, |multiplier_weight| {
        params.dynamic_params.mirror_multiplier_weight(&mut mirrored, multiplier_weight, setter);
    });
    let subdivision_weight =
        commit_gui_value(before.subdivision_weight, after.subdivision_weight, |subdivision_weight| {
            params.dynamic_params.mirror_subdivision_weight(&mut mirrored, subdivision_weight, setter);
        });
    let octave_distance_weight =
        commit_gui_value(before.octave_distance_weight, after.octave_distance_weight, |octave_distance_weight| {
            params.dynamic_params.mirror_octave_distance_weight(&mut mirrored, octave_distance_weight, setter);
        });
    let pitch_distance_weight =
        commit_gui_value(before.pitch_distance_weight, after.pitch_distance_weight, |pitch_distance_weight| {
            params.dynamic_params.mirror_pitch_distance_weight(&mut mirrored, pitch_distance_weight, setter);
        });
    let high_tempo_bias_weight =
        commit_gui_value(before.high_tempo_bias_weight, after.high_tempo_bias_weight, |high_tempo_bias_weight| {
            params.dynamic_params.mirror_high_tempo_bias_weight(&mut mirrored, high_tempo_bias_weight, setter);
        });

    DynamicBPMDetectionConfig {
        beats_lookback,
        normal_distribution_weight,
        time_distance_weight,
        velocity_current_note_weight,
        velocity_note_from_weight,
        in_beat_range_weight,
        multiplier_weight,
        subdivision_weight,
        octave_distance_weight,
        pitch_distance_weight,
        high_tempo_bias_weight,
    }
}

/// Invokes one typed host setter only when rendering changed its field.
///
/// The post-render value is returned so callers can construct an exhaustive
/// configuration group whose fields become the next GUI draft.
fn commit_gui_value<Value: Copy + PartialEq>(before: Value, after: Value, commit: impl FnOnce(Value)) -> Value {
    if after != before {
        commit(after);
    }
    after
}

#[cfg(test)]
#[path = "../tests/unit/gui.rs"]
mod tests;
