use std::{
    sync::{Arc, atomic::AtomicUsize},
    time::Duration,
};

use bpm_detection_config::{GUIConfig, Settings};
use bpm_detection_core::parameters::{
    DynamicBPMDetectionConfig, DynamicBPMDetectionParameterFieldVisitor, NormalDistributionConfig,
    NormalDistributionParameterFieldVisitor, StaticBPMDetectionConfig,
};
use nih_plug::prelude::{Param, ParamFlags, Params, RemoteControlsPage};
use parameter::{Asf64, ParameterField};
use parameter_on_off::OnOff;

use super::*;
use crate::DeferredConfigUpdate;

fn assert_gui_config_eq(actual: &GUIConfig, expected: &GUIConfig) {
    assert_eq!(actual.interpolation_duration, expected.interpolation_duration);
    assert!((actual.interpolation_curve - expected.interpolation_curve).abs() < f32::EPSILON);
}

struct RemoteControlNames(Vec<String>);

impl RemoteControlsPage for RemoteControlNames {
    fn add_param(&mut self, param: &impl Param) {
        self.0.push(param.name().to_owned());
    }

    fn add_spacer(&mut self) {}
}

#[test]
fn plugin_on_off_params_initialize_enabled_state_from_matching_config_field() {
    let mut config = PluginConfig {
        bpm_detection: bpm_detection_config::Settings {
            dynamic_bpm_detection_config: DynamicBPMDetectionConfig {
                velocity_current_note_weight: OnOff::On(1.0),
                velocity_note_from_weight: OnOff::Off(2.0),
                ..DynamicBPMDetectionConfig::default()
            },
            ..bpm_detection_config::Settings::default()
        },
        ..PluginConfig::default()
    };
    let current_sample = Arc::new(AtomicUsize::new(0));
    let changed_at = DeferredConfigUpdate::idle();
    let daw_port = ArcAtomicOptionNonZeroU16::none();

    let params =
        MidiBpmDetectorParams::new(&mut config, &changed_at, &changed_at, &changed_at, &current_sample, &daw_port);

    assert!(params.dynamic_params.velocity_current_note_weight.is_enabled());
    assert!(!params.dynamic_params.velocity_note_from_weight.is_enabled());
}

#[test]
fn dynamic_remote_controls_expose_every_dynamic_parameter() {
    let mut config = PluginConfig::default();
    let current_sample = Arc::new(AtomicUsize::new(0));
    let changed_at = DeferredConfigUpdate::idle();
    let daw_port = ArcAtomicOptionNonZeroU16::none();
    let params =
        MidiBpmDetectorParams::new(&mut config, &changed_at, &changed_at, &changed_at, &current_sample, &daw_port);
    let mut remote_controls = RemoteControlNames(Vec::new());

    params.dynamic_params.add_remote_controls(&mut remote_controls);

    assert_eq!(
        remote_controls.0,
        [
            "Beats Lookback",
            "Normal distribution",
            "Time distance",
            "Note velocity",
            "From note velocity",
            "In beat range",
            "Multiplier",
            "Subdivision",
            "Octave distance",
            "Pitch distance",
            "High tempo bias",
        ]
    );
}

#[test]
fn gui_params_use_parameter_nih_plug_generated_surface() {
    fn assert_generated_params<T: parameter_nih_plug::GeneratedNihPlugParams>() {}

    assert_generated_params::<PluginGUIParams>();
}

#[test]
fn gui_generated_field_names_match_host_parameter_ids_in_order() {
    let mut config = PluginConfig::default();
    let current_sample = Arc::new(AtomicUsize::new(0));
    let changed_at = DeferredConfigUpdate::idle();
    let daw_port = ArcAtomicOptionNonZeroU16::none();
    let params =
        MidiBpmDetectorParams::new(&mut config, &changed_at, &changed_at, &changed_at, &current_sample, &daw_port);
    let ids_and_groups =
        params.gui_params.param_map().into_iter().map(|(id, _, group)| (id, group)).collect::<Vec<_>>();

    assert_eq!(
        ids_and_groups,
        [(String::from("interpolation_duration"), String::new()), (String::from("interpolation_curve"), String::new()),]
    );
}

#[test]
fn static_plugin_parameter_ids_match_config_field_names() {
    let mut config = PluginConfig::default();
    let current_sample = Arc::new(AtomicUsize::new(0));
    let changed_at = DeferredConfigUpdate::idle();
    let daw_port = ArcAtomicOptionNonZeroU16::none();
    let params =
        MidiBpmDetectorParams::new(&mut config, &changed_at, &changed_at, &changed_at, &current_sample, &daw_port);
    let param_ids = params.param_map().into_iter().map(|(id, _, _)| id).collect::<Vec<_>>();

    assert!(param_ids.contains(&String::from("bpm_center")));
    assert!(param_ids.contains(&String::from("bpm_range")));
    assert!(!param_ids.contains(&String::from("lower_bound")));
    assert!(!param_ids.contains(&String::from("upper_bound")));
}

