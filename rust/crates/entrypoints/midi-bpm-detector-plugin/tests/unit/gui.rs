use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex, atomic::AtomicUsize},
};

use gui::{EditableSettings, GuiChanges};
use nice_plug::{
    context::PluginApi,
    prelude::{GuiContext, ParamPtr, ParamSetter, Params, PluginState},
};
use parameter_on_off::OnOff;
use sync::{ArcAtomicBool, ArcAtomicOptionNonZeroU16};

use super::{commit_gui_edits, merge_host_changes};
use crate::{DeferredConfigUpdate, plugin_config::PluginConfig, plugin_parameters::MidiBpmDetectorParams};

fn editable_settings() -> EditableSettings {
    EditableSettings { bpm: bpm_detection_config::Settings::default(), send_tempo: Some(false) }
}

#[test]
fn host_and_gui_changes_to_different_dynamic_fields_survive() {
    let previous = editable_settings();
    let mut draft = previous.clone();
    draft.bpm.dynamic_bpm_detection_config.beats_lookback = 16;

    let mut current_host = previous.clone();
    current_host.bpm.dynamic_bpm_detection_config.time_distance_weight = OnOff::On(1.5);

    let draft = merge_host_changes(&draft, &previous, &current_host);

    assert_eq!(draft.bpm.dynamic_bpm_detection_config.beats_lookback, 16);
    assert_eq!(draft.bpm.dynamic_bpm_detection_config.time_distance_weight, OnOff::On(1.5));
}

#[test]
fn host_and_gui_changes_to_different_nested_static_fields_survive() {
    let previous = editable_settings();
    let mut draft = previous.clone();
    draft.bpm.static_bpm_detection_config.normal_distribution.std_dev = 2.5;

    let mut current_host = previous.clone();
    current_host.bpm.static_bpm_detection_config.normal_distribution.cutoff = 0.75;

    let draft = merge_host_changes(&draft, &previous, &current_host);

    assert!((draft.bpm.static_bpm_detection_config.normal_distribution.std_dev - 2.5).abs() < f64::EPSILON);
    assert!((draft.bpm.static_bpm_detection_config.normal_distribution.cutoff - 0.75).abs() < f32::EPSILON);
}

#[test]
fn newer_observed_host_value_replaces_same_gui_draft_field() {
    let previous = editable_settings();
    let mut draft = previous.clone();
    draft.bpm.static_bpm_detection_config.bpm_center = 110.0;

    let mut current_host = previous.clone();
    current_host.bpm.static_bpm_detection_config.bpm_center = 125.0;

    let draft = merge_host_changes(&draft, &previous, &current_host);

    assert!((draft.bpm.static_bpm_detection_config.bpm_center - 125.0).abs() < f32::EPSILON);
}

#[test]
fn enabled_only_gui_edit_targets_the_dynamic_boolean_host_parameter() {
    let config = PluginConfig::default();
    let current_sample = Arc::new(AtomicUsize::new(0));
    let changed_at = DeferredConfigUpdate::idle();
    let params = MidiBpmDetectorParams::new(
        &config,
        &changed_at,
        &changed_at,
        &changed_at,
        &current_sample,
        &ArcAtomicOptionNonZeroU16::none(),
        ArcAtomicBool::new(config.send_tempo),
    );
    let before = params.read_editable_settings();
    let mut after = before.clone();
    let weight = before.bpm.dynamic_bpm_detection_config.normal_distribution_weight;
    after.bpm.dynamic_bpm_detection_config.normal_distribution_weight =
        OnOff::new(!weight.is_enabled(), weight.value());
    let enabled_param = params
        .dynamic_params
        .param_map()
        .into_iter()
        .find_map(|(id, param, _)| (id == "normal_distribution_weight_enabled").then_some(param))
        .expect("normal distribution enabled parameter should exist");
    let context = RecordingGuiContext::default();
    let setter = ParamSetter::new(&context);

    let committed = commit_gui_edits(
        &params,
        &setter,
        &before,
        &after,
        GuiChanges { dynamic_detection: true, ..GuiChanges::default() },
    );

    assert_eq!(
        committed.bpm.dynamic_bpm_detection_config.normal_distribution_weight,
        after.bpm.dynamic_bpm_detection_config.normal_distribution_weight
    );
    assert_eq!(params.dynamic_params.normal_distribution_weight.read(), weight);
    assert_eq!(context.actions(), setter_actions(enabled_param));
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SetterAction {
    Begin(ParamPtr),
    Set(ParamPtr),
    End(ParamPtr),
}

fn setter_actions(param: ParamPtr) -> Vec<SetterAction> {
    vec![SetterAction::Begin(param), SetterAction::Set(param), SetterAction::End(param)]
}

#[derive(Default)]
struct RecordingGuiContext {
    actions: Mutex<Vec<SetterAction>>,
}

impl RecordingGuiContext {
    fn actions(&self) -> Vec<SetterAction> {
        self.actions.lock().unwrap().clone()
    }
}

impl GuiContext for RecordingGuiContext {
    fn plugin_api(&self) -> PluginApi {
        PluginApi::Standalone
    }

    fn request_resize(&self) -> bool {
        false
    }

    unsafe fn raw_begin_set_parameter(&self, param: ParamPtr) {
        self.actions.lock().unwrap().push(SetterAction::Begin(param));
    }

    unsafe fn raw_set_parameter_normalized(&self, param: ParamPtr, _normalized: f32) {
        self.actions.lock().unwrap().push(SetterAction::Set(param));
    }

    unsafe fn raw_end_set_parameter(&self, param: ParamPtr) {
        self.actions.lock().unwrap().push(SetterAction::End(param));
    }

    fn get_state(&self) -> PluginState {
        PluginState { version: String::new(), params: BTreeMap::new(), fields: BTreeMap::new() }
    }

    fn set_state(&self, _state: PluginState) {}
}