#[test]
fn static_params_use_parameter_nih_plug_generated_surface() {
    fn assert_generated_params<T: parameter_nih_plug::GeneratedNihPlugParams>() {}

    assert_generated_params::<PluginStaticParams>();
}

#[test]
fn dynamic_params_use_parameter_nih_plug_generated_surface() {
    fn assert_generated_params<T: parameter_nih_plug::GeneratedNihPlugParams>() {}

    assert_generated_params::<PluginDynamicParams>();
}

#[test]
fn static_generated_field_names_and_groups_match_host_parameters_in_order() {
    let mut config = PluginConfig::default();
    let current_sample = Arc::new(AtomicUsize::new(0));
    let changed_at = DeferredConfigUpdate::idle();
    let daw_port = ArcAtomicOptionNonZeroU16::none();
    let params =
        MidiBpmDetectorParams::new(&mut config, &changed_at, &changed_at, &changed_at, &current_sample, &daw_port);
    let ids_and_groups =
        params.static_params.param_map().into_iter().map(|(id, _, group)| (id, group)).collect::<Vec<_>>();

    assert_eq!(
        ids_and_groups,
        [
            (String::from("bpm_center"), String::new()),
            (String::from("bpm_range"), String::new()),
            (String::from("sample_rate"), String::new()),
            (String::from("std_dev"), String::from("normal_distribution")),
            (String::from("resolution"), String::from("normal_distribution")),
            (String::from("cutoff"), String::from("normal_distribution")),
            (String::from("factor"), String::from("normal_distribution")),
        ]
    );
}

#[test]
fn dynamic_on_off_persistent_keys_match_parameter_ids() {
    let mut config = PluginConfig::default();
    let current_sample = Arc::new(AtomicUsize::new(0));
    let changed_at = DeferredConfigUpdate::idle();
    let daw_port = ArcAtomicOptionNonZeroU16::none();
    let params =
        MidiBpmDetectorParams::new(&mut config, &changed_at, &changed_at, &changed_at, &current_sample, &daw_port);
    let persistent_keys = params.serialize_fields().into_keys().collect::<Vec<_>>();

    for key in [
        "normal_distribution_weight_onoff",
        "time_distance_weight_onoff",
        "velocity_current_note_weight_onoff",
        "velocity_note_from_weight_onoff",
        "in_beat_range_weight_onoff",
        "multiplier_weight_onoff",
        "subdivision_weight_onoff",
        "octave_distance_weight_onoff",
        "pitch_distance_weight_onoff",
        "high_tempo_bias_weight_onoff",
    ] {
        assert!(persistent_keys.contains(&String::from(key)));
    }
}

#[test]
fn dynamic_generated_field_names_match_host_parameter_ids_in_order() {
    let mut config = PluginConfig::default();
    let current_sample = Arc::new(AtomicUsize::new(0));
    let changed_at = DeferredConfigUpdate::idle();
    let daw_port = ArcAtomicOptionNonZeroU16::none();
    let params =
        MidiBpmDetectorParams::new(&mut config, &changed_at, &changed_at, &changed_at, &current_sample, &daw_port);
    let param_ids = params
        .dynamic_params
        .param_map()
        .into_iter()
        .map(|(id, _, group)| {
            assert_eq!(group, "");
            id
        })
        .collect::<Vec<_>>();
    let mut field_names = DynamicFieldNames(Vec::new());

    DynamicBPMDetectionConfig::PARAMETERS.visit_fields(&mut field_names);

    assert_eq!(param_ids, field_names.0);
}

#[test]
fn normal_distribution_params_use_parameter_nih_plug_generated_surface() {
    fn assert_generated_params<T: parameter_nih_plug::GeneratedNihPlugParams>() {}

    assert_generated_params::<NormalDistributionParams>();
}

#[test]
fn normal_distribution_generated_field_names_match_host_parameter_ids_in_order() {
    let mut config = PluginConfig::default();
    let current_sample = Arc::new(AtomicUsize::new(0));
    let changed_at = DeferredConfigUpdate::idle();
    let daw_port = ArcAtomicOptionNonZeroU16::none();
    let params =
        MidiBpmDetectorParams::new(&mut config, &changed_at, &changed_at, &changed_at, &current_sample, &daw_port);
    let normal_param_ids = params
        .static_params
        .normal_distribution
        .param_map()
        .into_iter()
        .map(|(id, _, group)| {
            assert_eq!(group, "");
            id
        })
        .collect::<Vec<_>>();
    let mut field_names = NormalDistributionFieldNames(Vec::new());

    NormalDistributionConfig::PARAMETERS.visit_fields(&mut field_names);

    assert_eq!(normal_param_ids, field_names.0);
}

#[test]
fn normal_distribution_params_keep_nested_static_group_name() {
    let mut config = PluginConfig::default();
    let current_sample = Arc::new(AtomicUsize::new(0));
    let changed_at = DeferredConfigUpdate::idle();
    let daw_port = ArcAtomicOptionNonZeroU16::none();
    let params =
        MidiBpmDetectorParams::new(&mut config, &changed_at, &changed_at, &changed_at, &current_sample, &daw_port);
    let normal_groups = params
        .static_params
        .param_map()
        .into_iter()
        .filter_map(|(id, _, group)| {
            ["std_dev", "resolution", "cutoff", "factor"].contains(&id.as_str()).then_some(group)
        })
        .collect::<Vec<_>>();

    assert_eq!(normal_groups, ["normal_distribution"; 4]);
}

#[test]
fn daw_port_is_visible_non_automatable_rendezvous_parameter() {
    let mut config = PluginConfig::default();
    let current_sample = Arc::new(AtomicUsize::new(0));
    let changed_at = DeferredConfigUpdate::idle();
    let daw_port = ArcAtomicOptionNonZeroU16::none();
    let params =
        MidiBpmDetectorParams::new(&mut config, &changed_at, &changed_at, &changed_at, &current_sample, &daw_port);

    let flags = params.daw_port.flags();

    assert!(flags.contains(ParamFlags::NON_AUTOMATABLE));
    assert!(!flags.contains(ParamFlags::HIDDEN));
    assert!(!flags.contains(ParamFlags::HIDE_IN_GENERIC_UI));
}

struct DynamicFieldNames(Vec<&'static str>);

impl DynamicBPMDetectionParameterFieldVisitor<DynamicBPMDetectionConfig> for DynamicFieldNames {
    fn field<ValueType: Asf64>(&mut self, field: ParameterField<DynamicBPMDetectionConfig, ValueType>) {
        self.0.push(field.field_name);
    }
}

struct NormalDistributionFieldNames(Vec<String>);

impl NormalDistributionParameterFieldVisitor<NormalDistributionConfig> for NormalDistributionFieldNames {
    fn field<ValueType: Asf64>(&mut self, field: ParameterField<NormalDistributionConfig, ValueType>) {
        self.0.push(String::from(field.field_name));
    }
}

#[test]
fn dynamic_params_read_initialized_host_values_as_dynamic_config() {
    let source_dynamic_config = DynamicBPMDetectionConfig {
        beats_lookback: 13,
        normal_distribution_weight: OnOff::On(0.9),
        time_distance_weight: OnOff::On(1.3),
        velocity_current_note_weight: OnOff::On(1.1),
        velocity_note_from_weight: OnOff::Off(1.2),
        in_beat_range_weight: OnOff::Off(1.8),
        multiplier_weight: OnOff::Off(1.6),
        subdivision_weight: OnOff::On(1.7),
        octave_distance_weight: OnOff::Off(1.4),
        pitch_distance_weight: OnOff::On(1.5),
        high_tempo_bias_weight: OnOff::Off(2.1),
    };
    let mut config = PluginConfig {
        bpm_detection: Settings { dynamic_bpm_detection_config: source_dynamic_config.clone(), ..Settings::default() },
        ..PluginConfig::default()
    };
    let current_sample = Arc::new(AtomicUsize::new(0));
    let changed_at = DeferredConfigUpdate::idle();
    let daw_port = ArcAtomicOptionNonZeroU16::none();
    let params =
        MidiBpmDetectorParams::new(&mut config, &changed_at, &changed_at, &changed_at, &current_sample, &daw_port);

    assert_eq!(params.dynamic_params.read_dynamic_config(), source_dynamic_config);
}

#[test]
fn static_params_read_initialized_host_values_as_static_config() {
    let source_static_config = StaticBPMDetectionConfig {
        bpm_center: 111.5,
        bpm_range: 48,
        sample_rate: 720,
        normal_distribution: NormalDistributionConfig { std_dev: 18.25, resolution: 0.5, cutoff: 128.0, factor: 32.0 },
    };
    let mut config = PluginConfig {
        bpm_detection: Settings { static_bpm_detection_config: source_static_config.clone(), ..Settings::default() },
        ..PluginConfig::default()
    };
    let current_sample = Arc::new(AtomicUsize::new(0));
    let changed_at = DeferredConfigUpdate::idle();
    let daw_port = ArcAtomicOptionNonZeroU16::none();
    let params =
        MidiBpmDetectorParams::new(&mut config, &changed_at, &changed_at, &changed_at, &current_sample, &daw_port);

    assert_eq!(params.static_params.read_static_config(), source_static_config);
}

#[test]
fn gui_params_read_initialized_host_values_as_gui_config() {
    let source_gui_config =
        GUIConfig { interpolation_duration: Duration::from_secs_f32(0.82), interpolation_curve: 1.25 };
    let mut config = PluginConfig {
        bpm_detection: Settings { gui_config: source_gui_config.clone(), ..Settings::default() },
        ..PluginConfig::default()
    };
    let current_sample = Arc::new(AtomicUsize::new(0));
    let changed_at = DeferredConfigUpdate::idle();
    let daw_port = ArcAtomicOptionNonZeroU16::none();
    let params =
        MidiBpmDetectorParams::new(&mut config, &changed_at, &changed_at, &changed_at, &current_sample, &daw_port);

    assert_gui_config_eq(&params.gui_params.read_gui_config(), &source_gui_config);
}
